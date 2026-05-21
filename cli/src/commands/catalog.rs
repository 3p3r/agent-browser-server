//! Command path catalog shared by CLI parsing and HTTP serve mode.
//!
//! `SESSION_HTTP_PATHS` drives OpenAPI documentation for serve mode. Subcommand
//! constants are the source of truth for `UnknownSubcommand` errors in parsers.

macro_rules! session_http_paths {
    ($($path:literal),* $(,)?) => {
        pub const SESSION_HTTP_PATHS: &[&str] = &[$($path),*];
    };
}

session_http_paths! {
    "back", "forward", "reload", "open", "goto", "navigate",
    "click", "dblclick", "fill", "type", "hover", "focus", "check", "uncheck", "select",
    "drag", "upload", "download",
    "press", "key", "keydown", "keyup",
    "keyboard/type", "keyboard/inserttext",
    "scroll", "scrollintoview", "scrollinto",
    "wait",
    "screenshot", "pdf", "snapshot", "eval",
    "close", "quit", "exit", "inspect",
    "confirm", "deny", "connect",
    "console", "errors", "highlight",
    "tap", "swipe",
    "vitals", "web-vitals", "pushstate", "removeinitscript",
    "auth/save", "auth/login", "auth/list", "auth/delete", "auth/remove", "auth/show",
    "stream/enable", "stream/disable", "stream/status",
    "get/text", "get/html", "get/value", "get/attr", "get/url", "get/title", "get/count",
    "get/box", "get/styles", "get/cdp-url",
    "is/visible", "is/enabled", "is/checked",
    "find/role", "find/text", "find/label", "find/placeholder", "find/alt", "find/title",
    "find/testid", "find/first", "find/last", "find/nth",
    "mouse/move", "mouse/down", "mouse/up", "mouse/wheel",
    "set/viewport", "set/device", "set/geo", "set/geolocation", "set/offline", "set/headers",
    "set/credentials", "set/auth", "set/media",
    "network/route", "network/unroute", "network/requests", "network/request",
    "network/har/start", "network/har/stop",
    "storage/local", "storage/session",
    "storage/local/get", "storage/local/set", "storage/local/clear",
    "storage/session/get", "storage/session/set", "storage/session/clear",
    "cookies", "cookies/get", "cookies/set", "cookies/clear",
    "tab", "tab/new", "tab/list", "tab/close",
    "window/new",
    "frame/main",
    "dialog/accept", "dialog/dismiss", "dialog/status",
    "trace/start", "trace/stop",
    "profiler/start", "profiler/stop",
    "record/start", "record/stop", "record/restart",
    "clipboard", "clipboard/read", "clipboard/write", "clipboard/copy", "clipboard/paste",
    "state/save", "state/load", "state/list", "state/clear", "state/show", "state/clean",
    "state/rename",
    "device", "device/list",
    "diff/snapshot", "diff/screenshot", "diff/url",
    "react/tree", "react/inspect", "react/renders/start", "react/renders/stop", "react/suspense",
}

pub const AUTH: &[&str] = &["save", "login", "list", "delete", "remove", "show"];
pub const STREAM: &[&str] = &["enable", "disable", "status"];
pub const GET: &[&str] = &[
    "text", "html", "value", "attr", "url", "title", "count", "box", "styles", "cdp-url",
];
pub const IS: &[&str] = &["visible", "enabled", "checked"];
pub const FIND: &[&str] = &[
    "role", "text", "label", "placeholder", "alt", "title", "testid", "first", "last", "nth",
];
pub const MOUSE: &[&str] = &["move", "down", "up", "wheel"];
pub const SET: &[&str] = &[
    "viewport", "device", "geo", "geolocation", "offline", "headers", "credentials", "auth",
    "media",
];
pub const NETWORK: &[&str] = &["route", "unroute", "requests", "request", "har"];
pub const NETWORK_HAR: &[&str] = &["start", "stop"];
pub const STORAGE_TYPES: &[&str] = &["local", "session"];
pub const STORAGE_OPS: &[&str] = &["get", "set", "clear"];
pub const DIALOG: &[&str] = &["accept", "dismiss", "status"];
pub const TRACE: &[&str] = &["start", "stop"];
pub const PROFILER: &[&str] = &["start", "stop"];
pub const RECORD: &[&str] = &["start", "stop", "restart"];
pub const CLIPBOARD: &[&str] = &["read", "write", "copy", "paste"];
pub const STATE: &[&str] = &["save", "load", "list", "clear", "show", "clean", "rename"];
pub const REACT: &[&str] = &["tree", "inspect", "renders", "suspense"];
pub const REACT_RENDERS: &[&str] = &["start", "stop"];
pub const DIFF: &[&str] = &["snapshot", "screenshot", "url"];
pub const KEYBOARD: &[&str] = &["type", "inserttext", "insertText"];
pub const WINDOW: &[&str] = &["new"];
pub const DEVICE: &[&str] = &["list"];
pub const TAB: &[&str] = &["new", "list", "close"];
pub const COOKIES: &[&str] = &["get", "set", "clear"];

/// OpenAPI tag for a session command path (first path segment, with small overrides).
pub fn openapi_tag(path: &str) -> &'static str {
    match path.split('/').next().unwrap_or("") {
        "get" | "is" => "query",
        "set" => "settings",
        "web-vitals" => "performance",
        "keyboard" => "keyboard",
        "find" => "find",
        "mouse" => "mouse",
        "network" => "network",
        "storage" => "storage",
        "cookies" => "cookies",
        "tab" | "window" => "tabs",
        "frame" => "frames",
        "dialog" | "confirm" | "deny" => "dialog",
        "trace" | "profiler" | "record" | "console" | "errors" | "highlight" => "debug",
        "clipboard" => "clipboard",
        "state" => "state",
        "tap" | "swipe" | "device" => "mobile",
        "diff" => "diff",
        "react" => "react",
        "vitals" => "performance",
        "auth" => "auth",
        "stream" => "stream",
        "scroll" | "scrollintoview" | "scrollinto" => "scrolling",
        "wait" => "timing",
        "screenshot" | "pdf" | "snapshot" => "capture",
        "eval" | "removeinitscript" => "script",
        "press" | "key" | "keydown" | "keyup" => "keyboard",
        "click" | "dblclick" | "fill" | "type" | "hover" | "focus" | "check" | "uncheck"
        | "select" | "drag" | "upload" | "download" => "interaction",
        "back" | "forward" | "reload" | "open" | "goto" | "navigate" | "pushstate" => {
            "navigation"
        }
        "close" | "quit" | "exit" | "inspect" => "lifecycle",
        "connect" => "connection",
        _ => "sessions",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_http_paths_count() {
        assert!(SESSION_HTTP_PATHS.len() >= 130);
    }

    #[test]
    fn session_http_paths_include_click_and_get_text() {
        assert!(SESSION_HTTP_PATHS.contains(&"click"));
        assert!(SESSION_HTTP_PATHS.contains(&"get/text"));
        assert!(SESSION_HTTP_PATHS.contains(&"network/har/start"));
    }
}
