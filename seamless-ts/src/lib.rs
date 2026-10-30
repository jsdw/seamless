//! Generate TypeScript types describing a [seamless](https://docs.rs/seamless) API.
//!
//! This crate is `no_std` (though it requires `alloc`). See [`generate()`] and [`generate_from_str()`].

// Tests use std (eg via the ApiBody macro), so only be no_std outside of them.
#![cfg_attr(not(test), no_std)]
#![warn(missing_docs)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{self, Write};
use fmt_helpers::{
    DoubleQuoted, PropertyKey, SingleQuoted, write_doc_comment, write_indent, write_route_comment,
};
use seamless_info::{ApiBodyInfo, ApiBodyType, RouteInfo};

mod fmt_helpers;

/// Take the JSON obtained by serializing the [`Vec<RouteInfo>`] for some API (ie the result of
/// calling `seamless::Api::info()`), and write TypeScript describing the API to the [`Write`]
/// implementation provided (eg a [`String`]).
///
/// See [`generate`] for more information. This additionally errors if the JSON provided can't be
/// deserialized into a [`Vec<RouteInfo>`], in which case nothing is written.
pub fn generate_from_str<W: Write + ?Sized>(route_info: &str, out: &mut W) -> Result<(), Error> {
    let routes: Vec<RouteInfo> = serde_json::from_str(route_info).map_err(Error::Deserialize)?;

    generate(routes, out)
}

/// Take some [`Vec<RouteInfo>`] for some API (ie the result of calling `seamless::Api::info()`),
/// and write TypeScript describing the API to the [`Write`] implementation provided (eg a [`String`]).
///
/// The generated TypeScript exports an `Api` type, which maps each route name to its HTTP
/// method, the type of its request body (`params`, which is `undefined` for `GET` routes) and
/// the type of its response (`result`). Since types don't exist at runtime, it also exports a
/// readonly `methods` object mapping each route name to its HTTP method. For example:
///
/// ```ts
/// export type Api = {
///     'users.get_by_id': {
///         method: 'POST'
///         params: { id: string }
///         result: { id: string, name: string }
///     }
/// }
///
/// export const methods = {
///     'users.get_by_id': 'POST',
/// } as const
/// ```
///
/// This can be used to build a type safe client for the API, for instance something like:
///
/// ```ts
/// type Params<R extends keyof Api> = Api[R]['params'] extends undefined ? [] : [Api[R]['params']]
///
/// async function api<R extends keyof Api>(route: R, ...[params]: Params<R>): Promise<Api[R]['result']> {
///     let method = methods[route];
///
///     let res = await fetch('url', {
///         method,
///         body: JSON.stringify(params)
///     })
///
///     let body = await res.json()
///
///     if (!res.ok) {
///         // This depends on how you handle seamless errors:
///         throw new Error(body.message)
///     } else {
///         // This is the response body from seamless if all is well:
///         return body
///     }
/// }
/// ```
///
/// # Errors
///
/// Errors if two routes have the same name, in which case nothing is written. Also errors if writing to
/// `out` fails, in which case some of the output may already have been written.
pub fn generate<W: Write + ?Sized>(mut routes: Vec<RouteInfo>, out: &mut W) -> Result<(), Error> {
    routes.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.method.cmp(&b.method)));

    // Routes are keyed by name in the generated type, so names must be unique.
    if let Some(pair) = routes.windows(2).find(|pair| pair[0].name == pair[1].name) {
        return Err(Error::DuplicateRouteName(pair[0].name.clone()));
    }

    write_ts(out, &routes).map_err(Error::Write)
}

/// An error generating the TypeScript.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The JSON provided could not be deserialized into an [`Vec<RouteInfo>`].
    #[error("Could not deserialize the API info: {0}")]
    Deserialize(serde_json::Error),
    /// The generated TypeScript could not be written to the output.
    #[error("Could not write the generated TypeScript")]
    Write(fmt::Error),
    /// More than one route has the given name (eg the same path with different methods).
    #[error("More than one route has the name '{0}'")]
    DuplicateRouteName(String),
}

fn write_ts<W: Write + ?Sized>(out: &mut W, routes: &[RouteInfo]) -> fmt::Result {
    writeln!(out, "// THIS IS AN AUTO GENERATED FILE. DO NOT EDIT")?;
    writeln!(out)?;
    writeln!(out, "/**")?;
    writeln!(out, " * The routes provided by our API")?;
    writeln!(out, " */")?;
    write!(out, "export type Api = {{")?;
    if !routes.is_empty() {
        writeln!(out)?;
    }
    for route in routes {
        write_route_comment(out, route, 1)?;
        write_indent(out, 1)?;
        writeln!(out, "{}: {{", SingleQuoted(&route.name))?;
        write_indent(out, 2)?;
        writeln!(out, "method: {}", SingleQuoted(&route.method))?;
        // Routes that aren't GET routes are expected to be given a body.
        write_indent(out, 2)?;
        write!(out, "params: ")?;
        if route.method.eq_ignore_ascii_case("GET") {
            write!(out, "undefined")?;
        } else {
            write_type(out, &route.request_type.ty, 2)?;
        }
        writeln!(out)?;
        write_indent(out, 2)?;
        write!(out, "result: ")?;
        write_type(out, &route.response_type.ty, 2)?;
        writeln!(out)?;
        write_indent(out, 1)?;
        writeln!(out, "}}")?;
    }
    writeln!(out, "}}")?;

    // The methods are also needed at runtime, in order to make requests:
    writeln!(out)?;
    writeln!(out, "/**")?;
    writeln!(out, " * The HTTP method of each route")?;
    writeln!(out, " */")?;
    write!(out, "export const methods = {{")?;
    if !routes.is_empty() {
        writeln!(out)?;
    }
    for route in routes {
        write_indent(out, 1)?;
        writeln!(
            out,
            "{}: {},",
            SingleQuoted(&route.name),
            SingleQuoted(&route.method)
        )?;
    }
    writeln!(out, "}} as const")
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

    fn generate_string(route_info: &str) -> Result<String, Error> {
        let mut out = String::new();
        generate_from_str(route_info, &mut out)?;
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

/**
 * The routes provided by our API
 */
export type Api = {
    /**
     * POST /gadgets.tune
     *
     * Tune a gadget.
     *
     * It's *\/ quite
     * a "good" route
     */
    'gadgets.tune': {
        method: 'POST'
        params: {
            /** How much to tune by */
            amount?: number
            /**
             * Some
             * multi-line docs.
             */
            type: string
        }
        result: boolean
    }
    /**
     * POST /gadgets.upload
     *
     * Upload a gadget
     */
    'gadgets.upload': {
        method: 'POST'
        params: Blob
        result: {
            [key: string]: any
        }
    }
    /**
     * POST /shelf/stack_items
     *
     * Stack some items on the shelf
     */
    'shelf/stack_items': {
        method: 'POST'
        params: {
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
        result: null
    }
    /**
     * GET /widgets.list_all
     *
     * List all of the widgets
     */
    'widgets.list_all': {
        method: 'GET'
        params: undefined
        result: ({
            colour: "Red" | "Blue" | null
            /** The widget name */
            name: string
            size: [number, number]
        })[]
    }
}

/**
 * The HTTP method of each route
 */
export const methods = {
    'gadgets.tune': 'POST',
    'gadgets.upload': 'POST',
    'shelf/stack_items': 'POST',
    'widgets.list_all': 'GET',
} as const
"#;

    #[test]
    fn generates_expected_types() {
        let info = serde_json::to_string(&example_api().info()).unwrap();
        let ts = generate_string(&info).unwrap();

        // Handy for checking the output with `tsc`:
        if let Some(path) = std::env::var_os("SEAMLESS_TS_OUT") {
            std::fs::write(path, &ts).unwrap();
        }

        assert_eq!(ts, EXPECTED);
    }

    #[test]
    fn unusual_names_and_types() {
        let info = json!([{
            "name": "it's/a-route",
            "method": "PUT",
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
        }]);
        let ts = generate_string(&info.to_string()).unwrap();

        assert!(ts.contains(
            r#"    /**
     * PUT /it's/a-route
     */
    'it\'s/a-route': {
        method: 'PUT'
        params: null
        result: {
            empty: {}
            lit: "it's \"quoted\""
            maybe?: boolean
            "not-an-ident": unknown
        }
    }
}
"#
        ));
        assert!(
            ts.ends_with("export const methods = {\n    'it\\'s/a-route': 'PUT',\n} as const\n")
        );
    }

    #[test]
    fn empty_api() {
        let ts = generate_string("[]").unwrap();
        assert!(ts.ends_with("export type Api = {}\n\n/**\n * The HTTP method of each route\n */\nexport const methods = {} as const\n"));
    }

    fn route(name: &str, method: &str) -> Value {
        json!({
            "name": name,
            "method": method,
            "description": "",
            "request_type": { "description": "", "shape": { "type": "Null" } },
            "response_type": { "description": "", "shape": { "type": "Null" } }
        })
    }

    #[test]
    fn errors_on_invalid_json() {
        let err = generate_string(r#"[{ "name": "foo" }]"#).unwrap_err();
        assert!(matches!(err, Error::Deserialize(_)));
    }

    #[test]
    fn errors_on_duplicate_route_names() {
        let info = json!([
            route("foo", "POST"),
            route("bar", "GET"),
            route("foo", "GET")
        ]);
        let mut out = String::new();
        let err = generate_from_str(&info.to_string(), &mut out).unwrap_err();
        assert_eq!(err.to_string(), "More than one route has the name 'foo'");
        assert_eq!(out, "", "nothing should be written on error");
    }
}
