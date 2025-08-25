use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 分析器类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "analyzer_type", rename_all = "UPPERCASE")]
#[serde(rename_all = "UPPERCASE")]
pub enum AnalyzerType {
    /// CAPE沙箱分析器
    CAPE,
    /// CFG 分析器
    CFG,
    // 未来可以扩展的分析器类型
    // YARA,     // YARA规则匹配
    // VT,       // VirusTotal分析
    // CUSTOM,   // 自定义分析器
}

impl AnalyzerType {
    /// 获取分析器的显示名称
    pub fn display_name(&self) -> &str {
        match self {
            AnalyzerType::CAPE => "CAPE Sandbox",
            AnalyzerType::CFG => "CFG Analyzer",
        }
    }

    /// 获取分析器的描述
    pub fn description(&self) -> &str {
        match self {
            AnalyzerType::CAPE => "高级恶意软件分析沙箱，提供行为分析和威胁检测",
            AnalyzerType::CFG => "恶意样本CFG提取与嵌入生成",
        }
    }

    /// 检查分析器是否启用
    pub fn is_enabled(&self) -> bool {
        match self {
            AnalyzerType::CAPE => true,
            AnalyzerType::CFG => true,
        }
    }

    /// 获取所有可用的分析器
    pub fn available_analyzers() -> Vec<AnalyzerType> {
        vec![AnalyzerType::CAPE, AnalyzerType::CFG]
    }
}

impl std::fmt::Display for AnalyzerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}
