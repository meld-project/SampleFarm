pub mod analyzer;
pub mod extractor;
pub mod hasher;
pub mod validator;

pub use analyzer::FileAnalyzer;
pub use extractor::ZipExtractor;
pub use hasher::FileHasher;
pub use validator::FileValidator;

use crate::error::AppResult;
use serde::{Deserialize, Serialize};

/// 文件处理结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileProcessingResult {
    /// 文件基本信息
    pub file_info: FileInfo,
    /// 文件哈希信息
    pub hashes: FileHashes,
    /// ZIP包中的子文件（如果是ZIP文件）
    pub sub_files: Option<Vec<SubFileInfo>>,
}

/// 文件基本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// 文件名
    pub filename: String,
    /// 文件大小（字节）
    pub size: u64,
    /// MIME类型
    pub mime_type: String,
    /// 文件扩展名
    pub extension: Option<String>,
    /// 是否为容器文件（ZIP等）
    pub is_container: bool,
    /// 检测到的文件类型描述
    pub file_type_description: String,
}

/// 文件哈希信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHashes {
    /// MD5哈希值
    pub md5: String,
    /// SHA1哈希值
    pub sha1: String,
    /// SHA256哈希值
    pub sha256: String,
}

/// 子文件信息（ZIP包中的文件）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubFileInfo {
    /// 在ZIP中的文件路径
    pub path_in_zip: String,
    /// 文件名
    pub filename: String,
    /// 文件大小（压缩前）
    pub uncompressed_size: u64,
    /// 文件大小（压缩后）
    pub compressed_size: u64,
    /// MIME类型
    pub mime_type: String,
    /// 文件扩展名
    pub extension: Option<String>,
    /// 文件哈希
    pub hashes: FileHashes,
    /// 是否为目录
    pub is_directory: bool,
    /// 修改时间
    pub modified_time: Option<chrono::DateTime<chrono::Utc>>,
    /// 文件数据
    pub data: Vec<u8>,
}

/// 文件处理配置
#[derive(Debug, Clone)]
pub struct FileProcessingConfig {
    /// 最大文件大小（字节）
    pub max_file_size: u64,
    /// 临时目录
    pub temp_dir: String,
    /// 支持的文件类型
    pub allowed_mime_types: Vec<String>,
    /// ZIP解压密码尝试列表
    pub default_passwords: Vec<String>,
    /// 最大ZIP条目数量
    pub max_zip_entries: usize,
    /// 最大解压后总大小
    pub max_extracted_size: u64,
}

impl Default for FileProcessingConfig {
    fn default() -> Self {
        Self {
            max_file_size: 1024 * 1024 * 1024, // 1GB
            temp_dir: "/tmp/samplefarm".to_string(),
            allowed_mime_types: vec![
                "application/zip".to_string(),
                "application/x-zip-compressed".to_string(),
                "application/octet-stream".to_string(),
                "application/x-executable".to_string(),
                "application/x-msdownload".to_string(),
                "application/x-msdos-program".to_string(),
            ],
            default_passwords: vec![
                "infected".to_string(),
                "malware".to_string(),
                "virus".to_string(),
                "password".to_string(),
                "123456".to_string(),
            ],
            max_zip_entries: 1000,
            max_extracted_size: 10 * 1024 * 1024 * 1024, // 10GB
        }
    }
}

/// 文件处理器主接口
pub struct FileProcessor {
    config: FileProcessingConfig,
    analyzer: FileAnalyzer,
    extractor: ZipExtractor,
    hasher: FileHasher,
    validator: FileValidator,
}

impl std::fmt::Debug for FileProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileProcessor")
            .field("config", &self.config)
            .finish()
    }
}

impl Clone for FileProcessor {
    fn clone(&self) -> Self {
        // 重新创建一个新的FileProcessor实例
        Self::new(self.config.clone()).unwrap_or_else(|_| {
            // 如果创建失败，使用默认配置
            Self::new(FileProcessingConfig::default()).expect("默认配置应该总是成功")
        })
    }
}

impl FileProcessor {
    /// 创建新的文件处理器
    pub fn new(config: FileProcessingConfig) -> AppResult<Self> {
        Ok(Self {
            analyzer: FileAnalyzer::new(),
            extractor: ZipExtractor::new(&config)?,
            hasher: FileHasher::new(),
            validator: FileValidator::new(&config),
            config,
        })
    }

    /// 处理单个文件
    pub async fn process_file(
        &self,
        file_data: &[u8],
        filename: &str,
    ) -> AppResult<FileProcessingResult> {
        self.process_file_with_passwords(file_data, filename, &[])
            .await
    }

    /// 处理单个文件（支持密码列表）
    pub async fn process_file_with_passwords(
        &self,
        file_data: &[u8],
        filename: &str,
        passwords: &[String],
    ) -> AppResult<FileProcessingResult> {
        // 1. 验证文件
        self.validator.validate_file(file_data, filename)?;

        // 2. 分析文件基本信息
        let file_info = self.analyzer.analyze_file(file_data, filename).await?;

        // 3. 计算文件哈希
        let hashes = self.hasher.calculate_hashes(file_data).await?;

        // 4. 如果是ZIP文件，解压并分析子文件
        let sub_files = if file_info.is_container {
            Some(self.extract_and_analyze_zip(file_data, passwords).await?)
        } else {
            None
        };

        Ok(FileProcessingResult {
            file_info,
            hashes,
            sub_files,
        })
    }

    /// 解压ZIP文件并分析子文件
    async fn extract_and_analyze_zip(
        &self,
        zip_data: &[u8],
        passwords: &[String],
    ) -> AppResult<Vec<SubFileInfo>> {
        let extracted_files = self
            .extractor
            .extract_zip_with_passwords(zip_data, passwords)
            .await?;
        let mut sub_files = Vec::new();

        for extracted_file in extracted_files {
            if !extracted_file.is_directory {
                // 计算子文件哈希
                let hashes = self.hasher.calculate_hashes(&extracted_file.data).await?;

                // 分析子文件MIME类型
                let mime_type = self.analyzer.detect_mime_type(&extracted_file.data);

                sub_files.push(SubFileInfo {
                    path_in_zip: extracted_file.path_in_zip,
                    filename: extracted_file.filename,
                    uncompressed_size: extracted_file.uncompressed_size,
                    compressed_size: extracted_file.compressed_size,
                    mime_type,
                    extension: extracted_file.extension,
                    hashes,
                    is_directory: extracted_file.is_directory,
                    modified_time: extracted_file.modified_time,
                    data: extracted_file.data,
                });
            }
        }

        Ok(sub_files)
    }

    /// 获取配置
    pub fn config(&self) -> &FileProcessingConfig {
        &self.config
    }
}
