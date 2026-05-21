use salvo::http::HeaderValue;
use salvo::prelude::*;
use salvo::oapi::swagger_ui::SwaggerUi;
use std::net::SocketAddr;
use std::time::Instant;

use crate::serve::openapi_paths::enrich_session_command_paths;
use crate::serve::routes::{meta_router, session_router};
use crate::serve::ServerConfig;

static STARTED: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

struct CorsMiddleware {
    origins: Vec<String>,
}

#[handler]
impl CorsMiddleware {
    async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
        let allow_origin = if self.origins.iter().any(|o| o == "*") {
            "*".to_string()
        } else if let Some(origin) = req.headers().get("origin").and_then(|v| v.to_str().ok()) {
            if self.origins.iter().any(|o| o == origin) {
                origin.to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if !allow_origin.is_empty() {
            if let Ok(v) = HeaderValue::from_str(&allow_origin) {
                res.headers_mut().insert("Access-Control-Allow-Origin", v);
            }
            res.headers_mut().insert(
                "Access-Control-Allow-Methods",
                HeaderValue::from_static("GET, POST, PUT, PATCH, DELETE, OPTIONS"),
            );
            res.headers_mut().insert(
                "Access-Control-Allow-Headers",
                HeaderValue::from_static("Content-Type, Authorization"),
            );
            if allow_origin != "*" {
                res.headers_mut()
                    .insert("Vary", HeaderValue::from_static("Origin"));
            }
        }

        if req.method() == salvo::http::Method::OPTIONS {
            res.status_code(salvo::http::StatusCode::NO_CONTENT);
            ctrl.skip_rest();
            return;
        }

        ctrl.call_next(req, depot, res).await;
    }
}

fn build_api_router() -> Router {
    Router::new()
        .push(Router::with_path("api").push(meta_router()).push(
            Router::with_path("sessions").push(session_router()),
        ))
}

pub async fn run_server(config: ServerConfig) {
    let _ = STARTED.set(Instant::now());

    let api_router = build_api_router();
    let doc = enrich_session_command_paths(
        salvo::oapi::OpenApi::new("agent-browser API", env!("CARGO_PKG_VERSION"))
            .merge_router(&api_router),
    );

    let cors = CorsMiddleware {
        origins: config.cors_origins.clone(),
    };

    let router = Router::new()
        .hoop(cors)
        .push(api_router)
        .unshift(doc.into_router("/openapi.json"))
        .unshift(SwaggerUi::new("/openapi.json").into_router("/swagger-ui/"));

    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .expect("invalid serve host/port");
    eprintln!(
        "agent-browser serve listening on http://{} (swagger-ui: http://{}/swagger-ui/)",
        addr, addr
    );

    let acceptor = TcpListener::new(addr).bind().await;
    Server::new(acceptor).serve(router).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use salvo::routing::filters::try_path;

    /// Salvo 0.93 path params use `{name}` and `{**rest}`; `<name>` and `{*rest}` do not match.
    const ROUTER_PATH_SEGMENTS: &[&str] = &[
        "{session}",
        "{**rest}",
        "execute",
        "status",
        "batch",
        "health",
        "sessions",
        "skills/{name}",
    ];

    #[test]
    fn serve_router_paths_use_salvo_syntax() {
        for segment in ROUTER_PATH_SEGMENTS {
            try_path(*segment)
                .unwrap_or_else(|e| panic!("invalid Salvo path segment {segment:?}: {e}"));
            assert!(
                !segment.contains('<') && !segment.contains('>'),
                "angle-bracket paths are not Salvo params: {segment}"
            );
            if segment.contains('*') {
                assert!(
                    segment.contains("{**"),
                    "wildcard segments must use {{**name}}: {segment}"
                );
            }
        }
    }

    #[test]
    fn openapi_builds() {
        let api_router = build_api_router();
        let doc = enrich_session_command_paths(
            salvo::oapi::OpenApi::new("agent-browser API", "test").merge_router(&api_router),
        );
        assert!(
            doc.paths.len() >= 130,
            "expected broad CLI coverage in OpenAPI, got {} paths",
            doc.paths.len()
        );
        assert!(
            doc.paths.contains_key("/api/sessions/{session}/snapshot"),
            "typed GET snapshot path missing from OpenAPI"
        );
        use salvo::oapi::PathItemType;
        let snapshot = &doc.paths["/api/sessions/{session}/snapshot"];
        assert!(
            snapshot.operations.0.contains_key(&PathItemType::Get),
            "snapshot should be documented as GET"
        );
    }
}
