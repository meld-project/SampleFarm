use super::FileHashes;
use crate::error::AppResult;
use md5;
use sha1::{Digest as Sha1Digest, Sha1};
use sha2::{Digest as Sha256Digest, Sha256};
use tokio::task;

/// 文件哈希计算器
pub struct FileHasher;

impl FileHasher {
    /// 创建新的文件哈希计算器
    pub fn new() -> Self {
        Self
    }

    /// 计算文件的MD5、SHA1和SHA256哈希值
    pub async fn calculate_hashes(&self, data: &[u8]) -> AppResult<FileHashes> {
        let data = data.to_vec(); // 克隆数据以便在异步任务中使用

        // 在后台任务中计算哈希，避免阻塞异步运行时
        let hashes = task::spawn_blocking(move || -> AppResult<FileHashes> {
            let md5_hash = Self::calculate_md5(&data);
            let sha1_hash = Self::calculate_sha1(&data);
            let sha256_hash = Self::calculate_sha256(&data);

            Ok(FileHashes {
                md5: md5_hash,
                sha1: sha1_hash,
                sha256: sha256_hash,
            })
        })
        .await
        .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!(e)))??;

        Ok(hashes)
    }

    /// 计算MD5哈希值
    pub fn calculate_md5(data: &[u8]) -> String {
        let digest = md5::compute(data);
        format!("{:x}", digest)
    }

    /// 计算SHA1哈希值
    pub fn calculate_sha1(data: &[u8]) -> String {
        let mut hasher = Sha1::new();
        Sha1Digest::update(&mut hasher, data);
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// 计算SHA256哈希值
    pub fn calculate_sha256(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        Sha256Digest::update(&mut hasher, data);
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// 计算文件流的哈希值（用于大文件）
    pub async fn calculate_hashes_streaming<R>(&self, mut reader: R) -> AppResult<FileHashes>
    where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
    {
        let mut buffer = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut reader, &mut buffer)
            .await
            .map_err(|e| crate::error::AppError::Io(e))?;

        self.calculate_hashes(&buffer).await
    }

    /// 验证文件哈希值
    pub async fn verify_hash(
        &self,
        data: &[u8],
        expected_md5: Option<&str>,
        expected_sha256: Option<&str>,
    ) -> AppResult<bool> {
        let hashes = self.calculate_hashes(data).await?;

        if let Some(expected) = expected_md5 {
            if hashes.md5.to_lowercase() != expected.to_lowercase() {
                return Ok(false);
            }
        }

        if let Some(expected) = expected_sha256 {
            if hashes.sha256.to_lowercase() != expected.to_lowercase() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// 比较两个哈希值是否相同（用于去重检查）
    pub fn compare_hashes(hash1: &FileHashes, hash2: &FileHashes) -> bool {
        hash1.md5.to_lowercase() == hash2.md5.to_lowercase()
            || hash1.sha256.to_lowercase() == hash2.sha256.to_lowercase()
    }

    /// 批量计算多个文件的哈希值
    pub async fn calculate_batch_hashes(
        &self,
        files: Vec<(&str, Vec<u8>)>,
    ) -> AppResult<Vec<(String, FileHashes)>> {
        let mut results = Vec::new();

        // 并发计算哈希值
        let tasks: Vec<_> = files
            .into_iter()
            .map(|(filename, data)| {
                let filename = filename.to_string();
                async move {
                    let hasher = FileHasher::new();
                    let hashes = hasher.calculate_hashes(&data).await?;
                    Ok::<(String, FileHashes), crate::error::AppError>((filename, hashes))
                }
            })
            .collect();

        for task in tasks {
            let result = task.await?;
            results.push(result);
        }

        Ok(results)
    }
}

impl Default for FileHasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hash_calculation() {
        let hasher = FileHasher::new();
        let test_data = b"Hello, World!";

        let hashes = hasher.calculate_hashes(test_data).await.unwrap();

        // 验证已知的哈希值
        assert_eq!(hashes.md5, "65a8e27d8879283831b664bd8b7f0ad4");
        assert_eq!(
            hashes.sha256,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
        );
    }

    #[tokio::test]
    async fn test_hash_verification() {
        let hasher = FileHasher::new();
        let test_data = b"Hello, World!";

        let is_valid = hasher
            .verify_hash(
                test_data,
                Some("65a8e27d8879283831b664bd8b7f0ad4"),
                Some("dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"),
            )
            .await
            .unwrap();

        assert!(is_valid);

        let is_invalid = hasher
            .verify_hash(test_data, Some("invalid_md5"), None)
            .await
            .unwrap();

        assert!(!is_invalid);
    }

    #[test]
    fn test_hash_comparison() {
        let hash1 = FileHashes {
            md5: "65a8e27d8879283831b664bd8b7f0ad4".to_string(),
            sha1: "943a702d06f34599aee1f8da8ef9f7296031d699".to_string(),
            sha256: "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f".to_string(),
        };

        let hash2 = FileHashes {
            md5: "65a8e27d8879283831b664bd8b7f0ad4".to_string(),
            sha1: "943a702d06f34599aee1f8da8ef9f7296031d699".to_string(),
            sha256: "different_sha256".to_string(),
        };

        assert!(FileHasher::compare_hashes(&hash1, &hash2));

        let hash3 = FileHashes {
            md5: "different_md5".to_string(),
            sha1: "different_sha1".to_string(),
            sha256: "different_sha256".to_string(),
        };

        assert!(!FileHasher::compare_hashes(&hash1, &hash3));
    }
}
