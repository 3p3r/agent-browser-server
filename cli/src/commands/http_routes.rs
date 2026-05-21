//! HTTP route specifications for `agent-browser serve` (method, typed args, OpenAPI).
//!
//! Each entry maps 1:1 to a CLI command path. Args become query (GET/DELETE) or JSON body (POST).

use salvo::http::Method;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Delete,
}

impl HttpMethod {
    pub fn salvo(self) -> salvo::http::Method {
        match self {
            HttpMethod::Get => Method::GET,
            HttpMethod::Post => Method::POST,
            HttpMethod::Delete => Method::DELETE,
        }
    }

    pub fn path_item_type(self) -> salvo::oapi::PathItemType {
        use salvo::oapi::PathItemType;
        match self {
            HttpMethod::Get => PathItemType::Get,
            HttpMethod::Post => PathItemType::Post,
            HttpMethod::Delete => PathItemType::Delete,
        }
    }
}

/// Where a route argument is sent on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgIn {
    /// Query on GET/DELETE; JSON field on POST.
    Auto,
    Query,
    Body,
}

/// Typed CLI/REST argument (maps to argv position or `--flag`).
#[derive(Debug, Clone, Copy)]
pub enum ArgSpec {
    /// Element selector (`@e1` or CSS).
    Selector { required: bool, location: ArgIn },
    /// Text to type/fill/clipboard-write.
    Text { required: bool, location: ArgIn },
    /// Navigation URL.
    Url { required: bool, location: ArgIn },
    /// Generic string value (storage, cookies, attribute name, etc.).
    StringArg {
        name: &'static str,
        required: bool,
        location: ArgIn,
        description: &'static str,
    },
    /// Integer (mouse coordinates, depth, tab index, fiber id, ...).
    IntArg {
        name: &'static str,
        required: bool,
        location: ArgIn,
        description: &'static str,
    },
    /// Float (geo lat/lng, viewport scale, threshold, ...).
    FloatArg {
        name: &'static str,
        required: bool,
        location: ArgIn,
        description: &'static str,
    },
    /// Boolean CLI flag (`--new-tab` → query/body `newTab`).
    BoolFlag {
        openapi_name: &'static str,
        cli_flag: &'static str,
        description: &'static str,
    },
    /// CLI flag with string value (`--url <v>` → `url` field).
    StringFlag {
        openapi_name: &'static str,
        cli_flag: &'static str,
        required: bool,
        description: &'static str,
    },
    /// Array of strings (select values, upload files, batch not here).
    StringList {
        name: &'static str,
        required: bool,
        description: &'static str,
    },
}

impl ArgSpec {
    pub fn openapi_name(self) -> &'static str {
        match self {
            ArgSpec::Selector { .. } => "selector",
            ArgSpec::Text { .. } => "text",
            ArgSpec::Url { .. } => "url",
            ArgSpec::StringArg { name, .. } => name,
            ArgSpec::IntArg { name, .. } => name,
            ArgSpec::FloatArg { name, .. } => name,
            ArgSpec::BoolFlag { openapi_name, .. } => openapi_name,
            ArgSpec::StringFlag { openapi_name, .. } => openapi_name,
            ArgSpec::StringList { name, .. } => name,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            ArgSpec::Selector { .. } => "Element selector (@ref or CSS)",
            ArgSpec::Text { .. } => "Text content",
            ArgSpec::Url { .. } => "URL",
            ArgSpec::StringArg { description, .. }
            | ArgSpec::IntArg { description, .. }
            | ArgSpec::FloatArg { description, .. }
            | ArgSpec::BoolFlag { description, .. }
            | ArgSpec::StringFlag { description, .. }
            | ArgSpec::StringList { description, .. } => description,
        }
    }

    pub fn is_required(self) -> bool {
        match self {
            ArgSpec::Selector { required, .. }
            | ArgSpec::Text { required, .. }
            | ArgSpec::Url { required, .. }
            | ArgSpec::StringArg { required, .. }
            | ArgSpec::IntArg { required, .. }
            | ArgSpec::FloatArg { required, .. }
            | ArgSpec::StringFlag { required, .. }
            | ArgSpec::StringList { required, .. } => required,
            ArgSpec::BoolFlag { .. } => false,
        }
    }

    pub fn resolve_location(self, method: HttpMethod) -> ArgIn {
        match self {
            ArgSpec::Selector { location, .. }
            | ArgSpec::Text { location, .. }
            | ArgSpec::Url { location, .. }
            | ArgSpec::StringArg { location, .. }
            | ArgSpec::IntArg { location, .. }
            | ArgSpec::FloatArg { location, .. } => match location {
                ArgIn::Auto => {
                    if method == HttpMethod::Get || method == HttpMethod::Delete {
                        ArgIn::Query
                    } else {
                        ArgIn::Body
                    }
                }
                other => other,
            },
            ArgSpec::BoolFlag { .. } | ArgSpec::StringFlag { .. } => ArgIn::Query,
            ArgSpec::StringList { .. } => ArgIn::Body,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HttpRouteSpec {
    pub path: &'static str,
    pub method: HttpMethod,
    pub summary: &'static str,
    pub args: &'static [ArgSpec],
}

impl HttpRouteSpec {
    pub const fn new(
        method: HttpMethod,
        path: &'static str,
        summary: &'static str,
        args: &'static [ArgSpec],
    ) -> Self {
        Self {
            path,
            method,
            summary,
            args,
        }
    }
}

macro_rules! routes {
    ($($method:ident $path:literal $summary:literal $( $arg:expr ),* $(,)? );* $(;)?) => {
        pub static HTTP_ROUTES: &[HttpRouteSpec] = &[
            $( HttpRouteSpec::new(HttpMethod::$method, $path, $summary, &[$($arg),*]) ),*
        ];
    };
}

// Shorthand arg constructors
const fn sel() -> ArgSpec {
    ArgSpec::Selector {
        required: true,
        location: ArgIn::Auto,
    }
}
const fn sel_opt() -> ArgSpec {
    ArgSpec::Selector {
        required: false,
        location: ArgIn::Auto,
    }
}
const fn txt() -> ArgSpec {
    ArgSpec::Text {
        required: true,
        location: ArgIn::Auto,
    }
}
const fn url_req() -> ArgSpec {
    ArgSpec::Url {
        required: true,
        location: ArgIn::Auto,
    }
}
const fn url_opt() -> ArgSpec {
    ArgSpec::Url {
        required: false,
        location: ArgIn::Auto,
    }
}
const fn s(name: &'static str, desc: &'static str) -> ArgSpec {
    ArgSpec::StringArg {
        name,
        required: true,
        location: ArgIn::Auto,
        description: desc,
    }
}
const fn s_opt(name: &'static str, desc: &'static str) -> ArgSpec {
    ArgSpec::StringArg {
        name,
        required: false,
        location: ArgIn::Auto,
        description: desc,
    }
}
const fn i(name: &'static str, desc: &'static str) -> ArgSpec {
    ArgSpec::IntArg {
        name,
        required: true,
        location: ArgIn::Auto,
        description: desc,
    }
}
const fn f(name: &'static str, desc: &'static str) -> ArgSpec {
    ArgSpec::FloatArg {
        name,
        required: true,
        location: ArgIn::Auto,
        description: desc,
    }
}
const fn flag(name: &'static str, cli: &'static str, desc: &'static str) -> ArgSpec {
    ArgSpec::BoolFlag {
        openapi_name: name,
        cli_flag: cli,
        description: desc,
    }
}
const fn sflag(name: &'static str, cli: &'static str, req: bool, desc: &'static str) -> ArgSpec {
    ArgSpec::StringFlag {
        openapi_name: name,
        cli_flag: cli,
        required: req,
        description: desc,
    }
}
const fn list(name: &'static str, desc: &'static str) -> ArgSpec {
    ArgSpec::StringList {
        name,
        required: true,
        description: desc,
    }
}

routes! {
    // Navigation
    Post "back" "Navigate back";
    Post "forward" "Navigate forward";
    Post "reload" "Reload page";
    Post "open" "Open URL or launch browser" url_opt();
    Post "goto" "Navigate to URL" url_req();
    Post "navigate" "Navigate to URL" url_req();
    Post "pushstate" "SPA pushState" url_req();

    // Interaction
    Post "click" "Click element" sel(), flag("newTab", "new-tab", "Open in new tab");
    Post "dblclick" "Double-click" sel();
    Post "fill" "Fill input" sel(), txt();
    Post "type" "Type into element" sel(), txt();
    Post "hover" "Hover" sel();
    Post "focus" "Focus" sel();
    Post "check" "Check" sel();
    Post "uncheck" "Uncheck" sel();
    Post "select" "Select option(s)" sel(), list("values", "Values to select");
    Post "drag" "Drag and drop" s("source", "Source selector"), s("target", "Target selector");
    Post "upload" "Upload files" sel(), list("files", "File paths");
    Post "download" "Download" sel(), s("path", "Save path");

    // Keyboard
    Post "press" "Press key" s("key", "Key name");
    Post "key" "Press key" s("key", "Key name");
    Post "keydown" "Key down" s("key", "Key name");
    Post "keyup" "Key up" s("key", "Key name");
    Post "keyboard/type" "Keyboard type text" txt();
    Post "keyboard/inserttext" "Insert text" txt();

    // Scroll / wait
    Post "scroll" "Scroll" s_opt("direction", "up|down|left|right"), i("amount", "Pixels"), sel_opt();
    Post "scrollintoview" "Scroll into view" sel();
    Post "scrollinto" "Scroll into view" sel();
    Post "wait" "Wait" sel_opt(), i("timeout", "Timeout ms"),
        sflag("url", "url", false, "Wait for URL pattern"),
        sflag("load", "load", false, "Wait for load state"),
        sflag("fn", "fn", false, "Wait for JS expression"),
        sflag("text", "text", false, "Wait for text");

    // Capture
    Post "screenshot" "Screenshot" sel_opt(), s_opt("path", "Output path"), flag("fullPage", "full", "Full page");
    Post "pdf" "Save PDF" s("path", "Output path");
    Get "snapshot" "Accessibility snapshot"
        flag("interactive", "interactive", "Include interactive elements"),
        flag("compact", "compact", "Compact output"),
        flag("cursor", "cursor", "Show cursor"),
        flag("urls", "urls", "Include URLs"),
        sflag("depth", "depth", false, "Max tree depth"),
        sel_opt();
    Post "eval" "Evaluate JavaScript" s("script", "Script source"),
        flag("base64", "base64", "Script is base64"), flag("stdin", "stdin", "Read script from stdin");

    // Lifecycle
    Delete "close" "Close browser";
    Delete "quit" "Close browser";
    Delete "exit" "Close browser";
    Post "inspect" "Open DevTools";

    // Auth
    Post "auth/save" "Save auth profile" s("name", "Profile name"),
        sflag("url", "url", true, "Login URL"),
        sflag("username", "username", true, "Username"),
        sflag("password", "password", false, "Password"),
        flag("passwordStdin", "password-stdin", "Read password from stdin"),
        sflag("usernameSelector", "username-selector", false, "Username field selector"),
        sflag("passwordSelector", "password-selector", false, "Password field selector"),
        sflag("submitSelector", "submit-selector", false, "Submit button selector");
    Post "auth/login" "Login with saved profile" s("name", "Profile name");
    Get "auth/list" "List auth profiles";
    Delete "auth/delete" "Delete auth profile" s("name", "Profile name");
    Delete "auth/remove" "Delete auth profile" s("name", "Profile name");
    Get "auth/show" "Show auth profile" s("name", "Profile name");

    Post "confirm" "Confirm action" s("confirmationId", "Confirmation id");
    Post "deny" "Deny action" s("confirmationId", "Confirmation id");
    Post "connect" "Connect via CDP" s_opt("endpoint", "CDP port or WebSocket URL");

    Get "stream/status" "Stream status";
    Post "stream/enable" "Enable stream" sflag("port", "port", false, "WebSocket port");
    Post "stream/disable" "Disable stream";

    // Query (GET)
    Get "get/text" "Get element text" sel();
    Get "get/html" "Get inner HTML" sel();
    Get "get/value" "Get input value" sel();
    Get "get/attr" "Get attribute" sel(), s("attribute", "Attribute name");
    Get "get/url" "Get current URL";
    Get "get/title" "Get title";
    Get "get/count" "Count elements" sel();
    Get "get/box" "Bounding box" sel();
    Get "get/styles" "Computed styles" sel();
    Get "get/cdp-url" "Get CDP URL";

    Get "is/visible" "Is visible" sel();
    Get "is/enabled" "Is enabled" sel();
    Get "is/checked" "Is checked" sel();

    Post "find/role" "Find by role" s("role", "ARIA role"), s_opt("action", "Sub-action"),
        s_opt("name", "Accessible name"), flag("exact", "exact", "Exact match");
    Post "find/text" "Find by text" s("text", "Text"), s_opt("action", "Sub-action"), flag("exact", "exact", "Exact match");
    Post "find/label" "Find by label" s("label", "Label"), s_opt("action", "Sub-action"), flag("exact", "exact", "Exact");
    Post "find/placeholder" "Find by placeholder" s("placeholder", "Placeholder"), s_opt("action", "Sub-action");
    Post "find/alt" "Find by alt text" s("alt", "Alt text"), s_opt("action", "Sub-action");
    Post "find/title" "Find by title" s("title", "Title text"), s_opt("action", "Sub-action");
    Post "find/testid" "Find by test id" s("testId", "Test id"), s_opt("action", "Sub-action");
    Post "find/first" "Find first" sel(), s_opt("action", "Sub-action");
    Post "find/last" "Find last" sel(), s_opt("action", "Sub-action");
    Post "find/nth" "Find nth" i("index", "Index"), sel(), s_opt("action", "Sub-action");

    Post "mouse/move" "Mouse move" i("x", "X"), i("y", "Y");
    Post "mouse/down" "Mouse down" s_opt("button", "Button (left|right|middle)");
    Post "mouse/up" "Mouse up" s_opt("button", "Button");
    Post "mouse/wheel" "Mouse wheel" i("deltaY", "deltaY"), s_opt("deltaX", "deltaX");

    Post "set/viewport" "Set viewport" i("width", "Width"), i("height", "Height"), f("scale", "Device scale factor");
    Post "set/device" "Emulate device" s("device", "Device name");
    Post "set/geo" "Set geolocation" f("latitude", "Latitude"), f("longitude", "Longitude");
    Post "set/geolocation" "Set geolocation" f("latitude", "Latitude"), f("longitude", "Longitude");
    Post "set/offline" "Set offline" flag("offline", "offline", "Offline mode");
    Post "set/headers" "Set HTTP headers" s("headers", "JSON headers object");
    Post "set/credentials" "Set HTTP credentials" s("username", "Username"), s("password", "Password");
    Post "set/auth" "Set HTTP credentials" s("username", "Username"), s("password", "Password");
    Post "set/media" "Emulate media" s_opt("colorScheme", "dark|light"), s_opt("reducedMotion", "reduce|no-preference");

    Post "network/route" "Route network" s("url", "URL pattern"), flag("abort", "abort", "Abort matched requests"),
        sflag("body", "body", false, "Response body JSON"), sflag("resourceType", "resource-type", false, "Resource types");
    Delete "network/unroute" "Remove route" s_opt("url", "URL pattern");
    Get "network/requests" "List requests" flag("clear", "clear", "Clear after read"),
        sflag("filter", "filter", false, "Filter"), sflag("type", "type", false, "Resource type"),
        sflag("method", "method", false, "HTTP method"), sflag("status", "status", false, "Status code");
    Get "network/request" "Request detail" s("requestId", "Request id");
    Post "network/har/start" "Start HAR";
    Post "network/har/stop" "Stop HAR" s_opt("path", "HAR output path");

    Get "storage/local" "Read all localStorage" s_opt("key", "Key");
    Get "storage/session" "Read all sessionStorage" s_opt("key", "Key");
    Get "storage/local/get" "Get localStorage" s("key", "Key");
    Post "storage/local/set" "Set localStorage" s("key", "Key"), s("value", "Value");
    Delete "storage/local/clear" "Clear localStorage";
    Get "storage/session/get" "Get sessionStorage" s("key", "Key");
    Post "storage/session/set" "Set sessionStorage" s("key", "Key"), s("value", "Value");
    Delete "storage/session/clear" "Clear sessionStorage";

    Get "cookies" "Get cookies";
    Get "cookies/get" "Get cookies";
    Post "cookies/set" "Set cookie" s("name", "Cookie name"), s("value", "Cookie value"),
        sflag("domain", "domain", false, "Domain"), sflag("path", "path", false, "Path"),
        flag("httpOnly", "httpOnly", "HttpOnly"), flag("secure", "secure", "Secure"),
        sflag("sameSite", "sameSite", false, "SameSite"), sflag("expires", "expires", false, "Expires timestamp");
    Delete "cookies/clear" "Clear cookies";

    Get "tab" "List tabs";
    Get "tab/list" "List tabs";
    Post "tab/new" "New tab" url_opt(), sflag("label", "label", false, "Tab label");
    Delete "tab/close" "Close tab" s_opt("tabId", "Tab id or index");

    Post "window/new" "New window";
    Get "frame/main" "Main frame";
    Post "dialog/accept" "Accept dialog" s_opt("promptText", "Prompt text");
    Post "dialog/dismiss" "Dismiss dialog" s_opt("promptText", "Prompt text");
    Get "dialog/status" "Dialog status";

    Post "trace/start" "Start trace";
    Post "trace/stop" "Stop trace" s_opt("path", "Trace output path");
    Post "profiler/start" "Start profiler" sflag("categories", "categories", false, "Category list");
    Post "profiler/stop" "Stop profiler" s_opt("path", "Output path");
    Post "record/start" "Start recording" s("path", "Output webm path"), url_opt();
    Post "record/stop" "Stop recording";
    Post "record/restart" "Restart recording" s("path", "Output path"), url_opt();

    Get "console" "Console messages" flag("clear", "clear", "Clear after read");
    Get "errors" "Page errors" flag("clear", "clear", "Clear after read");
    Post "highlight" "Highlight element" sel();

    Get "clipboard" "Read clipboard";
    Get "clipboard/read" "Read clipboard";
    Post "clipboard/write" "Write clipboard" txt();
    Post "clipboard/copy" "Copy selection";
    Post "clipboard/paste" "Paste";

    Post "state/save" "Save state" s("path", "Output path");
    Post "state/load" "Load state" s("path", "State file path");
    Get "state/list" "List state files";
    Delete "state/clear" "Clear state" flag("all", "all", "Clear all"), s_opt("sessionName", "Session name");
    Get "state/show" "Show state file" s("filename", "State filename");
    Post "state/clean" "Clean old state" sflag("olderThanDays", "older-than", true, "Purge files older than days");
    Post "state/rename" "Rename state" s("oldName", "Old name"), s("newName", "New name");

    Post "tap" "Tap" sel();
    Post "swipe" "Swipe" s("direction", "up|down|left|right"), i("distance", "Distance px");
    Get "device" "List iOS devices";
    Get "device/list" "List iOS devices";

    Post "diff/snapshot" "Diff snapshot"
        sflag("baseline", "baseline", false, "Baseline file"),
        sel_opt(), flag("compact", "compact", "Compact"), sflag("depth", "depth", false, "Max depth");
    Post "diff/screenshot" "Diff screenshot" sflag("baseline", "baseline", true, "Baseline image"),
        s_opt("output", "Output path"), f("threshold", "Threshold 0-1"), sel_opt(), flag("fullPage", "full", "Full page");
    Post "diff/url" "Diff URLs" url_req(), s("url2", "Second URL"), flag("screenshot", "screenshot", "Include screenshots"),
        flag("fullPage", "full", "Full page"), sflag("waitUntil", "wait-until", false, "Wait until");

    Get "react/tree" "React component tree" flag("json", "json", "JSON output");
    Post "react/inspect" "Inspect React fiber" i("fiberId", "Fiber id"), flag("json", "json", "JSON output");
    Post "react/renders/start" "Start render profiling";
    Post "react/renders/stop" "Stop render profiling";
    Get "react/suspense" "Suspense boundaries" flag("json", "json", "JSON"), flag("onlyDynamic", "only-dynamic", "Only dynamic");

    Get "vitals" "Core Web Vitals" url_opt(), flag("json", "json", "JSON output");
    Get "web-vitals" "Core Web Vitals" url_opt(), flag("json", "json", "JSON output");

    Post "removeinitscript" "Remove init script" s("identifier", "Script id");
}

/// Lookup a static route by CLI path and HTTP method.
pub fn find_route(path: &str, method: &Method) -> Option<&'static HttpRouteSpec> {
    let want = match *method {
        Method::GET => HttpMethod::Get,
        Method::POST => HttpMethod::Post,
        Method::DELETE => HttpMethod::Delete,
        _ => return None,
    };
    HTTP_ROUTES
        .iter()
        .find(|r| r.path == path && r.method == want)
}

pub fn openapi_tag(path: &str) -> &'static str {
    crate::commands::catalog::openapi_tag(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_routes_cover_catalog_paths() {
        for path in crate::commands::catalog::SESSION_HTTP_PATHS {
            assert!(
                HTTP_ROUTES.iter().any(|r| r.path == *path),
                "missing HTTP route spec for {}",
                path
            );
        }
    }

    #[test]
    fn http_routes_use_proper_verbs() {
        let get_count = HTTP_ROUTES.iter().filter(|r| r.method == HttpMethod::Get).count();
        let delete_count = HTTP_ROUTES
            .iter()
            .filter(|r| r.method == HttpMethod::Delete)
            .count();
        assert!(get_count >= 40, "expected many GET routes, got {}", get_count);
        assert!(delete_count >= 10, "expected DELETE routes, got {}", delete_count);
        assert!(!HTTP_ROUTES.iter().any(|r| {
            matches!(r.path, "get/url" | "is/visible" | "cookies") && r.method != HttpMethod::Get
        }));
    }
}
