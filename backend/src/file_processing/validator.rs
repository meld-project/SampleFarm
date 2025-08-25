use super::FileProcessingConfig;
use crate::error::{AppError, AppResult};
use std::path::Path;

/// 文件验证器
pub struct FileValidator {
    config: FileProcessingConfig,
}

impl FileValidator {
    /// 创建新的文件验证器
    pub fn new(config: &FileProcessingConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// 验证文件是否符合要求
    pub fn validate_file(&self, file_data: &[u8], filename: &str) -> AppResult<()> {
        // 1. 检查文件大小
        self.validate_file_size(file_data)?;

        // 2. 检查文件名
        self.validate_filename(filename)?;

        // 3. 检查文件内容（基本安全检查）
        self.validate_file_content(file_data)?;

        Ok(())
    }

    /// 验证文件大小
    fn validate_file_size(&self, file_data: &[u8]) -> AppResult<()> {
        let file_size = file_data.len() as u64;

        if file_size == 0 {
            return Err(AppError::Validation("文件不能为空".to_string()));
        }

        if file_size > self.config.max_file_size {
            return Err(AppError::FileTooLarge {
                max_size: self.config.max_file_size,
            });
        }

        Ok(())
    }

    /// 验证文件名
    fn validate_filename(&self, filename: &str) -> AppResult<()> {
        if filename.is_empty() {
            return Err(AppError::Validation("文件名不能为空".to_string()));
        }

        // 检查文件名长度
        if filename.len() > 255 {
            return Err(AppError::Validation(
                "文件名过长，最大支持255个字符".to_string(),
            ));
        }

        // 检查危险字符
        let dangerous_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
        if filename.chars().any(|c| dangerous_chars.contains(&c)) {
            return Err(AppError::Validation("文件名包含非法字符".to_string()));
        }

        // 检查保留名称（Windows）
        let reserved_names = [
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ];

        let name_without_ext = Path::new(filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_uppercase();

        if reserved_names.contains(&name_without_ext.as_str()) {
            return Err(AppError::Validation("文件名为系统保留名称".to_string()));
        }

        Ok(())
    }

    /// 验证文件内容（基本安全检查）
    fn validate_file_content(&self, file_data: &[u8]) -> AppResult<()> {
        // 检查是否为空文件
        if file_data.is_empty() {
            return Err(AppError::Validation("文件内容为空".to_string()));
        }

        // 检查文件头，识别可能的恶意模式
        self.check_suspicious_patterns(file_data)?;

        Ok(())
    }

    /// 检查可疑模式（简单的启发式检测）
    fn check_suspicious_patterns(&self, file_data: &[u8]) -> AppResult<()> {
        // 检查文件头是否符合预期
        if file_data.len() >= 2 {
            // 检查是否为可执行文件（在样本分析系统中这是正常的）
            match &file_data[0..2] {
                [0x4D, 0x5A] => {
                    // PE文件，记录日志但不阻止
                    tracing::debug!("检测到PE可执行文件");
                }
                [0x7F, 0x45] if file_data.len() >= 4 && &file_data[1..4] == b"ELF" => {
                    // ELF文件，记录日志但不阻止
                    tracing::debug!("检测到ELF可执行文件");
                }
                _ => {}
            }
        }

        // 对于样本分析系统，我们通常不会阻止可执行文件
        // 这里主要做一些基本的格式验证

        Ok(())
    }

    /// 验证ZIP文件特定要求
    pub fn validate_zip_file(&self, file_data: &[u8]) -> AppResult<()> {
        // 检查ZIP文件头
        if file_data.len() < 4 {
            return Err(AppError::Validation(
                "文件太小，不是有效的ZIP文件".to_string(),
            ));
        }

        // 检查ZIP文件签名
        let zip_signatures = [
            [0x50, 0x4B, 0x03, 0x04], // 标准ZIP
            [0x50, 0x4B, 0x05, 0x06], // 空ZIP
            [0x50, 0x4B, 0x07, 0x08], // Spanned ZIP
        ];

        let is_zip = zip_signatures
            .iter()
            .any(|sig| file_data.len() >= 4 && &file_data[0..4] == sig);

        if !is_zip {
            return Err(AppError::Validation("文件不是有效的ZIP格式".to_string()));
        }

        Ok(())
    }

    /// 验证MIME类型是否被允许
    pub fn validate_mime_type(&self, mime_type: &str) -> AppResult<()> {
        if self.config.allowed_mime_types.is_empty() {
            // 如果没有配置允许的类型，则允许所有类型
            return Ok(());
        }

        if self
            .config
            .allowed_mime_types
            .contains(&mime_type.to_string())
        {
            Ok(())
        } else {
            Err(AppError::UnsupportedFileType {
                file_type: mime_type.to_string(),
            })
        }
    }

    /// 验证文件扩展名
    pub fn validate_file_extension(
        &self,
        filename: &str,
        allowed_extensions: &[&str],
    ) -> AppResult<()> {
        if allowed_extensions.is_empty() {
            return Ok(());
        }

        let extension = Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());

        match extension {
            Some(ext) if allowed_extensions.contains(&ext.as_str()) => Ok(()),
            Some(ext) => Err(AppError::UnsupportedFileType {
                file_type: format!("扩展名: {}", ext),
            }),
            None => Err(AppError::Validation("文件没有扩展名".to_string())),
        }
    }

    /// 批量验证文件
    pub fn validate_files(&self, files: &[(Vec<u8>, String)]) -> AppResult<()> {
        for (file_data, filename) in files {
            self.validate_file(file_data, filename)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> FileProcessingConfig {
        FileProcessingConfig {
            max_file_size: 1024,
            allowed_mime_types: vec!["application/zip".to_string()],
            ..Default::default()
        }
    }

    #[test]
    fn test_validate_file_size() {
        let config = create_test_config();
        let validator = FileValidator::new(&config);

        // 测试空文件
        assert!(validator.validate_file_size(&[]).is_err());

        // 测试正常大小
        assert!(validator.validate_file_size(&vec![0; 512]).is_ok());

        // 测试过大文件
        assert!(validator.validate_file_size(&vec![0; 2048]).is_err());
    }

    #[test]
    fn test_validate_filename() {
        let config = create_test_config();
        let validator = FileValidator::new(&config);

        // 测试正常文件名
        assert!(validator.validate_filename("test.zip").is_ok());

        // 测试空文件名
        assert!(validator.validate_filename("").is_err());

        // 测试包含非法字符的文件名
        assert!(validator.validate_filename("test/file.zip").is_err());
        assert!(validator.validate_filename("test<file>.zip").is_err());

        // 测试保留名称
        assert!(validator.validate_filename("CON.zip").is_err());
        assert!(validator.validate_filename("NUL.txt").is_err());
    }

    #[test]
    fn test_validate_mime_type() {
        let config = create_test_config();
        let validator = FileValidator::new(&config);

        // 测试允许的MIME类型
        assert!(validator.validate_mime_type("application/zip").is_ok());

        // 测试不允许的MIME类型
        assert!(validator.validate_mime_type("text/plain").is_err());
    }

    #[test]
    fn test_validate_zip_file() {
        let config = create_test_config();
        let validator = FileValidator::new(&config);

        // 测试有效的ZIP文件头
        let zip_header = [0x50, 0x4B, 0x03, 0x04, 0x00, 0x00];
        assert!(validator.validate_zip_file(&zip_header).is_ok());

        // 测试无效的文件头
        let invalid_header = [0x00, 0x00, 0x00, 0x00];
        assert!(validator.validate_zip_file(&invalid_header).is_err());

        // 测试文件太小
        let small_file = [0x50, 0x4B];
        assert!(validator.validate_zip_file(&small_file).is_err());
    }
}
