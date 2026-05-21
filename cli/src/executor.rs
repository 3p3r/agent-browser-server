//! Shared command execution for CLI and HTTP serve mode.

use serde_json::{json, Value};

use crate::commands::{gen_id, parse_command, ParseError};
use crate::connection::{
    self, command_read_timeout, ensure_daemon, send_command_async, send_command_with_flags,
    DaemonOptions, DaemonResult, Response,
};
use crate::flags::Flags;
use crate::native;

#[derive(Debug, Clone)]
pub struct ParsedProxy {
    pub server: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

pub fn parse_proxy(proxy_str: &str) -> ParsedProxy {
    let Some(protocol_end) = proxy_str.find("://") else {
        return ParsedProxy {
            server: proxy_str.to_string(),
            username: None,
            password: None,
        };
    };
    let protocol = &proxy_str[..protocol_end + 3];
    let rest = &proxy_str[protocol_end + 3..];

    let Some(at_pos) = rest.rfind('@') else {
        return ParsedProxy {
            server: proxy_str.to_string(),
            username: None,
            password: None,
        };
    };

    let creds = &rest[..at_pos];
    let server_part = &rest[at_pos + 1..];
    let server = format!("{}{}", protocol, server_part);

    let (username, password) = match creds.find(':') {
        Some(colon_pos) => {
            let u = &creds[..colon_pos];
            let p = &creds[colon_pos + 1..];
            (
                if u.is_empty() {
                    None
                } else {
                    Some(u.to_string())
                },
                if p.is_empty() {
                    None
                } else {
                    Some(p.to_string())
                },
            )
        }
        None => (
            if creds.is_empty() {
                None
            } else {
                Some(creds.to_string())
            },
            None,
        ),
    };

    ParsedProxy {
        server,
        username,
        password,
    }
}

pub struct ExecutorContext<'a> {
    pub flags: &'a Flags,
}

#[derive(Debug)]
pub enum ExecutorError {
    Parse(ParseError),
    Message(String),
    MutexOptions(String),
}

impl ExecutorError {
    pub fn message(s: impl Into<String>) -> Self {
        ExecutorError::Message(s.into())
    }
}

pub struct BatchItemResult {
    pub index: usize,
    pub command: Vec<String>,
    pub action: Option<String>,
    pub response: Option<Response>,
    pub parse_error: Option<String>,
    pub transport_error: Option<String>,
}

pub enum ExecutorOutcome {
    Response(Response),
    Batch(Vec<BatchItemResult>),
}

pub fn state_response(cmd: &Value) -> Option<Response> {
    let result = native::state::dispatch_state_command(cmd)?;
    Some(match result {
        Ok(data) => Response {
            success: true,
            data: Some(data),
            error: None,
            warning: None,
        },
        Err(e) => Response {
            success: false,
            data: None,
            error: Some(e),
            warning: None,
        },
    })
}

pub fn ensure_daemon_for_flags(flags: &Flags) -> Result<DaemonResult, String> {
    let (proxy_server, proxy_username, proxy_password) = if let Some(ref proxy_str) = flags.proxy {
        let parsed = parse_proxy(proxy_str);
        (Some(parsed.server), parsed.username, parsed.password)
    } else {
        (None, None, None)
    };

    let daemon_opts = DaemonOptions {
        headed: flags.headed,
        debug: flags.debug,
        executable_path: flags.executable_path.as_deref(),
        extensions: &flags.extensions,
        init_scripts: &flags.init_scripts,
        enable: &flags.enable,
        args: flags.args.as_deref(),
        user_agent: flags.user_agent.as_deref(),
        proxy: proxy_server.as_deref(),
        proxy_bypass: flags.proxy_bypass.as_deref(),
        proxy_username: proxy_username.as_deref(),
        proxy_password: proxy_password.as_deref(),
        ignore_https_errors: flags.ignore_https_errors,
        allow_file_access: flags.allow_file_access,
        profile: flags.profile.as_deref(),
        state: flags.state.as_deref(),
        provider: flags.provider.as_deref(),
        device: flags.device.as_deref(),
        session_name: flags.session_name.as_deref(),
        download_path: flags.download_path.as_deref(),
        allowed_domains: flags.allowed_domains.as_deref(),
        action_policy: flags.action_policy.as_deref(),
        confirm_actions: flags.confirm_actions.as_deref(),
        engine: flags.engine.as_deref(),
        auto_connect: flags.auto_connect,
        idle_timeout: flags.idle_timeout.as_deref(),
        default_timeout: flags.default_timeout,
        cdp: flags.cdp.as_deref(),
        no_auto_dialog: flags.no_auto_dialog,
    };

    ensure_daemon(&flags.session, &daemon_opts)
}

pub fn validate_mutex_options(flags: &Flags) -> Result<(), ExecutorError> {
    if flags.cdp.is_some() && flags.provider.is_some() {
        return Err(ExecutorError::MutexOptions(
            "Cannot use --cdp and -p/--provider together".into(),
        ));
    }
    if flags.auto_connect && flags.cdp.is_some() {
        return Err(ExecutorError::MutexOptions(
            "Cannot use --auto-connect and --cdp together".into(),
        ));
    }
    if flags.auto_connect && flags.provider.is_some() {
        return Err(ExecutorError::MutexOptions(
            "Cannot use --auto-connect and -p/--provider together".into(),
        ));
    }
    if flags.provider.is_some() && !flags.extensions.is_empty() {
        return Err(ExecutorError::MutexOptions(
            "Cannot use --extension with -p/--provider (extensions require local browser)".into(),
        ));
    }
    if flags.cdp.is_some() && !flags.extensions.is_empty() {
        return Err(ExecutorError::MutexOptions(
            "Cannot use --extension with --cdp (extensions require local browser)".into(),
        ));
    }
    Ok(())
}

fn send_sync(cmd: Value, session: &str, flags: &Flags) -> Result<Response, String> {
    send_command_with_flags(cmd, session, flags)
}

async fn send_async(cmd: Value, session: &str, flags: &Flags) -> Result<Response, String> {
    let timeout = command_read_timeout(&cmd, flags);
    send_command_async(cmd, session, timeout).await
}

async fn run_prelaunch(
    flags: &Flags,
    daemon_result: &DaemonResult,
) -> Result<(), ExecutorError> {
    let session = &flags.session;

    if flags.auto_connect && !daemon_result.already_running {
        let mut launch_cmd = json!({
            "id": gen_id(),
            "action": "launch",
            "autoConnect": true
        });
        if flags.ignore_https_errors {
            launch_cmd["ignoreHTTPSErrors"] = json!(true);
        }
        if let Some(ref cs) = flags.color_scheme {
            launch_cmd["colorScheme"] = json!(cs);
        }
        if let Some(ref dp) = flags.download_path {
            launch_cmd["downloadPath"] = json!(dp);
        }
        match send_async(launch_cmd, session, flags).await {
            Ok(resp) if resp.success => {}
            Ok(resp) => {
                return Err(ExecutorError::message(
                    resp.error
                        .unwrap_or_else(|| "Auto-connect failed".to_string()),
                ));
            }
            Err(e) => return Err(ExecutorError::message(e)),
        }
    }

    if let Some(ref cdp_value) = flags.cdp {
        let launch_cmd = if cdp_value.starts_with("ws://")
            || cdp_value.starts_with("wss://")
            || cdp_value.starts_with("http://")
            || cdp_value.starts_with("https://")
        {
            json!({
                "id": gen_id(),
                "action": "launch",
                "cdpUrl": cdp_value
            })
        } else {
            let cdp_port: u16 = match cdp_value.parse::<u32>() {
                Ok(0) => {
                    return Err(ExecutorError::message(
                        "Invalid CDP port: port must be greater than 0",
                    ));
                }
                Ok(p) if p > 65535 => {
                    return Err(ExecutorError::message(format!(
                        "Invalid CDP port: {} is out of range (valid range: 1-65535)",
                        p
                    )));
                }
                Ok(p) => p as u16,
                Err(_) => {
                    return Err(ExecutorError::message(format!(
                        "Invalid CDP value: '{}' is not a valid port number or URL",
                        cdp_value
                    )));
                }
            };
            json!({
                "id": gen_id(),
                "action": "launch",
                "cdpPort": cdp_port
            })
        };

        if !daemon_result.already_running {
            let mut launch_cmd = launch_cmd;
            if flags.ignore_https_errors {
                launch_cmd["ignoreHTTPSErrors"] = json!(true);
            }
            if let Some(ref cs) = flags.color_scheme {
                launch_cmd["colorScheme"] = json!(cs);
            }
            if let Some(ref dp) = flags.download_path {
                launch_cmd["downloadPath"] = json!(dp);
            }
            match send_async(launch_cmd, session, flags).await {
                Ok(resp) if resp.success => {}
                Ok(resp) => {
                    return Err(ExecutorError::message(
                        resp.error
                            .unwrap_or_else(|| "CDP connection failed".to_string()),
                    ));
                }
                Err(e) => return Err(ExecutorError::message(e)),
            }
        }
    }

    if let Some(ref provider) = flags.provider {
        if !daemon_result.already_running {
            let mut launch_cmd = json!({
                "id": gen_id(),
                "action": "launch",
                "provider": provider
            });
            if let Some(ref cs) = flags.color_scheme {
                launch_cmd["colorScheme"] = json!(cs);
            }
            match send_async(launch_cmd, session, flags).await {
                Ok(resp) if resp.success => {}
                Ok(resp) => {
                    return Err(ExecutorError::message(
                        resp.error
                            .unwrap_or_else(|| "Provider connection failed".to_string()),
                    ));
                }
                Err(e) => return Err(ExecutorError::message(e)),
            }
        }
    }

    if (flags.headed
        || flags.cli_headed
        || flags.executable_path.is_some()
        || flags.profile.is_some()
        || flags.state.is_some()
        || flags.proxy.is_some()
        || flags.args.is_some()
        || flags.user_agent.is_some()
        || flags.allow_file_access
        || flags.color_scheme.is_some()
        || flags.download_path.is_some()
        || flags.engine.is_some()
        || !flags.extensions.is_empty())
        && flags.cdp.is_none()
        && flags.provider.is_none()
        && !flags.auto_connect
    {
        let mut launch_cmd = json!({
            "id": gen_id(),
            "action": "launch",
            "headless": !flags.headed
        });
        let cmd_obj = launch_cmd
            .as_object_mut()
            .expect("json object");

        if let Some(ref exec_path) = flags.executable_path {
            cmd_obj.insert("executablePath".to_string(), json!(exec_path));
        }
        if let Some(ref profile_path) = flags.profile {
            cmd_obj.insert("profile".to_string(), json!(profile_path));
        }
        if let Some(ref state_path) = flags.state {
            cmd_obj.insert("storageState".to_string(), json!(state_path));
        }
        if let Some(ref proxy_str) = flags.proxy {
            let parsed = parse_proxy(proxy_str);
            let mut proxy_obj = json!({ "server": parsed.server });
            if let Some(ref username) = parsed.username {
                proxy_obj["username"] = json!(username);
            }
            if let Some(ref password) = parsed.password {
                proxy_obj["password"] = json!(password);
            }
            if let Some(ref bypass) = flags.proxy_bypass {
                proxy_obj["bypass"] = json!(bypass);
            }
            cmd_obj.insert("proxy".to_string(), proxy_obj);
        }
        if let Some(ref ua) = flags.user_agent {
            cmd_obj.insert("userAgent".to_string(), json!(ua));
        }
        if let Some(ref a) = flags.args {
            let args_vec: Vec<String> = a
                .split(&[',', '\n'][..])
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            cmd_obj.insert("args".to_string(), json!(args_vec));
        }
        if !flags.extensions.is_empty() {
            cmd_obj.insert("extensions".to_string(), json!(&flags.extensions));
        }
        if flags.ignore_https_errors {
            launch_cmd["ignoreHTTPSErrors"] = json!(true);
        }
        if flags.allow_file_access {
            launch_cmd["allowFileAccess"] = json!(true);
        }
        if let Some(ref cs) = flags.color_scheme {
            launch_cmd["colorScheme"] = json!(cs);
        }
        if let Some(ref dp) = flags.download_path {
            launch_cmd["downloadPath"] = json!(dp);
        }
        if let Some(ref domains) = flags.allowed_domains {
            launch_cmd["allowedDomains"] = json!(domains);
        }
        if let Some(ref engine) = flags.engine {
            launch_cmd["engine"] = json!(engine);
        }

        match send_async(launch_cmd, session, flags).await {
            Ok(resp) if !resp.success => {
                return Err(ExecutorError::message(
                    resp.error
                        .unwrap_or_else(|| "Browser launch failed".to_string()),
                ));
            }
            Err(e) => return Err(ExecutorError::message(e)),
            Ok(_) => {}
        }
    }

    Ok(())
}

pub async fn execute_command_value(
    cmd: Value,
    ctx: &ExecutorContext<'_>,
) -> Result<ExecutorOutcome, ExecutorError> {
    let flags = &ctx.flags;
    let session = &flags.session;

    if let Some(resp) = state_response(&cmd) {
        return Ok(ExecutorOutcome::Response(resp));
    }

    validate_mutex_options(flags)?;

    let daemon_result = ensure_daemon_for_flags(flags).map_err(ExecutorError::message)?;

    run_prelaunch(flags, &daemon_result).await?;

    if cmd.get("action").and_then(|v| v.as_str()) == Some("batch") {
        let bail = cmd.get("bail").and_then(|v| v.as_bool()).unwrap_or(false);
        let commands: Vec<Vec<String>> = cmd
            .get("commands")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        if let Some(s) = v.as_str() {
                            Some(crate::commands::shell_words_split(s))
                        } else if let Some(inner) = v.as_array() {
                            Some(
                                inner
                                    .iter()
                                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                    .collect(),
                            )
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        return Ok(ExecutorOutcome::Batch(
            run_batch_commands(commands, flags, bail).await,
        ));
    }

    let resp = send_async(cmd, session, flags)
        .await
        .map_err(ExecutorError::message)?;
    Ok(ExecutorOutcome::Response(resp))
}

pub async fn run_batch_commands(
    commands: Vec<Vec<String>>,
    flags: &Flags,
    bail: bool,
) -> Vec<BatchItemResult> {
    let mut results = Vec::new();

    for (i, cmd_args) in commands.iter().enumerate() {
        if cmd_args.is_empty() {
            continue;
        }

        let parsed = match parse_command(cmd_args, flags) {
            Ok(c) => c,
            Err(e) => {
                results.push(BatchItemResult {
                    index: i,
                    command: cmd_args.clone(),
                    action: None,
                    response: None,
                    parse_error: Some(e.format()),
                    transport_error: None,
                });
                if bail {
                    break;
                }
                continue;
            }
        };

        let action = parsed
            .get("action")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        match send_async(parsed, &flags.session, flags).await {
            Ok(resp) => {
                let failed = !resp.success;
                results.push(BatchItemResult {
                    index: i,
                    command: cmd_args.clone(),
                    action,
                    response: Some(resp),
                    parse_error: None,
                    transport_error: None,
                });
                if failed && bail {
                    break;
                }
            }
            Err(e) => {
                results.push(BatchItemResult {
                    index: i,
                    command: cmd_args.clone(),
                    action,
                    response: None,
                    parse_error: None,
                    transport_error: Some(e),
                });
                if bail {
                    break;
                }
            }
        }
    }

    results
}

/// Sync path for CLI: runs full pipeline including ensure_daemon for a parsed command.
pub fn execute_sync(cmd: Value, flags: &Flags) -> Result<Response, String> {
    if let Some(resp) = state_response(&cmd) {
        return Ok(resp);
    }

    validate_mutex_options(flags).map_err(|e| match e {
        ExecutorError::Message(m) | ExecutorError::MutexOptions(m) => m,
        ExecutorError::Parse(p) => p.format(),
    })?;

    let daemon_result = ensure_daemon_for_flags(flags)?;

    // Prelaunch using blocking send - run via a small runtime
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(run_prelaunch(flags, &daemon_result))
        .map_err(|e| match e {
            ExecutorError::Message(m) | ExecutorError::MutexOptions(m) => m,
            ExecutorError::Parse(p) => p.format(),
        })?;

    send_sync(cmd, &flags.session, flags)
}

/// Close all sessions (for HTTP DELETE /api/sessions).
pub async fn close_all_sessions() -> Value {
    use crate::connection::{cleanup_stale_files, walk_daemons};

    let inventory = walk_daemons();
    let sessions: Vec<(String, u32)> = inventory
        .sessions
        .iter()
        .map(|s| (s.name.clone(), s.pid))
        .collect();

    if sessions.is_empty() {
        return json!({
            "success": true,
            "data": { "closed": 0, "sessions": [] },
        });
    }

    let mut closed: Vec<String> = Vec::new();
    let mut failed: Vec<Value> = Vec::new();
    let flags = crate::flags::parse_flags(&[]);

    for (session, pid) in sessions {
        let cmd = json!({ "id": gen_id(), "action": "close" });
        match send_async(cmd, &session, &flags).await {
            Ok(resp) if resp.success => closed.push(session),
            Ok(resp) => {
                failed.push(json!({
                    "session": session,
                    "error": resp.error.unwrap_or_else(|| "Unknown error".to_string()),
                }));
            }
            Err(_) => {
                #[cfg(unix)]
                unsafe {
                    libc::kill(pid as i32, libc::SIGKILL);
                }
                #[cfg(windows)]
                unsafe {
                    use windows_sys::Win32::Foundation::CloseHandle;
                    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess};
                    let handle = OpenProcess(1, 0, pid);
                    if handle != 0 {
                        TerminateProcess(handle, 1);
                        CloseHandle(handle);
                    }
                }
                cleanup_stale_files(&session);
                closed.push(session);
            }
        }
    }

    json!({
        "success": failed.is_empty(),
        "data": {
            "closed": closed.len(),
            "sessions": closed,
            "failed": failed,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_proxy_with_credentials() {
        let result = parse_proxy("http://user:pass@proxy.com:8080");
        assert_eq!(result.server, "http://proxy.com:8080");
        assert_eq!(result.username.as_deref(), Some("user"));
        assert_eq!(result.password.as_deref(), Some("pass"));
    }
}
