//! Build CLI argv from typed HTTP route specs.

use salvo::prelude::*;
use serde_json::Value;

use crate::commands::http_routes::{find_route, ArgIn, ArgSpec, HttpMethod};
use crate::serve::flags_http::query_pairs_from_req;
use crate::serve::routes::common::{run_cli_argv, HttpError};

const ROUTE_ARG_KEYS: &[&str] = &[
    "selector",
    "text",
    "url",
    "value",
    "attribute",
    "source",
    "target",
    "files",
    "values",
    "key",
    "path",
    "name",
    "script",
    "key",
    "confirmationId",
    "endpoint",
    "role",
    "action",
    "label",
    "placeholder",
    "testId",
    "alt",
    "title",
    "button",
    "x",
    "y",
    "deltaX",
    "deltaY",
    "width",
    "height",
    "scale",
    "device",
    "latitude",
    "longitude",
    "headers",
    "username",
    "password",
    "requestId",
    "tabId",
    "url2",
    "fiberId",
    "direction",
    "distance",
    "filename",
    "oldName",
    "newName",
    "identifier",
    "olderThanDays",
    "promptText",
    "headers",
    "colorScheme",
    "reducedMotion",
    "baseline",
    "output",
    "threshold",
    "categories",
    "index",
];

fn is_route_key(key: &str) -> bool {
    ROUTE_ARG_KEYS.contains(&key)
        || key.ends_with("Selector")
        || matches!(
            key,
            "newTab"
                | "interactive"
                | "compact"
                | "cursor"
                | "urls"
                | "base64"
                | "stdin"
                | "exact"
                | "abort"
                | "clear"
                | "all"
                | "offline"
                | "httpOnly"
                | "secure"
                | "passwordStdin"
                | "json"
                | "onlyDynamic"
                | "screenshot"
                | "fullPage"
                | "full"
        )
        || key.contains("flag")
        || crate::flags::http_flag_name(key).is_some()
}

fn query_val(req: &Request, name: &str) -> Option<String> {
    req.queries().get(name).map(|s| s.to_string())
}

fn body_val(body: &Value, name: &str) -> Option<String> {
    body.get(name).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    })
}

fn body_bool(body: &Value, name: &str) -> Option<bool> {
    body.get(name).and_then(|v| v.as_bool())
}

fn body_string_list(body: &Value, name: &str) -> Option<Vec<String>> {
    body.get(name).and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|x| x.as_str().map(|s| s.to_string()))
            .collect()
    })
}

fn read_arg(
    spec: ArgSpec,
    method: HttpMethod,
    req: &Request,
    body: &Value,
) -> Result<ArgValue, HttpError> {
    let loc = spec.resolve_location(method);
    let name = spec.openapi_name();
    Ok(match spec {
        ArgSpec::Selector { .. } | ArgSpec::Text { .. } | ArgSpec::Url { .. } => {
            let v = match loc {
                ArgIn::Query => query_val(req, name),
                ArgIn::Body => body_val(body, name),
                ArgIn::Auto => unreachable!(),
            };
            ArgValue::String(v)
        }
        ArgSpec::StringArg { .. } | ArgSpec::IntArg { .. } | ArgSpec::FloatArg { .. } => {
            let v = match loc {
                ArgIn::Query => query_val(req, name),
                ArgIn::Body => body_val(body, name),
                ArgIn::Auto => unreachable!(),
            };
            ArgValue::String(v)
        }
        ArgSpec::BoolFlag { .. } => {
            let b = query_val(req, name)
                .map(|s| s == "true" || s == "1")
                .or_else(|| body_bool(body, name))
                .unwrap_or(false);
            ArgValue::Bool(b)
        }
        ArgSpec::StringFlag { .. } => {
            let v = query_val(req, name).or_else(|| body_val(body, name));
            ArgValue::String(v)
        }
        ArgSpec::StringList { .. } => {
            let list = body_string_list(body, name).unwrap_or_default();
            ArgValue::Strings(list)
        }
    })
}

enum ArgValue {
    String(Option<String>),
    Bool(bool),
    Strings(Vec<String>),
}

fn push_cli_flag(argv: &mut Vec<String>, flag: &str, value: Option<&str>) {
    let cli = if flag == "interactive" {
        "-i"
    } else if flag == "compact" {
        "-c"
    } else if flag == "cursor" {
        "-C"
    } else if flag == "urls" {
        "-u"
    } else if flag == "full" || flag == "fullPage" {
        "-f"
    } else if flag == "base64" {
        "-b"
    } else if flag == "stdin" {
        "--stdin"
    } else if flag == "password-stdin" {
        "--password-stdin"
    } else if flag == "only-dynamic" {
        "--only-dynamic"
    } else if flag == "older-than" {
        "--older-than"
    } else {
        flag
    };
    if cli.starts_with('-') {
        argv.push(cli.to_string());
        if let Some(v) = value {
            argv.push(v.to_string());
        }
    } else {
        argv.push(format!("--{}", cli));
        if let Some(v) = value {
            argv.push(v.to_string());
        }
    }
}

pub fn build_argv_from_route(
    route: &crate::commands::http_routes::HttpRouteSpec,
    req: &Request,
    body: &Value,
) -> Result<Vec<String>, HttpError> {
    let mut argv: Vec<String> = route.path.split('/').map(|s| s.to_string()).collect();

    for spec in route.args {
        let val = read_arg(*spec, route.method, req, body)?;
        match (spec, val) {
            (ArgSpec::Selector { required, .. }, ArgValue::String(v))
            | (ArgSpec::Text { required, .. }, ArgValue::String(v))
            | (ArgSpec::Url { required, .. }, ArgValue::String(v))
            | (ArgSpec::StringArg { required, .. }, ArgValue::String(v))
            | (ArgSpec::IntArg { required, .. }, ArgValue::String(v))
            | (ArgSpec::FloatArg { required, .. }, ArgValue::String(v)) => {
                if *required && v.as_ref().is_none_or(|s| s.is_empty()) {
                    return Err(HttpError::bad_request(
                        format!("Missing required parameter: {}", spec.openapi_name()),
                        Some("missing_parameter"),
                    ));
                }
                if let Some(s) = v {
                    if !s.is_empty() {
                        argv.push(s);
                    }
                }
            }
            (ArgSpec::BoolFlag { cli_flag, .. }, ArgValue::Bool(true)) => {
                push_cli_flag(&mut argv, cli_flag, None);
            }
            (ArgSpec::BoolFlag { .. }, ArgValue::Bool(false)) => {}
            (
                ArgSpec::StringFlag {
                    cli_flag,
                    required,
                    ..
                },
                ArgValue::String(v),
            ) => {
                if *required && v.as_ref().is_none_or(|s| s.is_empty()) {
                    return Err(HttpError::bad_request(
                        format!("Missing required parameter: {}", spec.openapi_name()),
                        Some("missing_parameter"),
                    ));
                }
                if let Some(ref s) = v {
                    if !s.is_empty() {
                        push_cli_flag(&mut argv, cli_flag, Some(s.as_str()));
                    }
                }
            }
            (ArgSpec::StringList { required, .. }, ArgValue::Strings(list)) => {
                if *required && list.is_empty() {
                    return Err(HttpError::bad_request(
                        format!("Missing required parameter: {}", spec.openapi_name()),
                        Some("missing_parameter"),
                    ));
                }
                argv.extend(list);
            }
            _ => {}
        }
    }

    Ok(argv)
}

pub fn filter_global_query_pairs(pairs: &[(String, String)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .filter(|(k, _)| !is_route_key(k))
        .cloned()
        .collect()
}

pub async fn dispatch_route(
    path: &str,
    session: &str,
    req: &mut Request,
    res: &mut Response,
) {
    let method = req.method().clone();
    let body_json: Value = if method == salvo::http::Method::GET || method == salvo::http::Method::DELETE
    {
        Value::Object(Default::default())
    } else {
        req.parse_json()
            .await
            .unwrap_or(Value::Object(Default::default()))
    };

    let route = match find_route(path, &method) {
        Some(r) => r,
        None => {
            return crate::serve::routes::common::dispatch_cli_path(path, session, &body_json, req, res)
                .await;
        }
    };

    if let Err(e) = crate::serve::routes::common::session_from_path(session) {
        return crate::serve::routes::common::err_response(res, e);
    }

    let argv = match build_argv_from_route(route, req, &body_json) {
        Ok(a) => a,
        Err(e) => return crate::serve::routes::common::err_response(res, e),
    };

    let all_pairs = query_pairs_from_req(req);
    let global_pairs: Vec<(String, String)> = filter_global_query_pairs(&all_pairs);
    let pair_refs: Vec<(&str, &str)> = global_pairs
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    match run_cli_argv(session, argv, &pair_refs, None).await {
        Ok(api) => crate::serve::routes::common::ok_api(res, api),
        Err(e) => crate::serve::routes::common::err_response(res, e),
    }
}
