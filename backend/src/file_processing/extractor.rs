use super::FileProcessingConfig;
use crate::error::{AppError, AppResult};
use std::io::{Cursor, Read};
use std::path::Path;

use tokio::task;
use zip::ZipArchive;

/// 解压后的文件信息
#[derive(Debug, Clone)]
pub struct ExtractedFile {
    /// 在ZIP中的完整路径
    pub path_in_zip: String,
    /// 文件名（不含路径）
    pub filename: String,
    /// 文件数据
    pub data: Vec<u8>,
    /// 压缩前大小
    pub uncompressed_size: u64,
    /// 压缩后大小
    pub compressed_size: u64,
    /// 文件扩展名
    pub extension: Option<String>,
    /// 是否为目录
    pub is_directory: bool,
    /// 修改时间
    pub modified_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// ZIP解压器
pub struct ZipExtractor {
    config: FileProcessingConfig,
}

impl ZipExtractor {
    /// 创建新的ZIP解压器
    pub fn new(config: &FileProcessingConfig) -> AppResult<Self> {
        // 创建临时目录（如果不存在）
        std::fs::create_dir_all(&config.temp_dir)
            .map_err(|e| AppError::FileProcessing(format!("创建临时目录失败: {}", e)))?;

        Ok(Self {
            config: config.clone(),
        })
    }

    /// 解压ZIP文件
    pub async fn extract_zip(&self, zip_data: &[u8]) -> AppResult<Vec<ExtractedFile>> {
        let zip_data = zip_data.to_vec();
        let config = self.config.clone();

        // 在后台任务中进行解压，避免阻塞异步运行时
        let extracted_files = task::spawn_blocking(move || -> AppResult<Vec<ExtractedFile>> {
            Self::extract_zip_blocking(&zip_data, &config)
        })
        .await
        .map_err(|e| AppError::FileProcessing(format!("ZIP解压任务失败: {}", e)))??;

        Ok(extracted_files)
    }

    /// 阻塞式ZIP解压（在后台线程中运行）
    fn extract_zip_blocking(
        zip_data: &[u8],
        config: &FileProcessingConfig,
    ) -> AppResult<Vec<ExtractedFile>> {
        let cursor = Cursor::new(zip_data);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| AppError::FileProcessing(format!("打开ZIP文件失败: {}", e)))?;

        // 检查ZIP文件中的条目数量
        if archive.len() > config.max_zip_entries {
            return Err(AppError::FileProcessing(format!(
                "ZIP文件包含过多条目: {} > {}",
                archive.len(),
                config.max_zip_entries
            )));
        }

        let mut extracted_files = Vec::new();
        let mut total_extracted_size = 0u64;

        // 遍历ZIP中的所有条目
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| AppError::FileProcessing(format!("读取ZIP条目 {} 失败: {}", i, e)))?;

            let path_in_zip = file.name().to_string();
            let is_directory = file.is_dir();

            // 检查路径安全性（防止路径遍历攻击）
            if Self::is_unsafe_path(&path_in_zip) {
                tracing::warn!("跳过不安全的ZIP路径: {}", path_in_zip);
                continue;
            }

            let filename = Path::new(&path_in_zip)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("")
                .to_string();

            let extension = Path::new(&path_in_zip)
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_lowercase());

            let uncompressed_size = file.size();
            let compressed_size = file.compressed_size();

            // 检查解压后的总大小
            total_extracted_size += uncompressed_size;
            if total_extracted_size > config.max_extracted_size {
                return Err(AppError::FileProcessing(format!(
                    "解压后文件总大小超出限制: {} > {}",
                    total_extracted_size, config.max_extracted_size
                )));
            }

            // 转换修改时间
            let modified_time = file
                .last_modified()
                .and_then(|dt| {
                    use time::OffsetDateTime;
                    OffsetDateTime::try_from(dt).ok()
                })
                .and_then(|time| chrono::DateTime::from_timestamp(time.unix_timestamp(), 0));

            let mut data = Vec::new();
            if !is_directory {
                // 读取文件内容
                file.read_to_end(&mut data)
                    .map_err(|e| AppError::FileProcessing(format!("读取ZIP文件内容失败: {}", e)))?;

                // 验证读取的数据大小
                if data.len() as u64 != uncompressed_size {
                    return Err(AppError::FileProcessing(format!(
                        "ZIP文件大小不匹配: 期望 {}, 实际 {}",
                        uncompressed_size,
                        data.len()
                    )));
                }
            }

            extracted_files.push(ExtractedFile {
                path_in_zip,
                filename,
                data,
                uncompressed_size,
                compressed_size,
                extension,
                is_directory,
                modified_time,
            });
        }

        tracing::info!("成功解压ZIP文件，包含 {} 个条目", extracted_files.len());
        Ok(extracted_files)
    }

    /// 尝试使用密码解压ZIP文件
    pub async fn extract_zip_with_password(
        &self,
        zip_data: &[u8],
        password: &str,
    ) -> AppResult<Vec<ExtractedFile>> {
        let zip_data = zip_data.to_vec();
        let password = password.to_string();
        let config = self.config.clone();

        let extracted_files = task::spawn_blocking(move || -> AppResult<Vec<ExtractedFile>> {
            Self::extract_zip_with_password_blocking(&zip_data, &password, &config)
        })
        .await
        .map_err(|e| AppError::FileProcessing(format!("ZIP密码解压任务失败: {}", e)))??;

        Ok(extracted_files)
    }

    /// 阻塞式密码ZIP解压
    fn extract_zip_with_password_blocking(
        zip_data: &[u8],
        password: &str,
        config: &FileProcessingConfig,
    ) -> AppResult<Vec<ExtractedFile>> {
        let cursor = Cursor::new(zip_data);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| AppError::FileProcessing(format!("打开ZIP文件失败: {}", e)))?;

        let mut extracted_files = Vec::new();
        let mut total_extracted_size = 0u64;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index_decrypt(i, password.as_bytes())
                .map_err(|e| AppError::FileProcessing(format!("解密ZIP条目失败: {}", e)))?;

            let path_in_zip = file.name().to_string();
            let is_directory = file.is_dir();

            if Self::is_unsafe_path(&path_in_zip) {
                tracing::warn!("跳过不安全的ZIP路径: {}", path_in_zip);
                continue;
            }

            let filename = Path::new(&path_in_zip)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("")
                .to_string();

            let extension = Path::new(&path_in_zip)
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_lowercase());

            let uncompressed_size = file.size();
            let compressed_size = file.compressed_size();

            total_extracted_size += uncompressed_size;
            if total_extracted_size > config.max_extracted_size {
                return Err(AppError::FileProcessing(format!(
                    "解压后文件总大小超出限制: {} > {}",
                    total_extracted_size, config.max_extracted_size
                )));
            }

            let modified_time = file
                .last_modified()
                .and_then(|dt| {
                    use time::OffsetDateTime;
                    OffsetDateTime::try_from(dt).ok()
                })
                .and_then(|time| chrono::DateTime::from_timestamp(time.unix_timestamp(), 0));

            let mut data = Vec::new();
            if !is_directory {
                file.read_to_end(&mut data).map_err(|e| {
                    AppError::FileProcessing(format!("读取加密ZIP文件内容失败: {}", e))
                })?;
            }

            extracted_files.push(ExtractedFile {
                path_in_zip,
                filename,
                data,
                uncompressed_size,
                compressed_size,
                extension,
                is_directory,
                modified_time,
            });
        }

        tracing::info!("成功解压加密ZIP文件，包含 {} 个条目", extracted_files.len());
        Ok(extracted_files)
    }

    /// 尝试多个密码解压ZIP文件
    #[allow(unused_assignments)]
    pub async fn extract_zip_with_passwords(
        &self,
        zip_data: &[u8],
        passwords: &[String],
    ) -> AppResult<Vec<ExtractedFile>> {
        let mut last_error: Option<AppError> = None;

        // 首先尝试无密码解压
        match self.extract_zip(zip_data).await {
            Ok(files) => return Ok(files),
            Err(e) => {
                tracing::debug!("无密码解压失败: {}", e);
                last_error = Some(e);
            }
        }

        // 尝试配置中的默认密码
        for password in &self.config.default_passwords {
            match self.extract_zip_with_password(zip_data, password).await {
                Ok(files) => {
                    tracing::info!("使用密码 '{}' 成功解压ZIP文件", password);
                    return Ok(files);
                }
                Err(e) => {
                    tracing::debug!("密码 '{}' 解压失败: {}", password, e);
                    last_error = Some(e);
                }
            }
        }

        // 尝试提供的密码列表
        for password in passwords {
            match self.extract_zip_with_password(zip_data, password).await {
                Ok(files) => {
                    tracing::info!("使用提供的密码成功解压ZIP文件");
                    return Ok(files);
                }
                Err(e) => {
                    tracing::debug!("提供的密码解压失败: {}", e);
                    last_error = Some(e);
                }
            }
        }

        // 所有密码都失败了
        Err(last_error
            .unwrap_or_else(|| AppError::FileProcessing("所有密码尝试都失败了".to_string())))
    }

    /// 检查路径是否安全（防止路径遍历攻击）
    fn is_unsafe_path(path: &str) -> bool {
        // 检查路径遍历模式
        if path.contains("..") || path.contains("./") || path.contains(".\\") {
            return true;
        }

        // 检查绝对路径
        if path.starts_with('/') || path.starts_with('\\') {
            return true;
        }

        // 检查Windows驱动器路径
        if path.len() >= 2 && path.chars().nth(1) == Some(':') {
            return true;
        }

        false
    }

    /// 获取ZIP文件信息（不解压）
    pub async fn get_zip_info(&self, zip_data: &[u8]) -> AppResult<ZipInfo> {
        let zip_data = zip_data.to_vec();

        let info = task::spawn_blocking(move || -> AppResult<ZipInfo> {
            let cursor = Cursor::new(zip_data);
            let mut archive = ZipArchive::new(cursor)
                .map_err(|e| AppError::FileProcessing(format!("打开ZIP文件失败: {}", e)))?;

            let mut total_uncompressed_size = 0u64;
            let mut total_compressed_size = 0u64;
            let mut file_count = 0usize;
            let mut dir_count = 0usize;

            for i in 0..archive.len() {
                let file = archive
                    .by_index(i)
                    .map_err(|e| AppError::FileProcessing(format!("读取ZIP条目信息失败: {}", e)))?;

                if file.is_dir() {
                    dir_count += 1;
                } else {
                    file_count += 1;
                    total_uncompressed_size += file.size();
                    total_compressed_size += file.compressed_size();
                }
            }

            Ok(ZipInfo {
                total_entries: archive.len(),
                file_count,
                dir_count,
                total_uncompressed_size,
                total_compressed_size,
                compression_ratio: if total_uncompressed_size > 0 {
                    (total_compressed_size as f64 / total_uncompressed_size as f64) * 100.0
                } else {
                    0.0
                },
            })
        })
        .await
        .map_err(|e| AppError::FileProcessing(format!("ZIP信息获取任务失败: {}", e)))??;

        Ok(info)
    }
}

/// ZIP文件信息
#[derive(Debug, Clone)]
pub struct ZipInfo {
    /// 总条目数
    pub total_entries: usize,
    /// 文件数量
    pub file_count: usize,
    /// 目录数量
    pub dir_count: usize,
    /// 总的未压缩大小
    pub total_uncompressed_size: u64,
    /// 总的压缩大小
    pub total_compressed_size: u64,
    /// 压缩率（百分比）
    pub compression_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_unsafe_path() {
        // 安全路径
        assert!(!ZipExtractor::is_unsafe_path("file.txt"));
        assert!(!ZipExtractor::is_unsafe_path("folder/file.txt"));
        assert!(!ZipExtractor::is_unsafe_path("a/b/c/file.txt"));

        // 不安全路径
        assert!(ZipExtractor::is_unsafe_path("../file.txt"));
        assert!(ZipExtractor::is_unsafe_path("folder/../file.txt"));
        assert!(ZipExtractor::is_unsafe_path("./file.txt"));
        assert!(ZipExtractor::is_unsafe_path("/absolute/path"));
        assert!(ZipExtractor::is_unsafe_path("\\windows\\path"));
        assert!(ZipExtractor::is_unsafe_path("C:\\windows\\file.txt"));
    }
}
