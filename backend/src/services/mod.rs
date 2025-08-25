// 服务层模块
pub mod cape_client;
pub mod cape_instance_manager;
pub mod cape_manager;
pub mod cape_processor;
// pub mod cape_status_sync; // 已删除，使用解耦的轮询器与拉取器替代
pub mod cape_report_fetcher;
pub mod cape_status_poller;
pub mod startup_recovery;

// CFG modules
pub mod cfg_client;
pub mod cfg_instance_manager;
pub mod cfg_processor;
pub mod cfg_status_sync;

pub use cape_client::CapeClient;
pub use cape_instance_manager::CapeInstanceManager;
pub use cape_manager::{CapeManager, CapeTaskStats};
pub use cape_processor::CapeProcessor;
// pub use cape_status_sync::CapeStatusSyncer; // 已由新执行器替代，保留模块但不再导出类型
// 新增执行器的对外导出（后续在 main.rs 使用）
pub use cape_report_fetcher::CapeReportFetcher;
pub use cape_status_poller::CapeStatusPoller;
pub use startup_recovery::{RecoveryStats, StartupRecovery};

pub use cfg_client::CfgClient;
pub use cfg_instance_manager::CfgInstanceManager;
pub use cfg_processor::CfgProcessor;
pub use cfg_status_sync::CfgStatusSyncer;
