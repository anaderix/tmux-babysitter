use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LlmConfig {
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GuardRule {
    pub name: String,
    pub description: String,
    pub response: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GuardRails {
    pub rules: Vec<GuardRule>,
    pub default_response: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TmuxConfig {
    pub session: String,
    pub window: Option<String>,
    pub pane: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub tmux: TmuxConfig,
    pub llm: LlmConfig,
    pub guard_rails: GuardRails,
    pub monitoring_interval_ms: u64,
}
