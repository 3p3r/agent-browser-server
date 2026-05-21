use salvo::http::StatusCode;
use salvo::prelude::*;
use serde_json::Value;

use crate::commands::{parse_command, ParseError};
use crate::connection::Response as DaemonResponse;
use crate::executor::{ExecutorContext, ExecutorError, ExecutorOutcome};
use crate::flags::Flags;
use crate::flags::flags_from_http;
use crate::serve::openapi::ApiResponse;

pub struct HttpError {
    pub status: StatusCode,
    pub body: Value,
}

impl HttpError {
    pub fn bad_request(message: impl Into<String>, error_type: Option<&str>) -> Self {
        let mut body = serde_json::json!({
            "success": false,
            "error": message.into(),
        });
        if let Some(t) = error_type {
            body["type"] = Value::String(t.to_string());
        }
        Self {
            status: StatusCode::BAD_REQUEST,
            body,
        }
    }

    pub fn gateway(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_GATEWAY,
            body: serde_json::json!({
                "success": false,
                "error": message.into(),
            }),
        }
    }
}

pub fn session_from_path(session: &str) -> Result<(), HttpError> {
    if crate::validation::is_valid_session_name(session) {
        Ok(())
    } else {
        Err(HttpError::bad_request(
            crate::validation::session_name_error(session),
            Some("invalid_session_name"),
        ))
    }
}

pub async fn run_cli_argv(
    session: &str,
    argv: Vec<String>,
    query_pairs: &[(&str, &str)],
    body_flags: Option<&Value>,
) -> Result<ApiResponse, HttpError> {
    let mut flags = flags_from_http(query_pairs, body_flags);
    flags.session = session.to_string();

    let cmd = match parse_command(&argv, &flags) {
        Ok(c) => c,
        Err(e) => return Err(parse_error(e)),
    };

    let ctx = ExecutorContext { flags: &flags };
    match crate::executor::execute_command_value(cmd, &ctx).await {
        Ok(ExecutorOutcome::Response(resp)) => Ok(ApiResponse::from(resp)),
        Ok(ExecutorOutcome::Batch(_)) => Err(HttpError::bad_request(
            "Use POST /api/sessions/{session}/batch for batch commands",
            None,
        )),
        Err(e) => Err(executor_error(e)),
    }
}

pub async fn run_parsed_command(flags: &Flags, cmd: Value) -> Result<ApiResponse, HttpError> {
    let ctx = ExecutorContext { flags };
    match crate::executor::execute_command_value(cmd, &ctx).await {
        Ok(ExecutorOutcome::Response(resp)) => Ok(ApiResponse::from(resp)),
        Ok(ExecutorOutcome::Batch(_)) => Err(HttpError::bad_request(
            "Use the batch endpoint for batch commands",
            None,
        )),
        Err(e) => Err(executor_error(e)),
    }
}

pub fn parse_error(e: ParseError) -> HttpError {
    let error_type = match &e {
        ParseError::UnknownCommand { .. } => Some("unknown_command"),
        ParseError::UnknownSubcommand { .. } => Some("unknown_subcommand"),
        ParseError::MissingArguments { .. } => Some("missing_arguments"),
        ParseError::InvalidValue { .. } => Some("invalid_value"),
        ParseError::InvalidSessionName { .. } => Some("invalid_session_name"),
    };
    HttpError::bad_request(e.format(), error_type)
}

pub fn executor_error(e: ExecutorError) -> HttpError {
    let msg = match e {
        ExecutorError::Parse(p) => p.format(),
        ExecutorError::Message(m) | ExecutorError::MutexOptions(m) => m,
    };
    HttpError::gateway(msg)
}

pub fn json_response(res: &mut Response, status: StatusCode, body: Value) {
    res.status_code(status);
    res.render(Json(body));
}

pub fn ok_api(res: &mut Response, api: ApiResponse) {
    res.status_code(StatusCode::OK);
    res.render(Json(api));
}

pub fn err_response(res: &mut Response, err: HttpError) {
    json_response(res, err.status, err.body);
}

/// Build CLI argv from a fixed REST path plus JSON body fields.
pub fn argv_from_path_and_body(fixed_path: &str, body: &Value) -> Vec<String> {
    let mut argv: Vec<String> = fixed_path
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    if let Some(extra) = body.get("args").and_then(|v| v.as_array()) {
        for a in extra {
            if let Some(s) = a.as_str() {
                argv.push(s.to_string());
            }
        }
        return argv;
    }

    if let Some(sel) = body.get("selector").and_then(|v| v.as_str()) {
        argv.push(sel.to_string());
    }
    if let Some(t) = body.get("text").and_then(|v| v.as_str()) {
        argv.push(t.to_string());
    }
    if let Some(u) = body.get("url").and_then(|v| v.as_str()) {
        argv.push(u.to_string());
    }
    if let Some(v) = body.get("value").and_then(|v| v.as_str()) {
        argv.push(v.to_string());
    }
    argv
}

pub async fn dispatch_cli_path(
    fixed_path: &str,
    session: &str,
    body: &Value,
    req: &mut Request,
    res: &mut Response,
) {
    if let Err(e) = session_from_path(session) {
        err_response(res, e);
        return;
    }
    let argv = argv_from_path_and_body(fixed_path, body);
    let pairs: Vec<(String, String)> = crate::serve::flags_http::query_pairs_from_req(req);
    let pair_refs: Vec<(&str, &str)> = pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let flags_body = body.get("flags");
    match run_cli_argv(session, argv, &pair_refs, flags_body).await {
        Ok(api) => ok_api(res, api),
        Err(e) => err_response(res, e),
    }
}
