use serde::{Deserialize, Serialize};

#[derive(Deserialize, Default, Clone)]
pub struct Workspace {
    pub current_dir: Option<String>,
    pub project_dir: Option<String>,
}

#[derive(Deserialize, Default, Clone)]
pub struct ModelInfo {
    pub id: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Deserialize, Default, Clone)]
pub struct CurrentUsage {
    pub cache_read_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

#[derive(Deserialize, Default, Clone)]
pub struct ContextWindow {
    pub used_percentage: Option<f64>,
    pub remaining_percentage: Option<f64>,
    pub total_input_tokens: Option<u64>,
    pub total_output_tokens: Option<u64>,
    pub context_window_size: Option<u64>,
    pub current_usage: Option<CurrentUsage>,
}

#[derive(Deserialize, Default, Clone)]
pub struct SandboxInfo {
    pub enabled: Option<bool>,
    pub allow_network: Option<bool>,
}

#[derive(Deserialize, Default, Clone)]
pub struct AgentInfo {
    pub name: Option<String>,
    pub role: Option<String>,
    pub status: Option<String>,
}

#[derive(Deserialize, Default, Clone)]
pub struct InputVcsInfo {
    #[serde(rename = "type")]
    pub vcs_type: Option<String>,
    pub client: Option<String>,
    pub branch: Option<String>,
    pub dirty: Option<bool>,
}

#[derive(Deserialize, Default, Clone)]
pub struct InputJson {
    pub agent_state: Option<String>,
    pub model: Option<ModelInfo>,
    pub workspace: Option<Workspace>,
    pub cwd: Option<String>,
    pub context_window: Option<ContextWindow>,
    pub sandbox: Option<SandboxInfo>,
    pub agent: Option<AgentInfo>,
    pub vcs: Option<InputVcsInfo>,
    pub product: Option<String>,
    pub artifacts: Option<Vec<serde::de::IgnoredAny>>,
    pub artifact_count: Option<u32>,
    pub subagents: Option<Vec<serde::de::IgnoredAny>>,
    pub background_tasks: Option<Vec<serde::de::IgnoredAny>>,
    pub task_count: Option<u32>,
    pub tool_confirmation_pending: Option<bool>,
    pub pending_input_count: Option<u32>,
    pub plan_tier: Option<String>,
    pub email: Option<String>,
    pub version: Option<String>,
    pub conversation_id: Option<String>,
    pub terminal_width: Option<usize>,
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct QuotaItem {
    pub id: String,
    pub displayName: String,
    pub remainingFraction: f64,
    pub resetTime: Option<String>,
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct VcsInfo {
    pub cwd: String,
    pub branch: String,
    pub dirty: bool,
    #[serde(default)]
    pub ahead: u32,
    #[serde(default)]
    pub behind: u32,
    #[serde(default)]
    pub modified: u32,
    pub lastChecked: u64,
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct CacheData {
    #[serde(default)]
    pub quota: Vec<QuotaItem>,
    pub vcs: Option<VcsInfo>,
    #[serde(default)]
    pub lastRefreshed: u64,
    #[serde(default)]
    pub token_hash: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct LockContent {
    pub pid: u32,
    pub time: u64,
    pub cwd: String,
}

pub fn parse_input_json(input: &str) -> InputJson {
    let clean_input = input.trim();
    let clean_input = if clean_input.starts_with('\u{feff}') {
        &clean_input[3..]
    } else {
        clean_input
    };

    if !clean_input.is_empty() {
        if let Ok(parsed) = serde_json::from_str::<InputJson>(clean_input) {
            return parsed;
        }
    }
    InputJson::default()
}

#[derive(Deserialize, Serialize, Clone)]
pub struct LayoutConfig {
    #[serde(default = "default_true")]
    pub show_state: bool,
    #[serde(default = "default_true")]
    pub show_model: bool,
    #[serde(default = "default_true")]
    pub show_path: bool,
    #[serde(default = "default_true")]
    pub show_vcs: bool,
    #[serde(default = "default_true")]
    pub show_quota: bool,
    #[serde(default = "default_true")]
    pub show_quota_bar: bool,
    #[serde(default = "default_true")]
    pub show_pending_input: bool,
    #[serde(default = "default_true")]
    pub show_approval_alert: bool,
    #[serde(default = "default_true")]
    pub show_context_bar: bool,
    #[serde(default = "default_true")]
    pub show_cache_stats: bool,
    #[serde(default = "default_true")]
    pub show_artifacts: bool,
    #[serde(default = "default_true")]
    pub show_subagents: bool,
    #[serde(default = "default_true")]
    pub show_tasks: bool,
    #[serde(default = "default_true")]
    pub show_sandbox: bool,
    #[serde(default = "default_false")]
    pub show_conversation_id: bool,
    #[serde(default = "default_false")]
    pub show_version: bool,
    #[serde(default = "default_false")]
    pub show_plan_tier: bool,
    #[serde(default = "default_false")]
    pub show_email: bool,
}

fn default_true() -> bool { true }
fn default_false() -> bool { false }

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            show_state: true,
            show_model: true,
            show_path: true,
            show_vcs: true,
            show_quota: true,
            show_quota_bar: true,
            show_pending_input: true,
            show_approval_alert: true,
            show_context_bar: true,
            show_cache_stats: true,
            show_artifacts: true,
            show_subagents: true,
            show_tasks: true,
            show_sandbox: true,
            show_conversation_id: false,
            show_version: false,
            show_plan_tier: false,
            show_email: false,
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct StatesConfig {
    #[serde(default = "default_ready")]
    pub ready: String,
    #[serde(default = "default_thinking")]
    pub thinking: String,
    #[serde(default = "default_working")]
    pub working: String,
    #[serde(default = "default_tool_use")]
    pub tool_use: String,
    #[serde(default = "default_state")]
    pub default: String,
}

fn default_ready() -> String { "\x1b[92m\x1b[1m[READY]\x1b[0m".to_string() }
fn default_thinking() -> String { "\x1b[93m\x1b[1m[THINKING]\x1b[0m".to_string() }
fn default_working() -> String { "\x1b[96m\x1b[1m[WORKING]\x1b[0m".to_string() }
fn default_tool_use() -> String { "\x1b[95m\x1b[1m[TOOL]\x1b[0m".to_string() }
fn default_state() -> String { "\x1b[97m\x1b[1m[STATE]\x1b[0m".to_string() }

impl Default for StatesConfig {
    fn default() -> Self {
        Self {
            ready: default_ready(),
            thinking: default_thinking(),
            working: default_working(),
            tool_use: default_tool_use(),
            default: default_state(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ColorsConfig {
    #[serde(default = "default_color_vcs")]
    pub vcs: String,
    #[serde(default = "default_color_path")]
    pub path: String,
    #[serde(default = "default_color_model")]
    pub model: String,
    #[serde(default = "default_color_border")]
    pub border: String,
}

fn default_color_vcs() -> String { "\x1b[94m".to_string() }
fn default_color_path() -> String { "\x1b[94m".to_string() }
fn default_color_model() -> String { "\x1b[90m\x1b[3m".to_string() }
fn default_color_border() -> String { "\x1b[90m".to_string() }

impl Default for ColorsConfig {
    fn default() -> Self {
        Self {
            vcs: default_color_vcs(),
            path: default_color_path(),
            model: default_color_model(),
            border: default_color_border(),
        }
    }
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct UserConfig {
    #[serde(default)]
    pub layout: LayoutConfig,
    #[serde(default)]
    pub states: StatesConfig,
    #[serde(default)]
    pub colors: ColorsConfig,
}

pub fn load_user_config() -> UserConfig {
    let path = crate::utils::resolve_antigravity_path("statusline.json");
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str::<UserConfig>(&content) {
                return config;
            }
        }
    } else {
        let default_config = UserConfig::default();
        if let Ok(json_str) = serde_json::to_string_pretty(&default_config) {
            let _ = std::fs::write(&path, json_str);
        }
    }
    UserConfig::default()
}

