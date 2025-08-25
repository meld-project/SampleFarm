use super::FileInfo;
use crate::error::AppResult;
use std::path::Path;

/// 文件分析器
pub struct FileAnalyzer;

impl FileAnalyzer {
    /// 创建新的文件分析器
    pub fn new() -> Self {
        Self
    }

    /// 分析文件基本信息
    pub async fn analyze_file(&self, file_data: &[u8], filename: &str) -> AppResult<FileInfo> {
        // 检测MIME类型
        let mime_type = self.detect_mime_type(file_data);

        // 获取文件扩展名
        let extension = self.extract_extension(filename);

        // 判断是否为容器文件
        let is_container = self.is_container_file(&mime_type, &extension);

        // 生成文件类型描述
        let file_type_description = self.generate_file_type_description(&mime_type, &extension);

        Ok(FileInfo {
            filename: filename.to_string(),
            size: file_data.len() as u64,
            mime_type,
            extension,
            is_container,
            file_type_description,
        })
    }

    /// 检测文件MIME类型
    pub fn detect_mime_type(&self, file_data: &[u8]) -> String {
        // 使用infer库检测文件类型
        if let Some(kind) = infer::get(file_data) {
            kind.mime_type().to_string()
        } else {
            // 如果无法检测，尝试基于文件内容进行简单判断
            self.detect_mime_type_by_content(file_data)
        }
    }

    /// 基于文件内容检测MIME类型（fallback方法）
    fn detect_mime_type_by_content(&self, file_data: &[u8]) -> String {
        if file_data.is_empty() {
            return "application/octet-stream".to_string();
        }

        // 检查常见的文件头
        if file_data.len() >= 2 {
            match &file_data[0..2] {
                [0x4D, 0x5A] => return "application/x-msdownload".to_string(), // PE executable
                [0x7F, 0x45] if file_data.len() >= 4 && &file_data[1..4] == b"ELF" => {
                    return "application/x-executable".to_string(); // ELF executable
                }
                _ => {}
            }
        }

        if file_data.len() >= 4 {
            match &file_data[0..4] {
                [0x50, 0x4B, 0x03, 0x04] | [0x50, 0x4B, 0x05, 0x06] | [0x50, 0x4B, 0x07, 0x08] => {
                    return "application/zip".to_string(); // ZIP file
                }
                [0xCA, 0xFE, 0xBA, 0xBE] => return "application/java-vm".to_string(), // Java class
                _ => {}
            }
        }

        // 检查是否为文本文件
        if self.is_text_content(file_data) {
            "text/plain".to_string()
        } else {
            "application/octet-stream".to_string()
        }
    }

    /// 检查是否为文本内容
    fn is_text_content(&self, data: &[u8]) -> bool {
        // 简单的启发式方法：检查前1024字节中是否大部分都是可打印字符
        let sample_size = std::cmp::min(1024, data.len());
        if sample_size == 0 {
            return false;
        }

        let printable_count = data[..sample_size]
            .iter()
            .filter(|&&b| b >= 32 && b <= 126 || b == 9 || b == 10 || b == 13) // 可打印字符 + tab/lf/cr
            .count();

        (printable_count as f64 / sample_size as f64) > 0.8
    }

    /// 提取文件扩展名
    fn extract_extension(&self, filename: &str) -> Option<String> {
        Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
    }

    /// 判断是否为容器文件
    fn is_container_file(&self, mime_type: &str, extension: &Option<String>) -> bool {
        // 基于MIME类型判断
        match mime_type {
            "application/zip" | "application/x-zip-compressed" => return true,
            "application/x-tar" | "application/x-gzip" | "application/x-bzip2" => return true,
            "application/x-7z-compressed" | "application/x-rar-compressed" => return true,
            _ => {}
        }

        // 基于扩展名判断
        if let Some(ext) = extension {
            match ext.as_str() {
                "zip" | "jar" | "war" | "ear" => return true,
                "tar" | "gz" | "bz2" | "xz" => return true,
                "7z" | "rar" => return true,
                _ => {}
            }
        }

        false
    }

    /// 生成文件类型描述
    fn generate_file_type_description(
        &self,
        mime_type: &str,
        extension: &Option<String>,
    ) -> String {
        match mime_type {
            "application/zip" | "application/x-zip-compressed" => "ZIP压缩文件".to_string(),
            "application/x-msdownload" => "Windows可执行文件".to_string(),
            "application/x-executable" => "可执行文件".to_string(),
            "application/java-vm" => "Java字节码文件".to_string(),
            "text/plain" => "文本文件".to_string(),
            "application/pdf" => "PDF文档".to_string(),
            "image/jpeg" | "image/jpg" => "JPEG图像".to_string(),
            "image/png" => "PNG图像".to_string(),
            "image/gif" => "GIF图像".to_string(),
            _ => {
                // 基于扩展名生成描述
                if let Some(ext) = extension {
                    match ext.as_str() {
                        "exe" | "dll" | "sys" => "Windows可执行文件".to_string(),
                        "so" => "Linux共享库".to_string(),
                        "dylib" => "macOS动态库".to_string(),
                        "jar" => "Java归档文件".to_string(),
                        "apk" => "Android应用包".to_string(),
                        "ipa" => "iOS应用包".to_string(),
                        "msi" => "Windows安装包".to_string(),
                        "deb" => "Debian软件包".to_string(),
                        "rpm" => "RPM软件包".to_string(),
                        "dmg" => "macOS磁盘映像".to_string(),
                        "iso" => "光盘映像文件".to_string(),
                        "bin" => "二进制文件".to_string(),
                        "dat" => "数据文件".to_string(),
                        "log" => "日志文件".to_string(),
                        "tmp" => "临时文件".to_string(),
                        _ => format!("未知文件类型 ({})", mime_type),
                    }
                } else {
                    format!("未知文件类型 ({})", mime_type)
                }
            }
        }
    }
}

impl Default for FileAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
