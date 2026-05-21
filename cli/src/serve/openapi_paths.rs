use salvo::oapi::{
    Array, BasicType, Content, Object, OpenApi, Operation, Parameter, ParameterIn, PathItem,
    RequestBody, Required, Response, Responses, Schema, SchemaType,
};
use salvo::oapi::Required as SchemaRequired;

use crate::commands::http_routes::{ArgIn, ArgSpec, HttpMethod, HTTP_ROUTES};
use crate::commands::openapi_tag;

fn schema_for_arg(spec: ArgSpec) -> Schema {
    match spec {
        ArgSpec::IntArg { .. } => Schema::object(Object::with_type(BasicType::Integer)),
        ArgSpec::FloatArg { .. } => Schema::object(Object::with_type(BasicType::Number)),
        ArgSpec::BoolFlag { .. } => Schema::object(Object::with_type(BasicType::Boolean)),
        ArgSpec::StringList { .. } => Schema::Array(
            Array::new().items(Object::with_type(BasicType::String)),
        ),
        _ => Schema::object(Object::with_type(BasicType::String)),
    }
}

fn nullable_schema(spec: ArgSpec) -> Schema {
    match spec {
        ArgSpec::StringList { .. } => Schema::Array(
            Array::new()
                .schema_type(SchemaType::from_iter([BasicType::Array, BasicType::Null]))
                .items(Object::with_type(BasicType::String)),
        ),
        ArgSpec::IntArg { .. } => Schema::object(Object::with_type(SchemaType::from_iter([
            BasicType::Integer,
            BasicType::Null,
        ]))),
        ArgSpec::FloatArg { .. } => Schema::object(Object::with_type(SchemaType::from_iter([
            BasicType::Number,
            BasicType::Null,
        ]))),
        ArgSpec::BoolFlag { .. } => Schema::object(Object::with_type(SchemaType::from_iter([
            BasicType::Boolean,
            BasicType::Null,
        ]))),
        _ => Schema::object(Object::with_type(SchemaType::from_iter([
            BasicType::String,
            BasicType::Null,
        ]))),
    }
}

fn parameter_from_arg(spec: ArgSpec) -> Parameter {
    let mut p = Parameter::new(spec.openapi_name())
        .parameter_in(ParameterIn::Query)
        .description(spec.description())
        .schema(schema_for_arg(spec));
    if spec.is_required() {
        p.required = SchemaRequired::True;
    }
    p
}

fn body_schema_for_route(args: &[ArgSpec], method: HttpMethod) -> Option<Object> {
    if method != HttpMethod::Post {
        return None;
    }
    let mut obj = Object::new();
    let mut any = false;
    for spec in args {
        if spec.resolve_location(method) != ArgIn::Body {
            continue;
        }
        any = true;
        let schema = if spec.is_required() {
            schema_for_arg(*spec)
        } else {
            nullable_schema(*spec)
        };
        obj.properties
            .insert(spec.openapi_name().to_string(), schema.into());
        if spec.is_required() {
            obj.required.insert(spec.openapi_name().to_string());
        }
    }
    if any {
        Some(obj)
    } else {
        None
    }
}

fn global_flag_parameters() -> Vec<Parameter> {
    const FLAGS: &[(&str, &str)] = &[
        ("json", "JSON output"),
        ("headed", "Headed browser"),
        ("debug", "Debug logging"),
        ("sessionName", "Auto-save session name"),
        ("executablePath", "Chrome executable path"),
        ("cdp", "CDP endpoint"),
        ("provider", "Provider (e.g. ios)"),
        ("profile", "Chrome profile"),
        ("state", "Storage state JSON path"),
        ("proxy", "Proxy URL"),
        ("proxyBypass", "Proxy bypass list"),
        ("userAgent", "User agent"),
        ("device", "iOS device"),
        ("engine", "Browser engine"),
        ("model", "AI model id"),
        ("defaultTimeout", "Default timeout ms"),
        ("autoConnect", "Auto-connect to Chrome"),
        ("ignoreHttpsErrors", "Ignore HTTPS errors"),
        ("allowFileAccess", "Allow file:// URLs"),
        ("headers", "HTTP headers JSON"),
        ("annotate", "Annotate screenshots"),
        ("colorScheme", "Color scheme"),
        ("downloadPath", "Download directory"),
        ("contentBoundaries", "Content boundaries in snapshot"),
        ("maxOutput", "Max snapshot output chars"),
        ("allowedDomains", "Allowed domains CSV"),
        ("actionPolicy", "Action policy path"),
        ("confirmActions", "Actions requiring confirmation"),
        ("confirmInteractive", "Interactive confirmation"),
        ("screenshotDir", "Screenshot directory"),
        ("screenshotQuality", "Screenshot JPEG quality"),
        ("screenshotFormat", "Screenshot format png|jpeg"),
        ("idleTimeout", "Daemon idle timeout"),
        ("noAutoDialog", "Disable auto dialog handler"),
    ];
    FLAGS
        .iter()
        .map(|(name, desc)| {
            Parameter::new(*name)
                .parameter_in(ParameterIn::Query)
                .description(*desc)
                .schema(Schema::object(Object::with_type(BasicType::String)))
        })
        .collect()
}

fn method_name(m: HttpMethod) -> &'static str {
    match m {
        HttpMethod::Get => "get",
        HttpMethod::Post => "post",
        HttpMethod::Delete => "delete",
    }
}

/// Register typed session command paths in OpenApi.
pub fn enrich_session_command_paths(mut doc: OpenApi) -> OpenApi {
    let session_param = Parameter::new("session")
        .parameter_in(ParameterIn::Path)
        .description("Browser session name")
        .required(SchemaRequired::True);

    let responses = Responses::new().response(
        "200",
        Response::new("Daemon JSON response (`success`, `data`, `error`)"),
    );

    let global_params = global_flag_parameters();

    for route in HTTP_ROUTES {
        let full = format!("/api/sessions/{{session}}/{}", route.path);
        let mut params: Vec<Parameter> = vec![session_param.clone()];
        params.extend(global_params.clone());

        for spec in route.args {
            if spec.resolve_location(route.method) == ArgIn::Query
                || route.method == HttpMethod::Get
            {
                params.push(parameter_from_arg(*spec));
            }
        }

        let mut op = Operation::new()
            .tags([openapi_tag(route.path)])
            .summary(route.summary)
            .operation_id(format!(
                "{}_{}",
                method_name(route.method),
                route.path.replace('/', "_")
            ))
            .parameters(params)
            .responses(responses.clone());

        if let Some(body_obj) = body_schema_for_route(route.args, route.method) {
            let rb = RequestBody::new()
                .description("Command arguments (JSON)")
                .required(Required::False)
                .add_content(
                    "application/json",
                    Content::new(Schema::object(body_obj)),
                );
            op = op.request_body(rb);
        }

        let item = PathItem::new(route.method.path_item_type(), op);
        doc = doc.add_path(full, item);
    }

    doc
}
