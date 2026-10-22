//! Generate a type safe TypeScript client for a [seamless](https://docs.rs/seamless) API.
//!
//! This crate is `no_std` (though it requires `alloc`). See [`generate_client()`].

// Tests use std (eg via the ApiBody macro), so only be no_std outside of them.
#![cfg_attr(not(test), no_std)]
#![warn(missing_docs)]

extern crate alloc;

use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{self, Write};
use seamless_info::{ApiBodyInfo, ApiBodyType, RouteInfo};

const INDENT: &str = "    ";

/// Take the JSON obtained by serializing the [`Vec<RouteInfo>`] for some API (ie the result of
/// calling `seamless::Api::info()`), and write TypeScript for a type safe client for the API
/// to the [`Write`] implementation provided (eg a [`String`]).
///
/// The generated TypeScript contains:
///
/// - A type for the response of each route, named after the route (eg `UsersGetById` for
///   the route `users.get_by_id`), and if the route isn't a `GET` route, a type for the
///   request body too (eg `UsersGetByIdParams`).
/// - A `createApiClient` function. This takes a function capable of making requests to the
///   API, of the form `(method: string, route: string, params?: any, opts?: O) => Promise<any>`,
///   where `method` is the route's HTTP method (eg `"GET"` or `"POST"`), and returns
///   a client object with a typed method for each route. Routes are grouped by the `.` or `/`
///   separated parts of their names, so the route `users.get_by_id` is called with
///   `client.users.getById(params, opts)`.
///
/// # Errors
///
/// Errors if the JSON provided can't be deserialized into an [`Vec<RouteInfo>`], if a route name
/// can't be turned into names in the generated TypeScript, or if two routes would end up with the
/// same name in the generated TypeScript. Nothing is written in any of these cases. Also errors
/// if writing to `out` fails, in which case some of the output may already have been written.
pub fn generate_client<W: Write + ?Sized>(route_info: &str, out: &mut W) -> Result<(), Error> {
    let mut routes: Vec<RouteInfo> =
        serde_json::from_str(route_info).map_err(Error::Deserialize)?;
    routes.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.method.cmp(&b.method)));

    let routes: Vec<Route<'_>> = routes.iter().map(Route::new).collect::<Result<_, _>>()?;
    check_type_names(&routes)?;
    let client = build_client_tree(&routes)?;

    write_ts(out, &routes, &client).map_err(Error::Write)
}

/// An error generating the TypeScript client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The JSON provided could not be deserialized into an [`Vec<RouteInfo>`].
    #[error("Could not deserialize the API info: {0}")]
    Deserialize(serde_json::Error),
    /// The generated TypeScript could not be written to the output.
    #[error("Could not write the generated TypeScript")]
    Write(fmt::Error),
    /// The route name is empty, or one of its `.` or `/` separated parts contains no letters
    /// or digits, so it can't be named in the generated TypeScript.
    #[error("The route '{0}' can't be named in the generated TypeScript")]
    InvalidRouteName(String),
    /// Two routes would both produce TypeScript types with the given name.
    #[error(
        "The routes '{first_route}' and '{second_route}' would both generate the TypeScript type '{type_name}'"
    )]
    DuplicateTypeName {
        /// The name of the TypeScript type.
        type_name: String,
        /// The first route that produces this type name.
        first_route: String,
        /// The second route that produces this type name.
        second_route: String,
    },
    /// Two routes would both be found at the same place in the client object
    /// (or one route would need to be in the middle of the path to another).
    #[error(
        "The routes '{first_route}' and '{second_route}' would clash at '{client_path}' in the generated client"
    )]
    DuplicateClientPath {
        /// The path in the client object, eg `users.getById`.
        client_path: String,
        /// The first route that is placed here.
        first_route: String,
        /// The second route that is placed here.
        second_route: String,
    },
}

/// A route, along with the names we'll use for it in the generated TypeScript.
struct Route<'a> {
    info: &'a RouteInfo,
    /// Name of the response type, eg `UsersGetById`.
    type_name: String,
    /// Name of the params type, eg `UsersGetByIdParams`, if this route takes a request body.
    params_type_name: Option<String>,
    /// Where in the client object this route lives, eg `["users", "getById"]`.
    client_path: Vec<String>,
}

impl<'a> Route<'a> {
    fn new(info: &'a RouteInfo) -> Result<Self, Error> {
        let segments: Vec<&str> = info
            .name
            .split(['.', '/'])
            .filter(|s| !s.is_empty())
            .collect();

        let client_path: Vec<String> = segments.iter().map(|s| camel_case(s)).collect();
        if client_path.is_empty() || client_path.iter().any(|s| s.is_empty()) {
            return Err(Error::InvalidRouteName(info.name.clone()));
        }

        let mut type_name: String = segments.iter().map(|s| pascal_case(s)).collect();
        if type_name.starts_with(|c: char| c.is_ascii_digit()) {
            type_name.insert(0, '_');
        }

        // Routes that aren't GET routes are expected to be given a body.
        let params_type_name = (!info.method.eq_ignore_ascii_case("GET")).then(|| {
            let mut name = type_name.clone();
            name.push_str("Params");
            name
        });

        Ok(Route {
            info,
            type_name,
            params_type_name,
            client_path,
        })
    }
}

/// Ensure that no two routes produce the same TypeScript type names.
fn check_type_names(routes: &[Route<'_>]) -> Result<(), Error> {
    let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
    for route in routes {
        let names = core::iter::once(&route.type_name).chain(&route.params_type_name);
        for type_name in names {
            if let Some(first_route) = seen.insert(type_name, &route.info.name) {
                return Err(Error::DuplicateTypeName {
                    type_name: type_name.clone(),
                    first_route: first_route.to_owned(),
                    second_route: route.info.name.clone(),
                });
            }
        }
    }
    Ok(())
}

/// A node in the client object; either a route function or an object containing more nodes.
enum ClientNode<'r, 'a> {
    Route(&'r Route<'a>),
    Group(BTreeMap<&'r str, ClientNode<'r, 'a>>),
}

impl<'r, 'a> ClientNode<'r, 'a> {
    /// The name of some route found at or below this node (for error messages).
    fn some_route_name(&self) -> &'a str {
        match self {
            ClientNode::Route(route) => &route.info.name,
            ClientNode::Group(group) => group
                .values()
                .next()
                .map(ClientNode::some_route_name)
                .unwrap_or_default(),
        }
    }
}

/// Group the routes into the shape of the client object that we'll generate.
fn build_client_tree<'r, 'a>(
    routes: &'r [Route<'a>],
) -> Result<BTreeMap<&'r str, ClientNode<'r, 'a>>, Error> {
    let mut tree = BTreeMap::new();
    for route in routes {
        let clash = |existing: &ClientNode<'_, '_>, depth: usize| Error::DuplicateClientPath {
            client_path: route.client_path[..=depth].join("."),
            first_route: existing.some_route_name().to_owned(),
            second_route: route.info.name.clone(),
        };

        let (last, parents) = route
            .client_path
            .split_last()
            .expect("client paths are never empty");

        let mut group = &mut tree;
        for (depth, key) in parents.iter().enumerate() {
            let node = group
                .entry(key.as_str())
                .or_insert_with(|| ClientNode::Group(BTreeMap::new()));
            group = match node {
                ClientNode::Group(g) => g,
                route => return Err(clash(route, depth)),
            };
        }
        if let Some(existing) = group.get(last.as_str()) {
            return Err(clash(existing, parents.len()));
        }
        group.insert(last.as_str(), ClientNode::Route(route));
    }
    Ok(tree)
}

fn write_ts<W: Write + ?Sized>(
    out: &mut W,
    routes: &[Route<'_>],
    client: &BTreeMap<&str, ClientNode<'_, '_>>,
) -> fmt::Result {
    writeln!(out, "// THIS IS AN AUTO GENERATED FILE. DO NOT EDIT")?;

    // The types for each route, separated by two blank lines:
    for (idx, route) in routes.iter().enumerate() {
        let info = route.info;
        out.write_str(if idx == 0 { "\n" } else { "\n\n" })?;
        write_route_comment(out, route, "/*", 0)?;
        write!(out, "export type {} = ", route.type_name)?;
        write_type(out, &info.response_type.ty, 0)?;
        writeln!(out)?;
        if let Some(params_type_name) = &route.params_type_name {
            write!(out, "export type {params_type_name} = ")?;
            write_type(out, &info.request_type.ty, 0)?;
            writeln!(out)?;
        }
    }

    // The client:
    out.write_str(concat!(
        "\n",
        "/**\n",
        " * Instantiate a fully typed API client from the above, given an\n",
        " * 'api' function that's capable of taking an HTTP method, a route\n",
        " * string and params (if not GET), and making a request to the backend\n",
        " * to return the expected response type.\n",
        " */\n",
        "export function createApiClient<O = never>(api: (method: string, route: string, params?: any, opts?: O) => Promise<any>) {\n",
    ))?;
    write!(out, "{INDENT}return ")?;
    write_client_group(out, client, 1)?;
    writeln!(out, "\n}}")
}

/// Write an object in the client containing the nodes given.
fn write_client_group<W: Write + ?Sized>(
    out: &mut W,
    group: &BTreeMap<&str, ClientNode<'_, '_>>,
    indent: usize,
) -> fmt::Result {
    if group.is_empty() {
        return write!(out, "{{}}");
    }
    writeln!(out, "{{")?;
    for (idx, (key, node)) in group.iter().enumerate() {
        if idx > 0 {
            writeln!(out, ",")?;
        }
        match node {
            ClientNode::Group(group) => {
                write_indent(out, indent + 1)?;
                write!(out, "{}: ", PropertyKey(key))?;
                write_client_group(out, group, indent + 1)?;
            }
            ClientNode::Route(route) => {
                write_route_comment(out, route, "/**", indent + 1)?;
                write_indent(out, indent + 1)?;
                write!(out, "{}: function(", PropertyKey(key))?;
                if let Some(params_type_name) = &route.params_type_name {
                    write!(out, "params: {params_type_name}, ")?;
                }
                writeln!(out, "opts?: O): Promise<{}> {{", route.type_name)?;
                write_indent(out, indent + 2)?;
                let params = if route.params_type_name.is_some() {
                    "params"
                } else {
                    "undefined"
                };
                writeln!(
                    out,
                    "return api({}, {}, {params}, opts)",
                    SingleQuoted(&route.info.method),
                    SingleQuoted(&route.info.name)
                )?;
                write_indent(out, indent + 1)?;
                write!(out, "}}")?;
            }
        }
    }
    writeln!(out)?;
    write_indent(out, indent)?;
    write!(out, "}}")
}

/// Write a comment like the following, with the given opening (eg `/*` or `/**`):
///
/// ```text
/// /*
///  * POST /users.get_by_id
///  *
///  * Route description
///  */
/// ```
fn write_route_comment<W: Write + ?Sized>(
    out: &mut W,
    route: &Route<'_>,
    open: &str,
    indent: usize,
) -> fmt::Result {
    let info = route.info;
    write_indent(out, indent)?;
    writeln!(out, "{open}")?;
    write_indent(out, indent)?;
    writeln!(
        out,
        " * {} /{}",
        Comment(&info.method),
        Comment(info.name.trim_start_matches('/'))
    )?;
    let description = info.description.trim();
    if !description.is_empty() {
        write_indent(out, indent)?;
        writeln!(out, " *")?;
        write_comment_lines(out, description, indent)?;
    }
    write_indent(out, indent)?;
    writeln!(out, " */")
}

/// Write each line of some text as ` * line`, as part of a block comment.
fn write_comment_lines<W: Write + ?Sized>(out: &mut W, text: &str, indent: usize) -> fmt::Result {
    for line in text.lines() {
        write_indent(out, indent)?;
        let line = line.trim();
        if line.is_empty() {
            writeln!(out, " *")?;
        } else {
            writeln!(out, " * {}", Comment(line))?;
        }
    }
    Ok(())
}

/// Write a doc comment for some object property. Single line descriptions are written
/// as `/** description */`, and longer ones are written as a block.
fn write_doc_comment<W: Write + ?Sized>(
    out: &mut W,
    description: &str,
    indent: usize,
) -> fmt::Result {
    let description = description.trim();
    if description.is_empty() {
        return Ok(());
    }
    write_indent(out, indent)?;
    if !description.contains('\n') {
        return writeln!(out, "/** {} */", Comment(description));
    }
    writeln!(out, "/**")?;
    write_comment_lines(out, description, indent)?;
    write_indent(out, indent)?;
    writeln!(out, " */")
}

/// Write out the TypeScript type corresponding to some [`ApiBodyType`]. The first line is
/// written wherever we currently are, and any further lines are indented by `indent`.
fn write_type<W: Write + ?Sized>(out: &mut W, ty: &ApiBodyType, indent: usize) -> fmt::Result {
    match ty {
        ApiBodyType::String => write!(out, "string"),
        ApiBodyType::Number => write!(out, "number"),
        ApiBodyType::Boolean => write!(out, "boolean"),
        ApiBodyType::Null => write!(out, "null"),
        ApiBodyType::Any => write!(out, "any"),
        ApiBodyType::Unknown => write!(out, "unknown"),
        ApiBodyType::Binary => write!(out, "Blob"),
        ApiBodyType::ArrayOf { value } => {
            write!(out, "(")?;
            write_type(out, &value.ty, indent)?;
            write!(out, ")[]")
        }
        ApiBodyType::TupleOf { values } => {
            write!(out, "[")?;
            for (idx, value) in values.iter().enumerate() {
                if idx > 0 {
                    write!(out, ", ")?;
                }
                write_type(out, &value.ty, indent)?;
            }
            write!(out, "]")
        }
        ApiBodyType::ObjectOf { value } => {
            writeln!(out, "{{")?;
            write_indent(out, indent + 1)?;
            write!(out, "[key: string]: ")?;
            write_type(out, &value.ty, indent + 1)?;
            writeln!(out)?;
            write_indent(out, indent)?;
            write!(out, "}}")
        }
        ApiBodyType::Object { keys } => {
            if keys.is_empty() {
                return write!(out, "{{}}");
            }
            writeln!(out, "{{")?;
            for (key, value) in keys {
                write_object_property(out, key, value, indent + 1)?;
            }
            write_indent(out, indent)?;
            write!(out, "}}")
        }
        ApiBodyType::OneOf { values } => {
            if values.is_empty() {
                return write!(out, "never");
            }
            for (idx, value) in values.iter().enumerate() {
                if idx > 0 {
                    write!(out, " | ")?;
                }
                write_type(out, &value.ty, indent)?;
            }
            Ok(())
        }
        ApiBodyType::StringLiteral { literal } => write!(out, "{}", DoubleQuoted(literal)),
        // Optional only really makes sense as the type of an object property, which is
        // handled in `write_object_property`. Elsewhere, the value may be undefined.
        ApiBodyType::Optional { value } => {
            write_type(out, &value.ty, indent)?;
            write!(out, " | undefined")
        }
    }
}

/// Write out a single property of an object, eg `key?: string`, along with its description.
fn write_object_property<W: Write + ?Sized>(
    out: &mut W,
    key: &str,
    value: &ApiBodyInfo,
    indent: usize,
) -> fmt::Result {
    let (optional, inner) = match &value.ty {
        ApiBodyType::Optional { value: inner } => (true, &**inner),
        _ => (false, value),
    };
    let description = if value.description.is_empty() {
        &inner.description
    } else {
        &value.description
    };

    write_doc_comment(out, description, indent)?;
    write_indent(out, indent)?;
    write!(
        out,
        "{}{}: ",
        PropertyKey(key),
        if optional { "?" } else { "" }
    )?;
    write_type(out, &inner.ty, indent)?;
    writeln!(out)
}

fn write_indent<W: Write + ?Sized>(out: &mut W, indent: usize) -> fmt::Result {
    for _ in 0..indent {
        out.write_str(INDENT)?;
    }
    Ok(())
}

/// Split some name like `get_by_id` or `get-by-id` into its words.
fn words(s: &str) -> impl Iterator<Item = &str> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
}

/// Write a word with its first character uppercased or lowercased.
fn push_word(out: &mut String, word: &str, upper: bool) {
    let mut chars = word.chars();
    if let Some(first) = chars.next() {
        if upper {
            out.extend(first.to_uppercase());
        } else {
            out.extend(first.to_lowercase());
        }
        out.push_str(chars.as_str());
    }
}

/// `get_by_id` => `GetById`.
fn pascal_case(s: &str) -> String {
    let mut out = String::new();
    for word in words(s) {
        push_word(&mut out, word, true);
    }
    out
}

/// `get_by_id` => `getById`.
fn camel_case(s: &str) -> String {
    let mut out = String::new();
    for (idx, word) in words(s).enumerate() {
        push_word(&mut out, word, idx > 0);
    }
    out
}

/// Display an object key, quoting it if it isn't a valid identifier.
struct PropertyKey<'a>(&'a str);

impl fmt::Display for PropertyKey<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut chars = self.0.chars();
        let is_ident = chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
        if is_ident {
            f.write_str(self.0)
        } else {
            DoubleQuoted(self.0).fmt(f)
        }
    }
}

/// Display a string as a double quoted string literal.
struct DoubleQuoted<'a>(&'a str);

impl fmt::Display for DoubleQuoted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_quoted(f, self.0, '"')
    }
}

/// Display a string as a single quoted string literal.
struct SingleQuoted<'a>(&'a str);

impl fmt::Display for SingleQuoted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_quoted(f, self.0, '\'')
    }
}

fn write_quoted(f: &mut fmt::Formatter<'_>, s: &str, quote: char) -> fmt::Result {
    f.write_char(quote)?;
    for c in s.chars() {
        match c {
            '\\' => f.write_str("\\\\")?,
            '\n' => f.write_str("\\n")?,
            '\r' => f.write_str("\\r")?,
            '\t' => f.write_str("\\t")?,
            c if c == quote => {
                f.write_char('\\')?;
                f.write_char(c)?;
            }
            c if c.is_control() || c == '\u{2028}' || c == '\u{2029}' => {
                write!(f, "\\u{{{:x}}}", c as u32)?;
            }
            c => f.write_char(c)?,
        }
    }
    f.write_char(quote)
}

/// Display some text such that it can't end the comment that it's in.
struct Comment<'a>(&'a str);

impl fmt::Display for Comment<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = self.0.split("*/");
        if let Some(first) = parts.next() {
            f.write_str(first)?;
        }
        for part in parts {
            f.write_str("*\\/")?;
            f.write_str(part)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use pretty_assertions::assert_eq;
    use seamless::{
        api::{Api, ApiBody},
        handler::{
            body::{FromBinary, FromJson},
            response::ToJson,
        },
    };
    use serde_json::{Map, Value, json};

    /// A widget
    #[ApiBody]
    struct Widget {
        /// The widget name
        name: String,
        size: (u8, u8),
        colour: Option<Colour>,
    }

    #[ApiBody]
    enum Colour {
        Red,
        Blue,
    }

    #[ApiBody]
    struct StackRequest {
        items: Vec<Item>,
    }

    #[ApiBody]
    enum Item {
        Crate {
            width: u8,
        },
        Ball {
            /// The ball radius
            radius: f64,
        },
    }

    #[ApiBody]
    struct TuneParams {
        /// How much to tune by
        #[api_body(optional)]
        amount: Option<f64>,
        /// Some
        /// multi-line docs.
        r#type: String,
    }

    fn generate(route_info: &str) -> Result<String, Error> {
        let mut out = String::new();
        generate_client(route_info, &mut out)?;
        Ok(out)
    }

    // The handlers are never called; we only care about the types they describe.
    fn never<T>() -> T {
        unimplemented!()
    }

    fn example_api() -> Api {
        let mut api = Api::new();
        api.add("widgets.list_all")
            .description("List all of the widgets")
            .handler(|| async { ToJson(never::<Vec<Widget>>()) });
        api.add("shelf/stack_items")
            .description("Stack some items on the shelf")
            .handler(|FromJson(_): FromJson<StackRequest>| ToJson(()));
        api.add("gadgets.upload")
            .description("Upload a gadget")
            .handler(|_: FromBinary| ToJson(never::<Map<String, Value>>()));
        api.add("gadgets.tune")
            .description("Tune a gadget.\n\nIt's */ quite\na \"good\" route")
            .handler(|FromJson(_): FromJson<TuneParams>| ToJson(true));
        api
    }

    const EXPECTED: &str = r#"// THIS IS AN AUTO GENERATED FILE. DO NOT EDIT

/*
 * POST /gadgets.tune
 *
 * Tune a gadget.
 *
 * It's *\/ quite
 * a "good" route
 */
export type GadgetsTune = boolean
export type GadgetsTuneParams = {
    /** How much to tune by */
    amount?: number
    /**
     * Some
     * multi-line docs.
     */
    type: string
}


/*
 * POST /gadgets.upload
 *
 * Upload a gadget
 */
export type GadgetsUpload = {
    [key: string]: any
}
export type GadgetsUploadParams = Blob


/*
 * POST /shelf/stack_items
 *
 * Stack some items on the shelf
 */
export type ShelfStackItems = null
export type ShelfStackItemsParams = {
    items: ({
        /** Variant tag */
        kind: "Crate"
        width: number
    } | {
        /** Variant tag */
        kind: "Ball"
        /** The ball radius */
        radius: number
    })[]
}


/*
 * GET /widgets.list_all
 *
 * List all of the widgets
 */
export type WidgetsListAll = ({
    colour: "Red" | "Blue" | null
    /** The widget name */
    name: string
    size: [number, number]
})[]

/**
 * Instantiate a fully typed API client from the above, given an
 * 'api' function that's capable of taking an HTTP method, a route
 * string and params (if not GET), and making a request to the backend
 * to return the expected response type.
 */
export function createApiClient<O = never>(api: (method: string, route: string, params?: any, opts?: O) => Promise<any>) {
    return {
        gadgets: {
            /**
             * POST /gadgets.tune
             *
             * Tune a gadget.
             *
             * It's *\/ quite
             * a "good" route
             */
            tune: function(params: GadgetsTuneParams, opts?: O): Promise<GadgetsTune> {
                return api('POST', 'gadgets.tune', params, opts)
            },
            /**
             * POST /gadgets.upload
             *
             * Upload a gadget
             */
            upload: function(params: GadgetsUploadParams, opts?: O): Promise<GadgetsUpload> {
                return api('POST', 'gadgets.upload', params, opts)
            }
        },
        shelf: {
            /**
             * POST /shelf/stack_items
             *
             * Stack some items on the shelf
             */
            stackItems: function(params: ShelfStackItemsParams, opts?: O): Promise<ShelfStackItems> {
                return api('POST', 'shelf/stack_items', params, opts)
            }
        },
        widgets: {
            /**
             * GET /widgets.list_all
             *
             * List all of the widgets
             */
            listAll: function(opts?: O): Promise<WidgetsListAll> {
                return api('GET', 'widgets.list_all', undefined, opts)
            }
        }
    }
}
"#;

    #[test]
    fn generates_expected_client() {
        let info = serde_json::to_string(&example_api().info()).unwrap();
        let ts = generate(&info).unwrap();

        // Handy for checking the output with `tsc`:
        if let Some(path) = std::env::var_os("SEAMLESS_TS_OUT") {
            std::fs::write(path, &ts).unwrap();
        }

        assert_eq!(ts, EXPECTED);
    }

    #[test]
    fn unusual_names_and_types() {
        let route = |name: &str, method: &str| {
            json!({
                "name": name,
                "method": method,
                "description": "",
                "request_type": { "description": "", "shape": { "type": "Null" } },
                "response_type": {
                    "description": "",
                    "shape": {
                        "type": "Object",
                        "keys": {
                            "not-an-ident": { "description": "", "shape": { "type": "Unknown" } },
                            "maybe": {
                                "description": "",
                                "shape": { "type": "Optional", "value": { "description": "", "shape": { "type": "Boolean" } } }
                            },
                            "empty": { "description": "", "shape": { "type": "Object", "keys": {} } },
                            "lit": { "description": "", "shape": { "type": "StringLiteral", "literal": "it's \"quoted\"" } }
                        }
                    }
                }
            })
        };
        let info = json!([
            route("flim/flam", "PUT"),
            route("it's/2nd-route", "GET"),
            route("top", "GET"),
        ]);
        let ts = generate(&info.to_string()).unwrap();

        assert!(ts.contains(
            r#"export type FlimFlam = {
    empty: {}
    lit: "it's \"quoted\""
    maybe?: boolean
    "not-an-ident": unknown
}
export type FlimFlamParams = null"#
        ));
        assert!(ts.contains("return api('PUT', 'flim/flam', params, opts)"));
        assert!(ts.contains("export type ItS2ndRoute = {"));
        assert!(ts.contains(
            r#"        itS: {
            /**
             * GET /it's/2nd-route
             */
            "2ndRoute": function(opts?: O): Promise<ItS2ndRoute> {
                return api('GET', 'it\'s/2nd-route', undefined, opts)
            }
        },"#
        ));
        assert!(ts.contains(
            "        top: function(opts?: O): Promise<Top> {\n            return api('GET', 'top', undefined, opts)\n        }\n    }\n}\n"
        ));
    }

    #[test]
    fn empty_api() {
        let ts = generate("[]").unwrap();
        assert!(ts.ends_with("    return {}\n}\n"));
    }

    fn route(name: &str) -> Value {
        json!({
            "name": name,
            "method": "GET",
            "description": "",
            "request_type": { "description": "", "shape": { "type": "Null" } },
            "response_type": { "description": "", "shape": { "type": "Null" } }
        })
    }

    #[test]
    fn writes_nothing_on_error() {
        let info = json!([route("foo"), route("foo.bar")]).to_string();
        let mut out = String::new();
        assert!(generate_client(&info, &mut out).is_err());
        assert_eq!(out, "");
    }

    #[test]
    fn errors_on_invalid_json() {
        let err = generate(r#"[{ "name": "foo" }]"#).unwrap_err();
        assert!(matches!(err, Error::Deserialize(_)));
    }

    #[test]
    fn errors_on_unnameable_routes() {
        for name in ["", "/", "foo.-", "foo.-.bar"] {
            let info = json!([route(name)]).to_string();
            let err = generate(&info).unwrap_err();
            assert!(
                matches!(&err, Error::InvalidRouteName(n) if n == name),
                "unexpected error for '{name}': {err:?}"
            );
        }
    }

    #[test]
    fn errors_on_clashing_type_names() {
        let info = json!([route("foo.bar_baz"), route("foo_bar.baz")]).to_string();
        let err = generate(&info).unwrap_err();
        assert_eq!(
            err.to_string(),
            "The routes 'foo.bar_baz' and 'foo_bar.baz' would both generate the TypeScript type 'FooBarBaz'"
        );
    }

    #[test]
    fn errors_on_clashing_client_paths() {
        let info = json!([route("foo"), route("foo.bar")]).to_string();
        let err = generate(&info).unwrap_err();
        assert_eq!(
            err.to_string(),
            "The routes 'foo' and 'foo.bar' would clash at 'foo' in the generated client"
        );

        let info = json!([route("foo.bar"), route("foo.bar_")]).to_string();
        let err = generate(&info).unwrap_err();
        assert!(matches!(err, Error::DuplicateTypeName { .. }));

        let info = json!([route("foo.bar.baz"), route("foo.bar")]).to_string();
        let err = generate(&info).unwrap_err();
        assert_eq!(
            err.to_string(),
            "The routes 'foo.bar' and 'foo.bar.baz' would clash at 'foo.bar' in the generated client"
        );
    }
}
