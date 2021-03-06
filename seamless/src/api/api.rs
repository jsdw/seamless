use std::collections::HashMap;
use http::{ Request, Response, method::Method };
use serde::{ Serialize };
use super::info::{ ApiBodyInfo };
use super::error::ApiError;
use crate::handler::{ Handler, IntoHandler, request::AsyncReadBody };

/// The entry point; you can create an instance of this and then add API routes to it
/// using [`Self::add()`]. You can then get information about the routes that have been added
/// using [`Self::info()`], or handle an [`http::Request`] using [`Self::handle()`].
pub struct Api {
    base_path: String,
    routes: HashMap<(Method,String),ResolvedApiRoute>
}

// An API route has the contents of `ResolvedHandler` but also a description.
struct ResolvedApiRoute {
    description: String,
    resolved_handler: Handler
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
        Api {
            base_path: base_path.into(),
            routes: HashMap::new()
        }
    }

    /// Add a new route to the API. You must provide a path to make this route available at,
    /// and are given back a [`RouteBuilder`] which can be used to give the route a handler
    /// and a description.
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
    pub fn add<P: Into<String>>(&mut self, path: P) -> RouteBuilder {
        RouteBuilder::new(self, path.into())
    }

    // Add a route given the individual parts (for internal use)
    fn add_parts<A, P: Into<String>, HandlerFn: IntoHandler<A>>(&mut self, path: P, description: String, handler_fn: HandlerFn) {
        let resolved_handler = handler_fn.into_handler();
        let mut path: String = path.into();
        path = path.trim_matches('/').to_owned();
        self.routes.insert((resolved_handler.method.clone(), path.into()), ResolvedApiRoute {
            description,
            resolved_handler
        });
    }

    /// Match an incoming [`http::Request`] against our API routes and run the relevant handler if a
    /// matching one is found. We'll get back bytes representing a JSON response back if all goes ok,
    /// else we'll get back a [`RouteError`], which will either be [`RouteError::NotFound`] if no matching
    /// route was found, or a [`RouteError::Err`] if a matching route was found, but that handler emitted
    /// an error.
    pub async fn handle<Body: AsyncReadBody>(&self, req: Request<Body>) -> Result<Response<Vec<u8>>, RouteError<Body, ApiError>> {
        let base_path = &self.base_path.trim_start_matches('/');
        let req_path = req.uri().path().trim_start_matches('/');

        if req_path.starts_with(base_path) {
            // Ensure that the method and path suffix lines up as expected:
            let req_method = req.method().into();
            let req_path_tail = req_path[base_path.len()..].trim_start_matches('/').to_owned();

            // Turn req body into &mut dyn AsyncReadBody:
            let (req_parts, mut req_body) = req.into_parts();
            let dyn_req = Request::from_parts(req_parts, &mut req_body as &mut dyn AsyncReadBody);

            if let Some(route) = self.routes.get(&(req_method,req_path_tail)) {
                (route.resolved_handler.handler)(dyn_req).await.map_err(RouteError::Err)
            } else {
                let (req_parts, _) = dyn_req.into_parts();
                Err(RouteError::NotFound(Request::from_parts(req_parts, req_body)))
            }
        } else {
            Err(RouteError::NotFound(req))
        }
    }

    /// Return information about the API routes that have been defined so far.
    pub fn info(&self) -> Vec<RouteInfo> {
        let mut info = vec![];
        for ((_method,key), val) in &self.routes {
            info.push(RouteInfo {
                name: key.to_owned(),
                method: format!("{}", &val.resolved_handler.method),
                description: val.description.clone(),
                request_type: val.resolved_handler.request_type.clone(),
                response_type: val.resolved_handler.response_type.clone()
            });
        }
        info.sort_by(|a,b| a.name.cmp(&b.name));
        info
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
    description: String
}
impl <'a> RouteBuilder<'a> {
    fn new(api: &'a mut Api, path: String) -> Self {
        RouteBuilder { api, path, description: String::new() }
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
pub enum RouteError<B, E> {
    /// No route matched the provided request,
    /// so we hand it back.
    NotFound(Request<B>),
    /// The matching route failed; this is the error.
    Err(E)
}

impl <B, E: std::fmt::Debug> std::fmt::Debug for RouteError<B, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouteError::NotFound(..) => f.debug_tuple("RouteError::NotFound").finish(),
            RouteError::Err(e) => f.debug_tuple("RouteError::Err").field(e).finish()
        }
    }
}

impl <B, E> RouteError<B, E> {
    /// Assume that the `RouteError` contains an error and attempt to
    /// unwrap this
    ///
    /// # Panics
    ///
    /// Panics if the RouteError does not contain an error
    pub fn unwrap_err(self) -> E {
        match self {
            RouteError::Err(e) => e,
            _ => panic!("Attempt to unwrap_api_err on RouteError that is NotFound")
        }
    }
}

/// Information about a single route.
#[derive(Debug,Clone,PartialEq,Serialize)]
pub struct RouteInfo {
    /// The name/path that the [`http::Request`] needs to contain
    /// in order to match this route.
    pub name: String,
    /// The HTTP method expected in order for a [`http::Request`] to
    /// match this route, as a string.
    pub method: String,
    /// The description of the route as set by [`RouteBuilder::description()`]
    pub description: String,
    /// The shape of the data expected to be provided as part of the [`http::Request`]
    /// for this route. This doesn't care about the wire format that the data is provided in,
    /// though the type information is somewhat related to what the possible types that can
    /// be sent and received via JSON.
    ///
    /// Types can use the [`macro@crate::ApiBody`] macro, or implement [`type@crate::api::ApiBody`]
    /// manually in order to describe the shape and documentation that they should hand back.
    pub request_type: ApiBodyInfo,
    /// The shape of the data that is returned from this API route.
    pub response_type: ApiBodyInfo
}