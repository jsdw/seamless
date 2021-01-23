//! A router implementation that can handle requests in a type safe way, while
//! also allowing information about the routes, route descriptions and expected
//! input and output types to be automatically generated from it.

use std::collections::HashMap;
use http::{ Request, Response };
use serde::{ Serialize, de::DeserializeOwned };
use super::error::{ IntoApiError, ApiError };
use super::body::{ ApiBody, ApiBodyType };
use std::pin::Pin;
use std::future::Future;
use async_trait::async_trait;

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
    resolved_handler: ResolvedHandler
}

// Internally we resolve the provided handler functions into this:
#[doc(hidden)]
pub struct ResolvedHandler {
    method: Method,
    handler: Box<dyn Fn(Request<Vec<u8>>) -> Fut<Result<Response<Vec<u8>>,ApiError>> + Send + Sync>,
    request_type: ApiBodyType,
    response_type: ApiBodyType
}

// A type alias for an overly complicated boxed Future type that can be sent across threads.
type Fut<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// does this route expect a GET or POST request. This is used internally to match on routes.
#[doc(hidden)]
#[derive(Hash,Serialize,Clone,Copy,PartialEq,Eq,Debug)]
pub enum Method {
    Get,
    Post,
    Unknown
}
impl From<&http::Method> for Method {
    fn from(other: &http::Method) -> Method {
        match other {
            &http::Method::GET => Method::Get,
            &http::Method::POST => Method::Post,
            _ => Method::Unknown
        }
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
    /// # use seamless::{ Api, Json };
    /// # use std::convert::Infallible;
    /// # let mut api = Api::new();
    /// // This route expects a JSON formatted string to be provided, and echoes it straight back.
    /// api.add("some/route/name")
    ///    .description("This route takes some Foo's in and returns some Bar's")
    ///    .handler(|body: Json<String>| async move { Ok::<_,std::convert::Infallible>(body.json) });
    ///
    /// // This route delegates to an async fn to sum some values, so we can infer more types in the handler.
    /// api.add("another.route")
    ///    .description("This route takes an array of values and sums them")
    ///    .handler(|body: Json<_>| sum(body.json));
    ///
    /// async fn sum(ns: Vec<u64>) -> Result<u64, Infallible> {
    ///     Ok(ns.into_iter().sum())
    /// }
    /// ```
    pub fn add<P: Into<String>>(&mut self, path: P) -> RouteBuilder {
        RouteBuilder::new(self, path.into())
    }

    // Add a route given the individual parts (for internal use)
    fn add_parts<A, P: Into<String>, Handler: ResolveHandler<A>>(&mut self, path: P, description: String, handler: Handler) {
        let resolved_handler = handler.resolve_handler();
        self.routes.insert((resolved_handler.method, path.into()), ResolvedApiRoute {
            description,
            resolved_handler
        });
    }

    /// Match an incoming [`http::Request`] against our API routes and run the relevant handler if a
    /// matching one is found. We'll get back bytes representing a JSON response back if all goes ok,
    /// else we'll get back a [`RouteError`], which will either be [`RouteError::NotFound`] if no matching
    /// route was found, or a [`RouteError::Err`] if a matching route was found, but that handler emitted
    /// an error.
    pub async fn handle(&self, req: Request<Vec<u8>>) -> Result<Response<Vec<u8>>,RouteError> {
        let base_path = &self.base_path.trim_start_matches('/');
        let req_path = req.uri().path().trim_start_matches('/');

        if req_path.starts_with(base_path) {
            // Ensure that the method and path suffix lines up as expected:
            let req_method = req.method().into();
            let req_path_tail = req_path[base_path.len()..].trim_start_matches('/').to_owned();
            if let Some(route) = self.routes.get(&(req_method,req_path_tail)) {
                (route.resolved_handler.handler)(req).await.map_err(RouteError::Err)
            } else {
                Err(RouteError::NotFound(req))
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
                method: val.resolved_handler.method,
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
/// # use seamless::{ Api, Json };
/// # use std::convert::Infallible;
/// # let mut api = Api::new();
/// // This route expects a JSON formatted string to be provided, and echoes it straight back.
/// api.add("some/route/name")
///    .description("This route takes some Foo's in and returns some Bar's")
///    .handler(|body: Json<String>| async move { Ok::<_,std::convert::Infallible>(body.json) });
///
/// // This route delegates to an async fn to sum some values, so we can infer more types in the handler.
/// api.add("another.route")
///    .description("This route takes an array of values and sums them")
///    .handler(|body: Json<_>| sum(body.json));
///
/// async fn sum(ns: Vec<u64>) -> Result<u64, Infallible> {
///     Ok(ns.into_iter().sum())
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
    pub fn handler<A, Handler: ResolveHandler<A>>(self, handler: Handler) {
        self.api.add_parts(self.path, self.description, handler);
    }
}

/// A route is either not found, or we attempted to run it and ran into
/// an issue.
#[derive(Debug)]
pub enum RouteError {
    /// No route matched the provided request,
    /// so we hand it back.
    NotFound(Request<Vec<u8>>),
    /// The matching route failed; this is the error.
    Err(ApiError)
}

impl RouteError {
    /// Assume that the `RouteError` contains an `ApiError` and attempt to
    /// unwrap this
    ///
    /// # Panics
    ///
    /// Panics if the RouteError does not contain an ApiError
    pub fn unwrap_api_error(self) -> ApiError {
        match self {
            RouteError::Err(e) => e,
            _ => panic!("Attempt to unwrap_api_err on RouteError that is NotFound")
        }
    }
}

/// Information about a single route.
#[derive(Debug,Clone,Serialize)]
pub struct RouteInfo {
    /// The name/path that the [`http::Request`] needs to contain
    /// in order to match this route.
    pub name: String,
    /// The HTTP method expected in order for a [`http::Request`] to
    /// match this route.
    pub method: Method,
    /// The description of the route as set by [`RouteBuilder::description()`]
    pub description: String,
    /// The shape of the data expected to be provided as part of the [`http::Request`]
    /// for this route. This doesn't care about the wire format that the data is provided in
    /// (be is JSON or other).
    ///
    /// If the handler function for the route asks for something of type [`Json<T>`], then
    /// the data will be expected to be provided as JSON. However, it is up to the type that
    /// implements [`Body`] to decide on the expected wire format.
    pub request_type: ApiBodyType,
    /// The shape of the data that is returned from this API route.
    pub response_type: ApiBodyType
}

/// This trait is implemented by anything that represents the incoming request type.
/// Only one argument implementing this can be asked for in a given handler. The type
/// that implements this is used to determine the input type expected by the handler
/// for the sake of generating API information.
pub trait Body: Sized {
    /// The type of the error returned if [`Self::get_body()`] fails.
    type Error: IntoApiError;
    /// Given a request containing arbitrary bytes, this function needs to return an
    /// instance of the type that this trait is implemented on (typically by deserializing
    /// it from the bytes provided), or else it should return an error describing what
    /// went wrong.
    fn get_body(req: Request<Vec<u8>>) -> Result<Self,Self::Error>;
}

/// If the last argument to the handler is this, we'll assume
/// that the user needs to provide JSON that decodes to `T`.
/// Notably, `T` needs to implement `ApiBody` with the
/// Deserialize option.
pub struct Json<T> {
    /// the type that has been deserialized from JSON.
    pub json: T
}
impl <T> Body for Json<T> where T: DeserializeOwned {
    type Error = ApiError;
    fn get_body(req: Request<Vec<u8>>) -> Result<Self,ApiError> {
        let body = req.into_body();
        let json = serde_json::from_slice(&body)
            .map_err(|e| ApiError {
                code: 400,
                internal_message: e.to_string(),
                external_message: e.to_string(),
                value: None
            })?;
        Ok(Json { json })
    }
}
impl <T> ApiBody for Json<T> where T: ApiBody {
    fn api_body_type() -> ApiBodyType {
        T::api_body_type()
    }
}

/// If the last argument to the handler is this, we'll assume
/// that the user can provide arbitrary binary data, and
/// we'll make that data available within the handler as bytes.
pub struct Binary {
    /// The bytes that were provided in the incoming [`http::Request`]
    pub bytes: Vec<u8>
}
impl Body for Binary {
    type Error = ApiError;
    fn get_body(req: Request<Vec<u8>>) -> Result<Self,ApiError> {
        let bytes = req.into_body();
        Ok(Binary { bytes })
    }
}
impl ApiBody for Binary {
    fn api_body_type() -> ApiBodyType {
        ApiBodyType {
            description: "Binary data".to_owned(),
            ty: super::body::Type::String
        }
    }
}

/// When a route matches, [`Self::get_context()`] is called for each
/// argument that is provided to the handler and implements this trait.
/// A successful result is passed to the handler function. An erroneous
/// result leads to the handler function not being called, and instead an
/// error being returned from [`Api::handle()`].
#[async_trait]
pub trait Context: Sized {
    /// The type of the error returned if [`Self::get_context()`] fails.
    type Error: IntoApiError;
    /// Given a [`http::Request<()>`], return a value of type `T` back, or
    /// else return an [`Self::Error`] describing what went wrong. Any errors
    /// here will lead to the route bailing out and the handler not being run.
    async fn get_context(req: &Request<()>) -> Result<Self,Self::Error>;
}

// Option<Body> means we'll return None to the handler if get_context would fail.
// This will never error.
#[async_trait]
impl <T: Context> Context for Option<T> {
    type Error = std::convert::Infallible;
    async fn get_context(req: &Request<()>) -> Result<Self,Self::Error> {
        Ok(T::get_context(req).await.ok())
    }
}

// Result<Context,Err> means we'll return the result of attempting to obtain the context.
// This will never error.
#[async_trait]
impl <T: Context> Context for Result<T,<T as Context>::Error> {
    type Error = std::convert::Infallible;
    async fn get_context(req: &Request<()>) -> Result<Self,Self::Error> {
        Ok(T::get_context(req).await)
    }
}

/// This trait is implemented for all handler functions which are
/// applicable. Handler functions expect context arguments first, and
/// then optionally an argument that implements `Body` (eg `Json` or
/// `Binary`) if the handler requires a body to be provided. Arguments
/// are resolved in the order that they are provided.
#[doc(hidden)]
pub trait ResolveHandler<A> {
    fn resolve_handler(self) -> ResolvedHandler;
}

// We want the handler functions to support different numbers of contexts as well
// as either having a body or not; this macro generates the impls for this.
macro_rules! resolve_for_contexts {
    ( $( $($ctx:ident)* ;)* ) => {

        // Markers to differentiate the trait impls:
        #[doc(hidden)]
        pub struct WithBody;
        #[doc(hidden)]
        pub struct WithoutBody;

        // Impl each trait with and without the body, and with each number of contexts provided:
        $(

        impl <Req, Res, Err, F, Handler $(, $ctx)*> ResolveHandler<(WithBody,Req,Res,Err,F $(, $ctx)*)> for Handler
        where
            Req: Body + ApiBody + Send,
            $( $ctx: Context + Send, )*
            Res: ApiBody + Serialize + 'static,
            Err: IntoApiError + 'static,
            F: Future<Output = Result<Res,Err>> + Send + 'static,
            Handler: Fn($($ctx,)* Req) -> F + Clone + Sync + Send + 'static
        {
            fn resolve_handler(self) -> ResolvedHandler {
                #[allow(unused_variables)]
                let handler = move |req: Request<Vec<u8>>| {
                    let inner_handler = self.clone();
                    async move {

                        let (parts, body) = req.into_parts();
                        let bodyless_req = Request::from_parts(parts, ());

                        $(
                        #[allow(non_snake_case)]
                        let $ctx = $ctx::get_context(&bodyless_req)
                            .await
                            .map_err(|e| e.into_api_error())?;
                        )*

                        let (parts, _) = bodyless_req.into_parts();
                        let req = Request::from_parts(parts, body);
                        let body = Req::get_body(req).map_err(|e| e.into_api_error())?;
                        let handler_res = inner_handler($($ctx,)* body)
                            .await
                            .map_err(|e| e.into_api_error())?;

                        let response = Response::builder()
                            .header("Content-Type", "application/json")
                            .body(handler_res.to_json_vec())
                            .unwrap();

                        Ok(response)
                    }
                };

                ResolvedHandler {
                    method: Method::Post,
                    handler: Box::new(move |req| Box::pin(handler(req))),
                    request_type: Req::api_body_type(),
                    response_type: Res::api_body_type()
                }
            }
        }

        impl <Res, Err, F, Handler $(, $ctx)*> ResolveHandler<(WithoutBody,Res,Err,F $(, $ctx)*)> for Handler
        where
            $( $ctx: Context + Send, )*
            Res: ApiBody + Serialize + 'static,
            Err: IntoApiError + 'static,
            F: Future<Output = Result<Res,Err>> + Send + 'static,
            Handler: Fn($($ctx),*) -> F + Clone + Sync + Send + 'static
        {
            fn resolve_handler(self) -> ResolvedHandler {
                #[allow(unused_variables)]
                let handler = move |req: Request<Vec<u8>>| {
                    let inner_handler = self.clone();
                    async move {

                        let (parts, body) = req.into_parts();
                        let bodyless_req = Request::from_parts(parts, ());

                        $(
                        #[allow(non_snake_case)]
                        let $ctx = $ctx::get_context(&bodyless_req)
                            .await
                            .map_err(|e| e.into_api_error())?;
                        )*

                        let handler_res = inner_handler($($ctx),*)
                            .await
                            .map_err(|e| e.into_api_error())?;

                        let response = Response::builder()
                            .header("Content-Type", "application/json")
                            .body(handler_res.to_json_vec())
                            .unwrap();

                        Ok(response)
                    }
                };

                ResolvedHandler {
                    method: Method::Get,
                    handler: Box::new(move |req| Box::pin(handler(req))),
                    request_type: ApiBodyType {
                        description: "No request body is expected".to_owned(),
                        ty: super::body::Type::Null
                    },
                    response_type: Res::api_body_type()
                }
            }
        }

    )*}
}

resolve_for_contexts!(
    ;
    C1;
    C1 C2;
    C1 C2 C3;
    C1 C2 C3 C4;
    C1 C2 C3 C4 C5;
    C1 C2 C3 C4 C5 C6;
);