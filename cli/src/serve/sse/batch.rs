use std::convert::Infallible;

use salvo::prelude::*;
use salvo::sse::{self, SseEvent};
use serde_json::json;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::commands::parse_command;
use crate::connection::{command_read_timeout, send_command_async};
use crate::flags::Flags;
use crate::flags::flags_from_http;
use crate::serve::openapi::BatchRequest;
use crate::serve::routes::common::session_from_path;

pub async fn stream_batch(session: &str, body: &BatchRequest, req: &Request, res: &mut Response) {
    if let Err(e) = session_from_path(session) {
        res.status_code(e.status);
        res.render(Json(e.body));
        return;
    }

    let pairs: Vec<(String, String)> = crate::serve::flags_http::query_pairs_from_req(req);
    let pair_refs: Vec<(&str, &str)> = pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let flags = Flags {
        session: session.to_string(),
        ..flags_from_http(&pair_refs, None)
    };

    let (tx, rx) =
        tokio::sync::mpsc::unbounded_channel::<Result<SseEvent, Infallible>>();
    let session_owned = session.to_string();
    let commands = body.commands.clone();
    let bail = body.bail;

    tokio::spawn(async move {
        let _ = tx.send(Ok(SseEvent::default().name("start").text(
            json!({"total": commands.len()}).to_string(),
        )));

        let mut failed = 0usize;
        for (i, cmd_args) in commands.iter().enumerate() {
            if cmd_args.is_empty() {
                continue;
            }
            let event = match parse_command(cmd_args, &flags) {
                Ok(parsed) => {
                    let action = parsed.get("action").and_then(|v| v.as_str()).map(String::from);
                    let timeout = command_read_timeout(&parsed, &flags);
                    match send_command_async(parsed, &session_owned, timeout).await {
                        Ok(resp) => {
                            if !resp.success {
                                failed += 1;
                            }
                            json!({
                                "index": i,
                                "success": resp.success,
                                "action": action,
                                "data": resp.data,
                                "error": resp.error,
                            })
                        }
                        Err(e) => {
                            failed += 1;
                            json!({ "index": i, "success": false, "error": e })
                        }
                    }
                }
                Err(e) => {
                    failed += 1;
                    json!({ "index": i, "success": false, "error": e.format() })
                }
            };
            let ok = event.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
            let _ = tx.send(Ok(
                SseEvent::default().name("result").text(event.to_string()),
            ));
            if bail && !ok {
                break;
            }
        }

        let _ = tx.send(Ok(
            SseEvent::default().name("done").text(
                json!({"total": commands.len(), "failed": failed}).to_string(),
            ),
        ));
    });

    let stream = UnboundedReceiverStream::new(rx);
    sse::stream(res, stream);
}
