use super::error::ApiError;
use crate::handler::{
    Handler, IntoHandler, request::AsyncReadBody, response::Body as ResponseBody,
};
use http::{Method, Request, Response};
use seamless_info::RouteInfo;
use std::collections::HashMap;

/// The entry point; you can create an instance of this and then add API routes to it
/// using [`Self::add()`]. You can then get information about the routes that have been added
/// using [`Self::info()`], or handle an [`http::Request`] using [`Self::handle()`].
pub struct Api {
    base_path: String,
    routes: HashMap<(Method, String), Route>,
}

// A route's handler, along with its description.
struct Route {
    description: String,
    handler: Handler,
}

impl Default for Api {
    fn default() -> Self {
        Api::new()
    }
}

impl Api {
    /// Instantiate a new API.
    pub fn new() -> Api {
        Api::new_with_base_path("")
    }

    /// Instantiate a new API that will handle requests that begin with the
    /// provided base path.
    ///
    /// For example, if the provided `base_path` is "/foo/bar", and a route with
    /// the path "hi" is added, then an incoming [`http::Request`] with the path
    /// `"/foo/bar/hi"` will match it.
    pub fn new_with_base_path<S: Into<String>>(base_path: S) -> Api {
        // Store without leading or trailing slashes so that it's ready to
        // be matched against request paths in `handle`.
        let base_path = base_path.into().trim_matches('/').to_owned();
        Api {
            base_path,
            routes: HashMap::new(),
        }
    }

    /// Add a new route to the API. You must provide a path to make this route available at,
    /// and are given back a [`RouteBuilder`] which can be used to give the route a handler
    /// and a description.
    ///
    /// If you add a route at the same path and requiring the same method as an existing route,
    /// the existing route will be silently overwritten.
    ///
    /// Leading and trailing slashes are ignored, so `"/foo/"` is the same path as `"foo"`.
    ///
    /// # Panics
    ///
    /// Panics if the path is empty, or consists only of slashes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use seamless::{ Api, handler::{ body::FromJson, response::ToJson } };
    /// # use std::convert::Infallible;
    /// # let mut api = Api::new();
    /// // This route expects a JSON formatted string to be provided, and echoes it straight back.
    /// api.add("some/route/name")
    ///    .description("This route takes some Foo's in and returns some Bar's")
    ///    .handler(|body: FromJson<String>| ToJson(body.0));
    ///
    /// // This route delegates to an async fn to sum some values, so we can infer more types in the
    /// // handler.
    /// api.add("another.route")
    ///    .description("This route takes an array of values and sums them")
    ///    .handler(|body: FromJson<_>| sum(body.0));
    ///
    /// async fn sum(ns: Vec<u64>) -> ToJson<u64> {
    ///     ToJson(ns.into_iter().sum())
    /// }
    /// ```
    pub fn add<P: Into<String>>(&mut self, path: P) -> RouteBuilder<'_> {
        let path = path.into().trim_matches('/').to_owned();
        assert!(!path.is_empty(), "Routes cannot be added at an empty path");
        RouteBuilder::new(self, path)
    }

    // Add a route given the individual parts (for internal use)
    fn add_parts<A, HandlerFn: IntoHandler<A>>(
        &mut self,
        path: String,
        description: String,
        handler_fn: HandlerFn,
    ) {
        let handler = handler_fn.into_handler();
        self.routes.insert(
            (handler.method.clone(), path),
            Route {
                description,
                handler,
            },
        );
    }

    /// Match an incoming [`http::Request`] against our API routes and run the relevant handler if a
    /// matching one is found. We'll get back an [`http::Response`] whose body is a
    /// [`crate::handler::response::Body`] if all goes ok, else we'll get back a [`RouteError`],
    /// which will either be [`RouteError::NotFound`] if no matching route was found, or a
    /// [`RouteError::Err`] if a matching route was found, but that handler emitted an error.
    #[allow(clippy::result_large_err)]
    pub async fn handle<Body: AsyncReadBody>(
        &self,
        req: Request<Body>,
    ) -> Result<Response<ResponseBody>, RouteError<Body, ApiError>> {
        let Some(route) = self.find_route(&req) else {
            return Err(RouteError::NotFound(req));
        };

        // Handlers take the body as a `&mut dyn AsyncReadBody`:
        let (parts, mut body) = req.into_parts();
        let req = Request::from_parts(parts, &mut body as &mut dyn AsyncReadBody);

        route.handler.call(req).await.map_err(RouteError::Err)
    }

    /// Find the route matching the request's method and path, if there is one.
    fn find_route<Body>(&self, req: &Request<Body>) -> Option<&Route> {
        let path = req.uri().path().trim_start_matches('/');
        let path = strip_path_prefix(path, &self.base_path)?.trim_start_matches('/');
        self.routes.get(&(req.method().clone(), path.to_owned()))
    }

    /// Return information about the API routes that have been defined so far.
    /// Routes are currently sorted by name and then by method, but this order may change.
    pub fn info(&self) -> Vec<RouteInfo> {
        let mut info: Vec<RouteInfo> = self
            .routes
            .iter()
            .map(|((method, path), route)| RouteInfo {
                name: path.clone(),
                method: method.to_string(),
                description: route.description.clone(),
                request_type: route.handler.request_type.clone(),
                response_type: route.handler.response_type.clone(),
            })
            .collect();
        // Sort by name, then by method, so that routes sharing a path come out
        // in a stable order rather than in `HashMap` iteration order.
        info.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.method.cmp(&b.method)));
        info
    }
}

/// Strip `prefix` from the start of `path`, but only if it matches whole path
/// segments. Neither argument should have a leading slash, and `prefix` should
/// have no trailing slash. For example, with a prefix of `"api"`, the paths
/// `"api"` and `"api/foo"` match, but `"apiv2/foo"` does not.
fn strip_path_prefix<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    if prefix.is_empty() {
        return Some(path);
    }
    let tail = path.strip_prefix(prefix)?;
    if tail.is_empty() || tail.starts_with('/') {
        Some(tail)
    } else {
        None
    }
}

/// Add a new API route by providing a description (optional but encouraged)
/// and then a handler function.
///
/// # Examples
///
/// ```
/// # use seamless::{ Api, handler::{ body::FromJson, response::ToJson } };
/// # use std::convert::Infallible;
/// # let mut api = Api::new();
/// // This route expects a JSON formatted string to be provided, and echoes it straight back.
/// api.add("echo")
///    .description("Echo back the JSON encoded string provided")
///    .handler(|body: FromJson<String>| ToJson(body.0));
///
/// // This route delegates to an async fn to sum some values, so we can infer more types in the handler.
/// api.add("another.route")
///    .description("This route takes an array of values and sums them")
///    .handler(|FromJson(body)| sum(body));
///
/// async fn sum(ns: Vec<u64>) -> Result<ToJson<u64>, Infallible> {
///     Ok(ToJson(ns.into_iter().sum()))
/// }
/// ```
pub struct RouteBuilder<'a> {
    api: &'a mut Api,
    path: String,
    description: String,
}

impl<'a> RouteBuilder<'a> {
    fn new(api: &'a mut Api, path: String) -> Self {
        RouteBuilder {
            api,
            path,
            description: String::new(),
        }
    }

    /// Add a description to the API route.
    pub fn description<S: Into<String>>(mut self, desc: S) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a handler to the API route. Until this has been added, the route
    /// doesn't "exist".
    pub fn handler<A, HandlerFn: IntoHandler<A>>(self, handler: HandlerFn) {
        self.api.add_parts(self.path, self.description, handler);
    }
}

/// A route is either not found, or we attempted to run it and ran into
/// an issue.
#[allow(clippy::large_enum_variant)]
pub enum RouteError<B, E> {
    /// No route matched the provided request,
    /// so we hand it back.
    NotFound(Request<B>),
    /// The matching route failed; this is the error.
    Err(E),
}

impl<B, E: std::fmt::Debug> std::fmt::Debug for RouteError<B, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouteError::NotFound(..) => f.debug_tuple("RouteError::NotFound").finish(),
            RouteError::Err(e) => f.debug_tuple("RouteError::Err").field(e).finish(),
        }
    }
}

impl<B, E> RouteError<B, E> {
    /// Assume that the `RouteError` contains an error and attempt to
    /// unwrap it.
    ///
    /// # Panics
    ///
    /// Panics if the RouteError does not contain an error
    pub fn unwrap_err(self) -> E {
        match self {
            RouteError::Err(e) => e,
            _ => panic!("Attempt to unwrap_err on RouteError that is NotFound"),
        }
    }
}
