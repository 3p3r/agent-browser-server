use salvo::oapi::extract::*;
use salvo::prelude::*;
use serde_json::{json, Value};

use crate::connection::walk_daemons;
use crate::executor;
use crate::native::stream::chat;
use crate::serve::openapi::{ExecuteRequest, HealthResponse};
use crate::serve::routes::common::{
    err_response, ok_api, run_cli_argv, session_from_path,
};
use crate::serve::sse::chat::chat_handler;
use crate::skills;

/// Service health and version.
#[endpoint(tags("meta"))]
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

/// List active browser sessions.
#[endpoint(tags("meta"))]
async fn list_sessions() -> Json<Value> {
    let sessions: Vec<String> = walk_daemons()
        .sessions
        .into_iter()
        .map(|s| s.name)
        .collect();
    Json(json!({
        "success": true,
        "data": { "sessions": sessions },
    }))
}

/// Close all active sessions.
#[endpoint(tags("meta"))]
async fn close_all_sessions() -> Json<Value> {
    Json(executor::close_all_sessions().await)
}

/// Run any CLI command via argv list.
#[endpoint(tags("meta"))]
async fn execute_global(
    body: JsonBody<ExecuteRequest>,
    req: &mut Request,
    res: &mut Response,
) {
    let session = body
        .session
        .clone()
        .unwrap_or_else(|| "default".to_string());
    if let Err(e) = session_from_path(&session) {
        err_response(res, e);
        return;
    }
    let pairs: Vec<(String, String)> = crate::serve::flags_http::query_pairs_from_req(req);
    let pair_refs: Vec<(&str, &str)> = pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    match run_cli_argv(&session, body.args.clone(), &pair_refs, body.flags.as_ref()).await {
        Ok(api) => ok_api(res, api),
        Err(e) => err_response(res, e),
    }
}

/// List Chrome profiles on the host.
#[endpoint(tags("meta"))]
async fn profiles() -> Json<Value> {
    use crate::native::cdp::chrome::{find_chrome_user_data_dir, list_chrome_profiles};

    match find_chrome_user_data_dir() {
        Some(dir) => {
            let items: Vec<Value> = list_chrome_profiles(&dir)
                .iter()
                .map(|p| json!({ "directory": p.directory, "name": p.name }))
                .collect();
            Json(json!({ "success": true, "data": items }))
        }
        None => Json(json!({
            "success": false,
            "error": "No Chrome user data directory found",
        })),
    }
}

/// List bundled skills.
#[endpoint(tags("meta"))]
async fn skills_list() -> Json<Value> {
    Json(skills::list_skills_json())
}

/// Get a skill by name.
#[endpoint(tags("meta"))]
async fn skills_get(name: PathParam<String>, req: &Request) -> Json<Value> {
    let full = req
        .queries()
        .get("full")
        .is_some_and(|v| v == "true" || v == "1");
    Json(skills::get_skill_json(&name, full))
}

/// AI gateway status.
#[endpoint(tags("meta"))]
async fn chat_status() -> Json<Value> {
    let body: Value = serde_json::from_str(&chat::chat_status_json()).unwrap_or(json!({}));
    Json(body)
}

/// List AI gateway models.
#[endpoint(tags("meta"))]
async fn models() -> Json<Value> {
    Json(chat::fetch_models_json().await)
}

pub fn meta_router() -> Router {
    Router::new()
        .push(Router::with_path("health").get(health))
        .push(Router::with_path("sessions").get(list_sessions).delete(close_all_sessions))
        .push(Router::with_path("execute").post(execute_global))
        .push(Router::with_path("profiles").get(profiles))
        .push(Router::with_path("skills").get(skills_list))
        .push(Router::with_path("skills/{name}").get(skills_get))
        .push(Router::with_path("chat").post(chat_handler))
        .push(Router::with_path("chat/status").get(chat_status))
        .push(Router::with_path("models").get(models))
}
