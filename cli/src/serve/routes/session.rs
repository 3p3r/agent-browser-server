use salvo::oapi::extract::*;
use salvo::prelude::*;
use serde_json::{json, Value};

use crate::connection::get_socket_dir;
use crate::executor::{self, ExecutorContext, ExecutorOutcome};
use crate::serve::http_dispatch::dispatch_route;
use crate::serve::openapi::{BatchRequest, SessionExecuteRequest};
use crate::serve::routes::common::{
    err_response, ok_api, run_cli_argv, session_from_path, HttpError,
};
use crate::serve::sse;

/// Run a CLI command in a session via argv list.
#[endpoint(tags("sessions"))]
async fn session_execute(
    session: PathParam<String>,
    body: JsonBody<SessionExecuteRequest>,
    req: &mut Request,
    res: &mut Response,
) {
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

/// Session daemon and stream status.
#[endpoint(tags("sessions"))]
async fn session_status(session: PathParam<String>) -> Json<Value> {
    if let Err(e) = session_from_path(&session) {
        return Json(e.body);
    }
    let stream_file = get_socket_dir().join(format!("{}.stream", session));
    let stream_port = std::fs::read_to_string(&stream_file)
        .ok()
        .and_then(|s| s.trim().parse::<u16>().ok());
    let stream_ws_url = stream_port.map(|p| format!("ws://127.0.0.1:{}", p));
    Json(json!({
        "success": true,
        "data": {
            "session": session.as_str(),
            "streamPort": stream_port,
            "streamWsUrl": stream_ws_url,
        }
    }))
}

/// Run batch commands (JSON array or SSE stream).
#[endpoint(tags("sessions"))]
async fn batch(
    session: PathParam<String>,
    body: JsonBody<BatchRequest>,
    req: &mut Request,
    res: &mut Response,
) {
    if let Err(e) = session_from_path(&session) {
        err_response(res, e);
        return;
    }
    if body.stream {
        sse::batch::stream_batch(&session, &body, req, res).await;
        return;
    }

    let pairs: Vec<(String, String)> = crate::serve::flags_http::query_pairs_from_req(req);
    let flags = crate::flags::flags_from_http(
        &pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect::<Vec<_>>(),
        None,
    );
    let flags = crate::flags::Flags {
        session: session.to_string(),
        ..flags
    };

    let cmd = json!({
        "id": crate::commands::gen_id(),
        "action": "batch",
        "bail": body.bail,
        "commands": body.commands,
    });
    let ctx = ExecutorContext { flags: &flags };
    match executor::execute_command_value(cmd, &ctx).await {
        Ok(ExecutorOutcome::Batch(items)) => {
            let results: Vec<Value> = items
                .iter()
                .map(|item| {
                    if let Some(ref err) = item.parse_error {
                        json!({ "index": item.index, "success": false, "error": err })
                    } else if let Some(ref err) = item.transport_error {
                        json!({ "index": item.index, "success": false, "error": err })
                    } else if let Some(ref resp) = item.response {
                        json!({
                            "index": item.index,
                            "success": resp.success,
                            "data": resp.data,
                            "error": resp.error,
                        })
                    } else {
                        json!({ "index": item.index, "success": false })
                    }
                })
                .collect();
            res.render(Json(json!({ "success": true, "data": results })));
        }
        Ok(ExecutorOutcome::Response(_)) => {
            err_response(
                res,
                HttpError::bad_request("Unexpected single response for batch", None),
            );
        }
        Err(e) => err_response(res, crate::serve::routes::common::executor_error(e)),
    }
}

/// Session CLI command with typed params and correct HTTP verb (GET/POST/DELETE).
#[endpoint(tags("sessions"))]
async fn session_command(
    session: PathParam<String>,
    req: &mut Request,
    res: &mut Response,
) {
    let rest = req
        .params()
        .get("rest")
        .cloned()
        .unwrap_or_default();
    if rest.is_empty() {
        err_response(res, HttpError::bad_request("Missing command path", None));
        return;
    }
    dispatch_route(&rest, &session, req, res).await;
}

pub fn session_router() -> Router {
    Router::with_path("{session}")
        .push(Router::with_path("execute").post(session_execute))
        .push(Router::with_path("status").get(session_status))
        .push(Router::with_path("batch").post(batch))
        .push(
            Router::with_path("{**rest}")
                .get(session_command)
                .post(session_command)
                .delete(session_command),
        )
}
