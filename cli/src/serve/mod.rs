//! HTTP API server (`agent-browser serve`).

mod flags_http;
mod http_dispatch;
mod openapi;
mod openapi_paths;
mod routes;
mod server;
pub mod sse;

use std::env;

pub use server::run_server;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 6578;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    /// Allowed CORS origins. Empty means use `*`.
    pub cors_origins: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
            cors_origins: vec!["*".to_string()],
        }
    }
}

impl ServerConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();
        if let Ok(host) = env::var("AGENT_BROWSER_SERVE_HOST") {
            if !host.is_empty() {
                config.host = host;
            }
        }
        if let Ok(port) = env::var("AGENT_BROWSER_SERVE_PORT") {
            if let Ok(p) = port.parse() {
                config.port = p;
            }
        }
        if let Ok(cors) = env::var("AGENT_BROWSER_SERVE_CORS") {
            if cors.is_empty() {
                config.cors_origins = vec!["*".to_string()];
            } else {
                config.cors_origins = cors
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
        config
    }
}

pub fn parse_serve_args(args: &[String]) -> Result<ServerConfig, String> {
    let mut config = ServerConfig::default();
    let mut cors_explicit = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--host" => {
                i += 1;
                config.host = args
                    .get(i)
                    .ok_or_else(|| "--host requires a value".to_string())?
                    .clone();
            }
            "--port" => {
                i += 1;
                config.port = args
                    .get(i)
                    .ok_or_else(|| "--port requires a value".to_string())?
                    .parse()
                    .map_err(|_| "Invalid --port value".to_string())?;
            }
            "--cors" => {
                cors_explicit = true;
                i += 1;
                let origin = args
                    .get(i)
                    .ok_or_else(|| "--cors requires an origin value".to_string())?
                    .clone();
                if config.cors_origins == vec!["*".to_string()] && config.cors_origins.len() == 1 {
                    config.cors_origins.clear();
                }
                config.cors_origins.push(origin);
            }
            other => return Err(format!("Unknown serve argument: {}", other)),
        }
        i += 1;
    }
    if cors_explicit && config.cors_origins.is_empty() {
        return Err("--cors requires an origin value".to_string());
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_serve_args_defaults() {
        let config = parse_serve_args(&[]).unwrap();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 6578);
        assert_eq!(config.cors_origins, vec!["*"]);
    }

    #[test]
    fn test_parse_serve_args_cors_multiple() {
        let args = vec![
            "--host".into(),
            "0.0.0.0".into(),
            "--port".into(),
            "9000".into(),
            "--cors".into(),
            "http://localhost:3000".into(),
            "--cors".into(),
            "https://app.example.com".into(),
        ];
        let config = parse_serve_args(&args).unwrap();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 9000);
        assert_eq!(config.cors_origins.len(), 2);
    }
}
