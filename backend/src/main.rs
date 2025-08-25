/*
 * SampleFarm - Professional Malware Analysis Platform
 * Copyright (c) 2024 SampleFarm Project
 * 
 * This work is licensed under CC BY-NC-SA 4.0
 * https://creativecommons.org/licenses/by-nc-sa/4.0/
 */

use axum::response::Html;
use axum::{
    Router,
    extract::{DefaultBodyLimit, Query, State},
    http::Method,
    response::Json,
    routing::get,
};
use samplefarm_backend::{
    config::Config,
    database::Database,
    docs::ApiDoc,
    error::AppResult,
    file_processing::{FileProcessingConfig, FileProcessor},
    handlers::AppState,
    response::ApiResponse,
    routes::create_api_routes,
    services::{CfgInstanceManager, CfgProcessor, CfgStatusSyncer},
    storage::MinioStorage,
};
use serde::Deserialize;
use std::collections::HashMap;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;

#[derive(Deserialize)]
struct HealthQuery {
    #[serde(default)]
    detail: bool,
}

/// 健康检查处理器
async fn health_check(Query(params): Query<HealthQuery>) -> Json<ApiResponse<serde_json::Value>> {
    if params.detail {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let mut details = std::collections::HashMap::new();
        details.insert("status", "healthy");
        details.insert("version", "0.1.0");
        details.insert("timestamp", timestamp.as_str());

        Json(ApiResponse::success(serde_json::json!(details)))
    } else {
        Json(ApiResponse::success(serde_json::json!({"status": "ok"})))
    }
}

/// 系统信息处理器
async fn system_info() -> Json<ApiResponse<HashMap<&'static str, serde_json::Value>>> {
    let mut info = HashMap::new();
    info.insert("name", serde_json::json!("SampleFarm Backend"));
    info.insert("version", serde_json::json!("0.1.0"));
    info.insert(
        "build_time",
        serde_json::json!(chrono::Utc::now().to_rfc3339()),
    );

    Json(ApiResponse::success(info))
}

/// 数据库健康检查处理器
async fn db_health_check(
    State(app_state): State<AppState>,
) -> Json<ApiResponse<serde_json::Value>> {
    match &app_state.database {
        Some(db) => match db.health_check().await {
            Ok(true) => {
                let timestamp = chrono::Utc::now().to_rfc3339();
                let mut details = HashMap::new();
                details.insert("database", "healthy");
                details.insert("timestamp", timestamp.as_str());
                Json(ApiResponse::success(serde_json::json!(details)))
            }
            Ok(false) => Json(ApiResponse::error_with_data(
                503,
                "数据库连接异常".to_string(),
                serde_json::json!({"status": "unhealthy"}),
            )),
            Err(e) => {
                tracing::error!("数据库健康检查失败: {}", e);
                Json(ApiResponse::error_with_data(
                    503,
                    format!("数据库健康检查失败: {}", e),
                    serde_json::json!({"status": "error"}),
                ))
            }
        },
        None => Json(ApiResponse::error_with_data(
            503,
            "数据库未配置或连接失败".to_string(),
            serde_json::json!({"status": "unavailable"}),
        )),
    }
}

/// 存储健康检查处理器
async fn storage_health_check(
    State(app_state): State<AppState>,
) -> Json<ApiResponse<serde_json::Value>> {
    match &app_state.storage {
        Some(storage) => match storage.health_check().await {
            Ok(true) => {
                let timestamp = chrono::Utc::now().to_rfc3339();
                let mut details = HashMap::new();
                details.insert("storage", "healthy");
                details.insert("timestamp", timestamp.as_str());
                Json(ApiResponse::success(serde_json::json!(details)))
            }
            Ok(false) => Json(ApiResponse::error_with_data(
                503,
                "存储服务连接异常".to_string(),
                serde_json::json!({"status": "unhealthy"}),
            )),
            Err(e) => {
                tracing::error!("存储健康检查失败: {}", e);
                Json(ApiResponse::error_with_data(
                    503,
                    format!("存储健康检查失败: {}", e),
                    serde_json::json!({"status": "error"}),
                ))
            }
        },
        None => Json(ApiResponse::error_with_data(
            503,
            "存储服务未配置或连接失败".to_string(),
            serde_json::json!({"status": "unavailable"}),
        )),
    }
}

/// 文件处理器健康检查处理器
async fn file_processor_health_check(
    State(app_state): State<AppState>,
) -> Json<ApiResponse<serde_json::Value>> {
    match &app_state.file_processor {
        Some(_processor) => {
            let timestamp = chrono::Utc::now().to_rfc3339();
            let mut details = HashMap::new();
            details.insert("file_processor", "healthy");
            details.insert("timestamp", timestamp.as_str());
            Json(ApiResponse::success(serde_json::json!(details)))
        }
        None => Json(ApiResponse::error_with_data(
            503,
            "文件处理器未配置或初始化失败".to_string(),
            serde_json::json!({"status": "unavailable"}),
        )),
    }
}

#[tokio::main]
async fn main() -> AppResult<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "samplefarm_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 加载配置
    let config = match Config::from_file("config.toml") {
        Ok(config) => {
            tracing::info!("已加载配置文件: config.toml");
            config
        }
        Err(_) => {
            tracing::warn!("未找到配置文件，使用默认配置");
            let default_config = Config::default();
            // 保存默认配置到文件
            if let Err(e) = default_config.save_to_file("config.toml") {
                tracing::warn!("保存默认配置失败: {}", e);
            }
            default_config
        }
    };

    tracing::info!("服务器配置: {}", config.server_addr());

    // 初始化数据库（如果连接失败则继续启动，但记录警告）
    let database = match Database::new(&config.database).await {
        Ok(db) => {
            // 验证数据库连接和版本
            if let Err(e) = db.verify_connection().await {
                tracing::warn!("数据库验证失败: {}", e);
            }
            Some(db)
        }
        Err(e) => {
            tracing::warn!("数据库连接失败，服务将在无数据库模式下启动: {}", e);
            None
        }
    };

    // 初始化MinIO存储（如果连接失败则继续启动，但记录警告）
    let storage = match MinioStorage::new(config.minio.clone()).await {
        Ok(storage) => {
            // 确保默认bucket存在
            if let Err(e) = storage.ensure_bucket(&config.minio.bucket).await {
                tracing::warn!("创建默认bucket失败: {}", e);
            }
            Some(storage)
        }
        Err(e) => {
            tracing::warn!("MinIO存储连接失败，存储服务将不可用: {}", e);
            None
        }
    };

    // 初始化文件处理器
    let file_processor = match FileProcessor::new(FileProcessingConfig::default()) {
        Ok(processor) => {
            tracing::info!("文件处理器初始化成功");
            Some(processor)
        }
        Err(e) => {
            tracing::warn!("文件处理器初始化失败: {}", e);
            None
        }
    };

    // 初始化CAPE管理器
    let cape_manager = if let (Some(db), Some(storage_ref)) = (&database, &storage) {
        match samplefarm_backend::services::CapeManager::new(db.pool().clone(), storage_ref.clone())
            .await
        {
            Ok(manager) => {
                tracing::info!("CAPE管理器初始化成功");
                Some(manager)
            }
            Err(e) => {
                tracing::warn!("CAPE管理器初始化失败: {}", e);
                None
            }
        }
    } else {
        tracing::warn!("CAPE管理器初始化跳过：缺少必要的依赖（数据库/存储）");
        None
    };

    // 创建应用状态
    let app_state = AppState {
        database,
        storage,
        file_processor,
        cape_manager: cape_manager.clone(),
        config: config.clone(),
    };

    // 启动CAPE状态轮询器与报告拉取器（新解耦执行器）
    if let (Some(cape_mgr), Some(db)) = (&cape_manager, &app_state.database) {
        let instance_manager = cape_mgr.instance_manager().clone();
        let pool = db.pool().clone();
        let task_repo = samplefarm_backend::repositories::TaskRepository::new(pool.clone());

        let poll_interval_secs = config
            .cape
            .as_ref()
            .map(|c| c.status_check_interval_seconds)
            .unwrap_or(30);

        // 启动状态轮询器
        let poller = samplefarm_backend::services::CapeStatusPoller::new(
            task_repo.clone(),
            std::sync::Arc::new(instance_manager.clone()),
            poll_interval_secs,
        );
        tracing::info!("启动CAPE状态轮询器，间隔: {}秒", poll_interval_secs);
        tokio::spawn(async move {
            poller.start().await;
        });

        // 启动报告拉取器
        let fetcher = samplefarm_backend::services::CapeReportFetcher::new(
            task_repo.clone(),
            std::sync::Arc::new(instance_manager.clone()),
            poll_interval_secs,
        );
        tracing::info!("启动CAPE报告拉取器，间隔: {}秒", poll_interval_secs);
        tokio::spawn(async move {
            fetcher.start().await;
        });
    }

    // 初始化CFG组件（从数据库管理实例，不再依赖配置文件）
    let _cfg_components =
        if let (Some(db), Some(storage_ref)) = (&app_state.database, &app_state.storage) {
            // 初始化 CFG 实例管理器（从数据库加载）
            match CfgInstanceManager::new(db.pool().clone()).await {
                Ok(manager) => {
                    let manager = std::sync::Arc::new(manager);

                    // 检查是否有可用的CFG实例
                    if manager.health_check().await.unwrap_or(false) {
                        // 确保CFG结果bucket存在（使用默认bucket名称）
                        let cfg_result_bucket = "cfg-results".to_string();
                        if let Err(e) = storage_ref.ensure_bucket(&cfg_result_bucket).await {
                            tracing::warn!("创建CFG结果bucket失败: {}", e);
                        }

                        let processor = std::sync::Arc::new(CfgProcessor::new(
                            manager.clone(),
                            std::sync::Arc::new(db.clone()),
                            std::sync::Arc::new(storage_ref.clone()),
                            cfg_result_bucket,
                            config.minio.bucket.clone(),
                        ));

                        let syncer = std::sync::Arc::new(CfgStatusSyncer::new(
                            std::sync::Arc::new(db.clone()),
                            manager.clone(),
                            processor.clone(),
                            60, // 默认60秒同步间隔
                        ));
                        tokio::spawn(async move {
                            syncer.start_sync_loop().await;
                        });
                        tracing::info!("CFG状态同步器已启动，检查间隔: 60秒");

                        Some((manager, processor))
                    } else {
                        tracing::info!("没有可用的CFG实例，CFG功能暂时禁用");
                        None
                    }
                }
                Err(e) => {
                    tracing::warn!("CFG实例管理器初始化失败: {}", e);
                    None
                }
            }
        } else {
            None
        };

    // 临时禁用启动恢复服务以消除与状态同步器的竞争
    // TODO: 重新设计启动恢复服务，避免与状态同步器冲突
    tracing::info!("启动恢复服务已临时禁用，pending任务将由状态同步器处理");

    // 创建CORS中间件
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(vec![
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any);

    // 创建主路由
    /// Swagger UI 页面（访问路径：/swagger-ui 或 /swagger-ui/）
    /// OpenAPI JSON 路径：/api-docs/openapi.json
    async fn swagger_ui_page() -> Html<String> {
        let html = r#"<!DOCTYPE html>
<html>
<head>
  <meta charset=UTF-8>
  <title>SampleFarm API 文档</title>
  <link rel=stylesheet href=https://cdn.jsdelivr.net/npm/swagger-ui-dist@5.11.0/swagger-ui.css>
  <style>
    body { margin: 0; font-family: Arial, sans-serif; }
    #swagger-ui { max-width: 100%; }
  </style>
</head>
<body>
  <div id=swagger-ui>
    <div style="padding: 50px; text-align: center;">正在加载 API 文档...</div>
  </div>
  <script src=https://cdn.jsdelivr.net/npm/swagger-ui-dist@5.11.0/swagger-ui-bundle.js></script>
  <script src=https://cdn.jsdelivr.net/npm/swagger-ui-dist@5.11.0/swagger-ui-standalone-preset.js></script>
  <script>
    window.onload = function() {
      try {
        window.ui = SwaggerUIBundle({
          url: '/api-docs/openapi.json',
          dom_id: '#swagger-ui',
          deepLinking: true,
          presets: [SwaggerUIBundle.presets.apis, SwaggerUIStandalonePreset],
          layout: 'StandaloneLayout',
          validatorUrl: null
        });
      } catch (error) {
        console.error('SwaggerUI error:', error);
        document.getElementById('swagger-ui').innerHTML = '<h2>Failed to load API docs</h2><a href="/api-docs/openapi.json">View raw OpenAPI JSON</a>';
      }
    };
  </script>
</body>
</html>"#.to_string();
        Html(html)
    }

    let app = Router::new()
        // 健康检查和系统信息
        .route("/health", get(health_check))
        .route("/api/system/info", get(system_info))
        .route("/api/health/db", get(db_health_check))
        .route("/api/health/storage", get(storage_health_check))
        .route(
            "/api/health/file-processor",
            get(file_processor_health_check),
        )
        // OpenAPI JSON 路由
        .route(
            "/api-docs/openapi.json",
            get(|| async { Json(ApiDoc::openapi()) }),
        )
        // Swagger UI 页面
        .route("/swagger-ui", get(swagger_ui_page))
        .route("/swagger-ui/", get(swagger_ui_page))
        // 调试测试页面
        .route(
            "/swagger-test",
            get(|| async {
                Html(
                    r#"<!DOCTYPE html>
<html>
<head><title>SwaggerUI 测试</title></head>
<body>
<h1>SwaggerUI 加载测试</h1>
<p><a href="/api-docs/openapi.json" target="_blank">测试 OpenAPI JSON</a></p>
<p><a href="/swagger-ui" target="_blank">测试 SwaggerUI</a></p>
<script>
console.log('测试页面已加载');
fetch('/api-docs/openapi.json')
  .then(response => response.json())
  .then(data => console.log('OpenAPI JSON 加载成功:', data))
  .catch(error => console.error('OpenAPI JSON 加载失败:', error));
</script>
</body>
</html>"#
                        .to_string(),
                )
            }),
        )
        // 业务API路由
        .merge(create_api_routes())
        .with_state(app_state)
        .layer(DefaultBodyLimit::max(config.file.max_size as usize)) // 设置请求体大小限制
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    // 启动服务器
    let listener = tokio::net::TcpListener::bind(&config.server_addr()).await?;
    tracing::info!("🚀 服务器启动成功，监听地址: {}", config.server_addr());

    axum::serve(listener, app).await?;

    Ok(())
}
