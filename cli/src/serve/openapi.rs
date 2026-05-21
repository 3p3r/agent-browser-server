use salvo_oapi::ToSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

impl From<crate::connection::Response> for ApiResponse {
    fn from(r: crate::connection::Response) -> Self {
        Self {
            success: r.success,
            data: r.data,
            error: r.error,
            warning: r.warning,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub error_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExecuteRequest {
    pub session: Option<String>,
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SessionExecuteRequest {
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BatchRequest {
    pub commands: Vec<Vec<String>>,
    #[serde(default)]
    pub bail: bool,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ChatRequest {
    pub session: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Request body for session CLI routes. Pass positional CLI args in `args`, or use
/// convenience fields (`selector`, `text`, `url`, `value`) that are appended after the path.
#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct CliCommandBody {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<Value>,
}

/// Global CLI flags exposed as query parameters on session command routes.
#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct GlobalFlagsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debug: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cdp: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_bypass: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_timeout: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_connect: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_https_errors: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_file_access: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotate: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_scheme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_boundaries: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_policy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirm_actions: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirm_interactive: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screenshot_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screenshot_quality: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screenshot_format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idle_timeout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_auto_dialog: Option<bool>,
}

