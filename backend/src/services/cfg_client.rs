use reqwest::{Client, StatusCode};
use serde_json::Value as JsonValue;
use tracing::{debug, error};

use crate::{
    config::cfg::CfgConfig,
    error::{AppError, AppResult},
};

#[derive(Debug, Clone)]
pub struct CfgClient {
    http: Client,
    base_url: String,
}

impl CfgClient {
    pub fn new(cfg: CfgConfig) -> AppResult<Self> {
        let http = Client::builder()
            .build()
            .map_err(|e| AppError::config(format!("创建HTTP客户端失败: {}", e)))?;
        Ok(Self {
            http,
            base_url: cfg.base_url,
        })
    }

    /// 使用base_url创建客户端（用于实例管理）
    pub fn new_with_base_url(base_url: String) -> AppResult<Self> {
        let http = Client::builder()
            .build()
            .map_err(|e| AppError::config(format!("创建HTTP客户端失败: {}", e)))?;
        Ok(Self { http, base_url })
    }

    fn url(&self, path: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        format!("{}/{}", base, path)
    }

    pub async fn submit_preprocess_pe(
        &self,
        file_path: &str,
        task_id: &str,
        label: i32,
    ) -> AppResult<JsonValue> {
        // 兼容旧接口：从文件路径读取
        let data = std::fs::read(file_path)
            .map_err(|e| AppError::file_processing(format!("读取待上传文件失败: {}", e)))?;
        let file_name = std::path::Path::new(file_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("upload.bin");
        self.submit_preprocess_pe_bytes(file_name, &data, task_id, label)
            .await
    }

    pub async fn submit_preprocess_pe_bytes(
        &self,
        file_name: &str,
        file_bytes: &[u8],
        task_id: &str,
        label: i32,
    ) -> AppResult<JsonValue> {
        let url = self.url("/preprocess_pe");
        let part_file = reqwest::multipart::Part::bytes(file_bytes.to_vec())
            .file_name(file_name.to_string())
            .mime_str("application/octet-stream")
            .map_err(|e| AppError::file_processing(format!("构造上传part失败: {}", e)))?;
        let form = reqwest::multipart::Form::new()
            .part("file", part_file)
            .text("task_id", task_id.to_string())
            .text("label", label.to_string());

        let resp = self
            .http
            .post(url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        debug!(len = text.len(), "CFG submit resp len");
        let json: JsonValue =
            serde_json::from_str(&text).unwrap_or(JsonValue::String(text.clone()));
        if !status.is_success() {
            // 特判任务已存在，降低日志级别，避免误报为错误
            let preview = text.chars().take(400).collect::<String>();
            let is_exists = status == StatusCode::BAD_REQUEST
                && (preview.contains("已存在")
                    || preview.to_ascii_lowercase().contains("already exist"));
            if is_exists {
                // 用 info 记录，以便上层将其视为幂等提交命中
                tracing::info!(%status, preview = %preview, "CFG 任务已存在");
            } else {
                // 其他错误仍按错误处理
                error!(%status, preview = %preview, "CFG 提交失败");
            }
            return Err(AppError::service_unavailable(format!(
                "CFG 提交失败: status={}, body_preview={}",
                status,
                text.chars().take(200).collect::<String>()
            )));
        }
        Ok(json)
    }

    pub async fn get_task_status(&self, task_id: &str) -> AppResult<JsonValue> {
        let url = self.url(&format!("/task/{}", task_id));
        let resp = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        debug!(len = text.len(), "CFG status resp len");
        let json: JsonValue =
            serde_json::from_str(&text).unwrap_or(JsonValue::String(text.clone()));
        if status == StatusCode::NOT_FOUND {
            return Err(AppError::not_found(format!(
                "CFG 任务未找到或已失败: {}",
                task_id
            )));
        }
        if !status.is_success() {
            // 状态查询失败多为服务端/任务态问题，归类为服务不可用并携带预览
            return Err(AppError::service_unavailable(format!(
                "获取CFG任务状态失败: status={}, body_preview={}",
                status,
                text.chars().take(200).collect::<String>()
            )));
        }
        Ok(json)
    }

    pub async fn get_result(&self, task_id: &str) -> AppResult<JsonValue> {
        let url = self.url(&format!("/result/{}", task_id));
        let resp = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        debug!(len = text.len(), "CFG result resp len");
        if status == StatusCode::NOT_FOUND {
            return Err(AppError::not_found(format!("CFG 结果未找到: {}", task_id)));
        }
        if !status.is_success() {
            return Err(AppError::service_unavailable(format!(
                "获取CFG结果失败: status={}, body_preview={}",
                status,
                text.chars().take(200).collect::<String>()
            )));
        }
        let json: JsonValue =
            serde_json::from_str(&text).unwrap_or(JsonValue::String(text.clone()));
        Ok(json)
    }

    pub async fn download_result_file(
        &self,
        task_id: &str,
        filename: &str,
    ) -> AppResult<bytes::Bytes> {
        let url = self.url(&format!("/download/{}/{}", task_id, filename));
        let resp = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        if !resp.status().is_success() {
            return Err(AppError::bad_request(format!(
                "下载结果文件失败: {}",
                resp.status()
            )));
        }
        let data = resp
            .bytes()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        Ok(data)
    }

    pub async fn get_system_status(&self) -> AppResult<JsonValue> {
        let url = self.url("/system/status");
        let resp = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        let text = resp.text().await.unwrap_or_default();
        let json: JsonValue =
            serde_json::from_str(&text).unwrap_or(JsonValue::String(text.clone()));
        Ok(json)
    }
}
