use crate::error::AppError;
use chrono::{DateTime, Utc};
use reqwest::{
    Client,
    multipart::{Form, Part},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::{collections::HashMap, path::Path, time::Duration};
use tokio::{fs, time::sleep};
use tracing::{debug, error, info, warn};

/// CAPE Sandbox 客户端
#[derive(Debug, Clone)]
pub struct CapeClient {
    client: Client,
    base_url: String,
}

/// CAPE 任务提交响应
#[derive(Debug, Deserialize, Serialize)]
pub struct CapeTaskResponse {
    pub error: bool,
    pub data: Option<CapeTaskData>,
    pub errors: Option<Vec<JsonValue>>, // 服务器可能返回字符串或对象，放宽为 JsonValue
    pub error_value: Option<String>,    // 某些情况下提供简要错误信息
    pub url: Option<Vec<String>>,
}

/// CAPE 任务数据
#[derive(Debug, Deserialize, Serialize)]
pub struct CapeTaskData {
    pub task_id: Option<i32>,
    pub task_ids: Option<Vec<i32>>,
    pub message: Option<String>,
}

/// CAPE 任务状态响应
#[derive(Debug, Deserialize, Serialize)]
pub struct CapeTaskStatus {
    pub error: bool,
    pub data: Option<String>, // 状态字符串，如 "running", "reported", "completed"
    pub error_value: Option<String>, // 错误信息
}

/// CAPE 任务信息
#[derive(Debug, Deserialize, Serialize)]
pub struct CapeTaskInfo {
    pub id: i32,
    pub status: String,
    pub target: String,
    pub category: String,
    pub timeout: Option<i32>,
    pub priority: Option<i32>,
    pub machine: Option<String>,
    pub package: Option<String>,
    pub tags: Option<Vec<String>>,
    pub completed_on: Option<String>,
    pub added_on: Option<String>,
    pub started_on: Option<String>,
    pub processing: Option<String>,
    pub errors: Option<JsonValue>,
}

/// CAPE 静态分析
#[derive(Debug, Deserialize, Serialize)]
pub struct CapeStaticAnalysis {
    pub pe: Option<JsonValue>,
    pub strings: Option<Vec<String>>,
    pub imports: Option<Vec<CapeImport>>,
    pub exports: Option<Vec<CapeExport>>,
}

/// CAPE 导入函数
#[derive(Debug, Deserialize, Serialize)]
pub struct CapeImport {
    pub dll: String,
    pub functions: Vec<String>,
}

/// CAPE 导出函数
#[derive(Debug, Deserialize, Serialize)]
pub struct CapeExport {
    pub address: String,
    pub name: String,
}

/// CAPE 调试信息
#[derive(Debug, Deserialize, Serialize)]
pub struct CapeDebugInfo {
    pub errors: Option<Vec<String>>,
    pub log: Option<JsonValue>, // 可能是字符串或字符串数组
}

/// CAPE报告（包含原始文本与解析后的JSON）
#[derive(Debug, Clone)]
pub struct CapeReport {
    pub json: serde_json::Value,
    pub raw_text: String,
}

/// 任务执行统计
#[derive(Debug, Clone, Serialize)]
pub struct TaskExecutionStats {
    pub submit_start_time: DateTime<Utc>,
    pub submit_end_time: Option<DateTime<Utc>>,
    pub submit_duration: Option<Duration>,
    pub analysis_start_time: Option<DateTime<Utc>>,
    pub analysis_end_time: Option<DateTime<Utc>>,
    pub analysis_duration: Option<Duration>,
    pub total_duration: Option<Duration>,
    pub status_check_count: u32,
    pub status_check_interval: Duration,
    pub file_size: u64,
    pub throughput_mbps: Option<f64>,
}

impl CapeClient {
    /// 创建新的 CAPE 客户端（无超时限制）
    pub fn new(base_url: String) -> Self {
        let client = Client::builder().build().expect("创建HTTP客户端失败");

        Self { client, base_url }
    }

    /// 提交文件进行分析
    pub async fn submit_file(
        &self,
        file_path: &Path,
        machine: Option<&str>,
        options: Option<HashMap<String, String>>,
    ) -> Result<(i32, TaskExecutionStats), AppError> {
        let start_time = Utc::now();

        info!("开始提交文件到CAPE进行分析: {:?}", file_path);

        // 读取文件
        let file_data = fs::read(file_path)
            .await
            .map_err(|e| AppError::bad_request(format!("读取文件失败: {}", e)))?;

        let file_size = file_data.len() as u64;
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown_file");

        info!("准备提交文件: {}, 大小: {} 字节", file_name, file_size);

        if file_size == 0 {
            return Err(AppError::bad_request(format!(
                "文件 {} 为空，无法提交到CAPE",
                file_name
            )));
        }

        // 构建multipart表单
        let mut form = Form::new().part(
            "file",
            Part::bytes(file_data)
                .file_name(file_name.to_string())
                .mime_str("application/octet-stream")
                .map_err(|e| AppError::bad_request(format!("设置文件MIME类型失败: {}", e)))?,
        );

        // 添加机器名称
        if let Some(machine_name) = machine {
            form = form.text("machine", machine_name.to_string());
        }

        // 添加其他选项
        if let Some(opts) = options {
            for (key, value) in opts {
                form = form.text(key, value);
            }
        }

        // 发送请求
        let url = format!("{}/tasks/create/file/", self.base_url);
        debug!("提交文件到URL: {}", url);

        let response = self
            .client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| AppError::service_unavailable(format!("提交文件到CAPE失败: {}", e)))?;

        let submit_end_time = Utc::now();
        let submit_duration = submit_end_time.signed_duration_since(start_time);

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| AppError::service_unavailable(format!("读取响应失败: {}", e)))?;

        debug!("CAPE响应状态: {}, 内容: {}", status, response_text);

        if !status.is_success() {
            return Err(AppError::service_unavailable(format!(
                "CAPE返回错误状态 {}: {}",
                status, response_text
            )));
        }

        // 解析响应（放宽 errors 字段的类型）
        let cape_response: CapeTaskResponse = serde_json::from_str(&response_text)
            .map_err(|e| AppError::service_unavailable(format!("解析CAPE响应失败: {}", e)))?;

        // 检查是否有错误
        if cape_response.error {
            // 兼容多种错误返回结构
            let mut messages: Vec<String> = Vec::new();
            if let Some(error_value) = cape_response.error_value {
                messages.push(error_value);
            }
            if let Some(errs) = cape_response.errors {
                for item in errs {
                    match item {
                        JsonValue::String(s) => messages.push(s),
                        JsonValue::Object(obj) => {
                            // 将对象展平成 key: value 的可读字符串
                            let flat = obj
                                .iter()
                                .map(|(k, v)| format!("{}: {}", k, v))
                                .collect::<Vec<_>>()
                                .join(", ");
                            messages.push(flat);
                        }
                        other => messages.push(other.to_string()),
                    }
                }
            }
            let error_msg = if messages.is_empty() {
                "CAPE返回未知错误".to_string()
            } else {
                messages.join(" | ")
            };
            // 将原始响应一并包含，便于持久化和前端展示
            return Err(AppError::service_unavailable(format!(
                "CAPE分析失败: {} | raw: {}",
                error_msg, response_text
            )));
        }

        // 提取任务ID
        let task_data = cape_response
            .data
            .ok_or_else(|| AppError::service_unavailable("CAPE响应缺少数据字段".to_string()))?;

        let task_id = task_data
            .task_id
            .or_else(|| task_data.task_ids.and_then(|ids| ids.into_iter().next()))
            .ok_or_else(|| AppError::service_unavailable("CAPE未返回任务ID".to_string()))?;

        let stats = TaskExecutionStats {
            submit_start_time: start_time,
            submit_end_time: Some(submit_end_time),
            submit_duration: Some(Duration::from_secs(
                submit_duration.num_seconds().max(0) as u64
            )),
            analysis_start_time: None,
            analysis_end_time: None,
            analysis_duration: None,
            total_duration: None,
            status_check_count: 0,
            status_check_interval: Duration::from_secs(30),
            file_size,
            throughput_mbps: None,
        };

        info!(
            "文件提交成功，任务ID: {}, 耗时: {:?}",
            task_id, submit_duration
        );

        Ok((task_id, stats))
    }

    /// 查询任务状态
    pub async fn get_task_status(&self, task_id: i32) -> Result<CapeTaskStatus, AppError> {
        let url = format!("{}/tasks/status/{}/", self.base_url, task_id);
        debug!("查询任务状态: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询任务状态失败: {}", e)))?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| AppError::service_unavailable(format!("读取状态响应失败: {}", e)))?;

        if !status.is_success() {
            return Err(AppError::service_unavailable(format!(
                "CAPE状态查询返回错误 {}: {}",
                status, response_text
            )));
        }

        let task_status: CapeTaskStatus = serde_json::from_str(&response_text)
            .map_err(|e| AppError::service_unavailable(format!("解析任务状态失败: {}", e)))?;

        if task_status.error {
            if let Some(error_value) = &task_status.error_value {
                debug!("任务 {} 查询失败: {}", task_id, error_value);
            }
        } else if let Some(data) = &task_status.data {
            debug!("任务 {} 状态: {}", task_id, data);
        }

        Ok(task_status)
    }

    /// 获取任务列表 (批量轮询)
    /// 返回原始JSON格式，用于批量状态同步
    pub async fn get_tasks_list(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<serde_json::Value, AppError> {
        let url = format!("{}/tasks/list/{}/{}/", self.base_url, limit, offset);
        debug!("获取任务列表: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::service_unavailable(format!("获取任务列表失败: {}", e)))?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| AppError::service_unavailable(format!("读取列表响应失败: {}", e)))?;

        if !status.is_success() {
            return Err(AppError::service_unavailable(format!(
                "CAPE任务列表查询返回错误 {}: {}",
                status, response_text
            )));
        }

        debug!("任务列表响应长度: {} 字节", response_text.len());
        if response_text.len() < 1000 {
            debug!("任务列表响应: {}", response_text);
        } else {
            debug!("任务列表响应（前500字符）: {}...", &response_text[..500]);
        }

        let list_data: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| AppError::service_unavailable(format!("解析任务列表JSON失败: {}", e)))?;

        // 检查是否是错误响应
        if let Some(obj) = list_data.as_object() {
            if obj.get("error").and_then(|v| v.as_bool()).unwrap_or(false) {
                let err_msg = obj
                    .get("error_value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("CAPE返回未知错误");
                return Err(AppError::service_unavailable(format!(
                    "获取任务列表失败: {}",
                    err_msg
                )));
            }
        }

        debug!("成功获取任务列表，limit={}, offset={}", limit, offset);

        Ok(list_data)
    }

    /// 获取分析报告原始JSON（返回原始文本与解析后的JSON）
    pub async fn get_report_raw(&self, task_id: i32) -> Result<CapeReport, AppError> {
        let url = format!("{}/tasks/get/report/{}/", self.base_url, task_id);
        debug!("获取任务 {} 的原始CAPE报告: {}", task_id, url);

        // 提高重试次数，适配多实例下报告生成延迟
        let max_attempts = 20u32;
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let response =
                self.client.get(&url).send().await.map_err(|e| {
                    AppError::service_unavailable(format!("获取分析报告失败: {}", e))
                })?;

            let status = response.status();
            let response_text = response
                .text()
                .await
                .map_err(|e| AppError::service_unavailable(format!("读取报告响应失败: {}", e)))?;

            if !status.is_success() {
                return Err(AppError::service_unavailable(format!(
                    "CAPE报告查询返回错误 {}: {}",
                    status, response_text
                )));
            }

            // 打印原始响应以便调试
            debug!("CAPE原始报告响应长度: {} 字节", response_text.len());
            if response_text.len() < 2000 {
                debug!("CAPE原始报告响应: {}", response_text);
            } else {
                debug!(
                    "CAPE原始报告响应（前500字符）: {}...",
                    &response_text[..500]
                );
            }

            let report_json: serde_json::Value = match serde_json::from_str(&response_text) {
                Ok(v) => v,
                Err(e) => {
                    error!("解析CAPE JSON报告失败: {}", e);
                    error!(
                        "响应前200字符: {}",
                        &response_text[..response_text.len().min(200)]
                    );
                    return Err(AppError::service_unavailable(format!(
                        "解析JSON报告失败: {}",
                        e
                    )));
                }
            };

            // 如为错误包裹，判断是否需要重试
            if let Some(obj) = report_json.as_object() {
                if obj.get("error").and_then(|v| v.as_bool()).unwrap_or(false) {
                    let err_msg = obj
                        .get("error_value")
                        .and_then(|v| v.as_str())
                        .unwrap_or("CAPE返回未知错误");
                    warn!("获取原始报告返回错误: {}", err_msg);
                    if err_msg.contains("still being analyzed") && attempt < max_attempts {
                        // 指数退避：从1500ms开始，每次*1.3，上限6000ms
                        let backoff_ms = (1500.0
                            * (1.3_f64).powi((attempt as i32).saturating_sub(1)))
                        .min(6000.0) as u64;
                        debug!(
                            "报告尚未就绪，{}ms 后重试，第 {}/{} 次",
                            backoff_ms, attempt, max_attempts
                        );
                        sleep(Duration::from_millis(backoff_ms)).await;
                        continue;
                    }
                    return Err(AppError::service_unavailable(format!(
                        "获取报告失败: {}",
                        err_msg
                    )));
                }
            }

            // 打印报告结构概览
            debug!("成功获取任务 {} 的原始报告", task_id);
            if let Some(obj) = report_json.as_object() {
                debug!("CAPE报告顶层字段: {:?}", obj.keys().collect::<Vec<_>>());
                if !obj.contains_key("info") {
                    warn!("CAPE报告缺少 'info' 字段");
                }
                if let Some(info) = obj.get("info") {
                    if let Some(info_obj) = info.as_object() {
                        debug!(
                            "CAPE报告 info 字段: {:?}",
                            info_obj.keys().collect::<Vec<_>>()
                        );
                    }
                }
            }
            return Ok(CapeReport {
                json: report_json,
                raw_text: response_text,
            });
        }
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<(), AppError> {
        // 使用一个简单的任务状态查询来检查CAPE服务是否可用
        // 查询一个不存在的任务ID（99999）来测试连接
        let url = format!("{}/tasks/status/99999/", self.base_url);

        debug!("CAPE健康检查: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::service_unavailable(format!("CAPE健康检查失败: {}", e)))?;

        // 对于健康检查，我们只关心能否连接到服务
        // 即使返回404（任务不存在）也表示服务正常
        if response.status().is_success() || response.status() == 404 {
            debug!("CAPE健康检查成功");
            Ok(())
        } else {
            Err(AppError::service_unavailable(format!(
                "CAPE服务返回错误状态: {}",
                response.status()
            )))
        }
    }

    /// 轮询等待任务完成（无超时限制）
    pub async fn wait_for_completion(
        &self,
        task_id: i32,
        mut stats: TaskExecutionStats,
        _max_wait_time: Duration, // 保留参数以保持API兼容性，但不使用
        check_interval: Duration,
    ) -> Result<(CapeTaskStatus, TaskExecutionStats), AppError> {
        let _start_wait_time = Utc::now();
        stats.status_check_interval = check_interval;

        info!(
            "开始轮询任务 {} 状态（无超时限制，直到任务完成或失败）",
            task_id
        );

        loop {
            stats.status_check_count += 1;
            let task_status = self.get_task_status(task_id).await?;

            // 检查是否有错误
            if task_status.error {
                return Err(AppError::service_unavailable(format!(
                    "CAPE任务 {} 查询状态时返回错误",
                    task_id
                )));
            }

            // 检查任务状态
            let status_str = task_status.data.as_deref().unwrap_or("unknown");
            match status_str {
                "pending" => {
                    debug!("任务 {} 状态: 等待处理", task_id);
                }
                "running" => {
                    if stats.analysis_start_time.is_none() {
                        stats.analysis_start_time = Some(Utc::now());
                        info!("任务 {} 开始分析", task_id);
                    }
                    debug!("任务 {} 状态: 正在分析", task_id);
                }
                "completed" => {
                    debug!("任务 {} 状态: 分析完成，等待生成报告", task_id);
                    // completed 不是最终状态，继续等待到 reported
                }
                "reported" => {
                    // reported 才是成功的最终状态
                    let end_time = Utc::now();
                    stats.analysis_end_time = Some(end_time);

                    if let Some(analysis_start) = stats.analysis_start_time {
                        let analysis_duration = end_time.signed_duration_since(analysis_start);
                        stats.analysis_duration = Some(Duration::from_secs(
                            analysis_duration.num_seconds().max(0) as u64,
                        ));
                    }

                    let total_duration = end_time.signed_duration_since(stats.submit_start_time);
                    stats.total_duration = Some(Duration::from_secs(
                        total_duration.num_seconds().max(0) as u64,
                    ));

                    // 计算吞吐量 (MB/s)
                    if let Some(total_dur) = stats.total_duration {
                        let total_seconds = total_dur.as_secs_f64();
                        if total_seconds > 0.0 {
                            let mb_size = stats.file_size as f64 / (1024.0 * 1024.0);
                            stats.throughput_mbps = Some(mb_size / total_seconds);
                        }
                    }

                    info!(
                        "任务 {} 分析完成并生成报告，总耗时: {:?}, 状态检查次数: {}",
                        task_id, stats.total_duration, stats.status_check_count
                    );

                    return Ok((task_status, stats));
                }
                "failed_analysis" | "failed_processing" | "failed_reporting" => {
                    warn!("任务 {} 分析失败，状态: {}", task_id, status_str);
                    return Err(AppError::service_unavailable(format!(
                        "CAPE任务 {} 分析失败: {}",
                        task_id, status_str
                    )));
                }
                "failed" => {
                    // 兼容旧版本的 failed 状态
                    warn!("任务 {} 分析失败", task_id);
                    return Err(AppError::service_unavailable(format!(
                        "CAPE任务 {} 分析失败",
                        task_id
                    )));
                }
                status => {
                    debug!("任务 {} 未知状态: {}", task_id, status);
                }
            }

            // 等待下次检查
            tokio::time::sleep(check_interval).await;
        }
    }
}

impl TaskExecutionStats {
    /// 生成性能报告
    pub fn performance_report(&self) -> String {
        let mut report = String::new();

        report.push_str(&format!("=== CAPE 任务执行统计 ===\n"));
        report.push_str(&format!(
            "文件大小: {:.2} MB\n",
            self.file_size as f64 / (1024.0 * 1024.0)
        ));

        if let Some(submit_dur) = self.submit_duration {
            report.push_str(&format!("提交耗时: {:?}\n", submit_dur));
        }

        if let Some(analysis_dur) = self.analysis_duration {
            report.push_str(&format!("分析耗时: {:?}\n", analysis_dur));
        }

        if let Some(total_dur) = self.total_duration {
            report.push_str(&format!("总耗时: {:?}\n", total_dur));
        }

        report.push_str(&format!("状态检查次数: {}\n", self.status_check_count));
        report.push_str(&format!("状态检查间隔: {:?}\n", self.status_check_interval));

        if let Some(throughput) = self.throughput_mbps {
            report.push_str(&format!("平均吞吐量: {:.2} MB/s\n", throughput));
        }

        report.push_str(&format!("===========================\n"));

        report
    }

    /// 获取预计剩余时间（基于历史统计）
    pub fn estimate_remaining_time(&self, historical_avg: Option<Duration>) -> Option<Duration> {
        if let Some(analysis_start) = self.analysis_start_time {
            let elapsed = Utc::now().signed_duration_since(analysis_start);
            let elapsed_duration = Duration::from_secs(elapsed.num_seconds().max(0) as u64);

            if let Some(avg_duration) = historical_avg {
                if avg_duration > elapsed_duration {
                    return Some(avg_duration - elapsed_duration);
                }
            }
        }
        None
    }
}
