use salvo::oapi::extract::*;
use salvo::prelude::*;
use serde_json::json;

use crate::chat;
use crate::native::stream::chat as stream_chat;
use crate::serve::openapi::ChatRequest;
use crate::serve::routes::common::{err_response, json_response, HttpError};

const DEFAULT_MODEL: &str = "anthropic/claude-sonnet-4.6";

/// AI chat (JSON response or SSE when `stream` is true).
#[endpoint(tags("streaming"))]
pub async fn chat_handler(body: JsonBody<ChatRequest>, res: &mut Response) {
    if !stream_chat::is_chat_enabled() {
        err_response(
            res,
            HttpError::bad_request(
                "AI_GATEWAY_API_KEY not set. Set the AI_GATEWAY_API_KEY environment variable to enable chat.",
                None,
            ),
        );
        return;
    }

    let session = body
        .session
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let model = body
        .model
        .clone()
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());

    if body.stream {
        err_response(
            res,
            HttpError::bad_request(
                "Streaming chat via SSE is not yet wired for serve mode; use stream: false",
                None,
            ),
        );
        return;
    }

    let result = chat::api_chat_json(&session, &model, &body.message).await;
    if result.get("success").and_then(|v| v.as_bool()) == Some(true) {
        res.status_code(StatusCode::OK);
        res.render(Json(result));
    } else {
        json_response(res, StatusCode::OK, result);
    }
}
