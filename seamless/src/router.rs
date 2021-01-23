use std::collections::HashMap;
use http::{ Request, Response };
use serde::{ Serialize, de::DeserializeOwned };
use super::error::{ IntoApiError, ApiError };
use super::body::{ ApiBody, ApiBodyType };
use std::pin::Pin;
use std::future::Future;
use async_trait::async_trait;

/// The entry point; you can create an instance of this and then
/// add API routes to it using `api.add`, then get information about
/// those routes using `api.info`, or handle an incoming request
/// using `api.handle`.
pub struct Api {
    base_path: String,
    routes: HashMap<(Method,String),ResolvedApiRoute>
}

// An API route has the contents of `ResolvedHandler` but also a
// description.
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

type Fut<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// does this route expect a GET or POST request
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

    /// Instantiate a new API
    pub fn new() -> Api {
        Api::new_with_base_path("")
    }

    /// Instantiate a new API that will handle requests with the
    /// provided base path.
    pub fn new_with_base_path<S: Into<String>>(base_path: S) -> Api {
        Api {
            base_path: base_path.into(),
            routes: HashMap::new()
        }
    }

    /// Add a Route to the API
    pub fn add<P: Into<String>>(&mut self, path: P) -> RouteBuilder {
        RouteBuilder::new(self, path.into())
    }

    // Add a route given the individual parts (internal only)
    fn add_parts<A, P: Into<String>, Handler: ResolveHandler<A>>(&mut self, path: P, description: String, handler: Handler) {
        let resolved_handler = handler.resolve_handler();
        self.routes.insert((resolved_handler.method, path.into()), ResolvedApiRoute {
            description,
            resolved_handler
        });
    }

    /// Match a request against our API routes and run the relevant handler
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

    /// Return information for our current API routes
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
pub struct RouteBuilder<'a> {
    api: &'a mut Api,
    path: String,
    description: String
}
impl <'a> RouteBuilder<'a> {
    pub fn new(api: &'a mut Api, path: String) -> Self {
        RouteBuilder { api, path, description: String::new() }
    }
    pub fn description<S: Into<String>>(mut self, desc: S) -> Self {
        self.description = desc.into();
        self
    }
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

/// Information about a single route.
#[derive(Debug,Clone,Serialize)]
pub struct RouteInfo {
    pub name: String,
    pub method: Method,
    pub description: String,
    pub request_type: ApiBodyType,
    pub response_type: ApiBodyType
}

/// This trait is implemented by anything that represents the incoming request type
pub trait Body: Sized {
    type Error: IntoApiError;
    fn get_body(req: Request<Vec<u8>>) -> Result<Self,Self::Error>;
}

/// If the last argument to the handler is this, we'll assume
/// that the user needs to provide JSON that decodes to `T`.
/// Notably, `T` needs to implement `ApiBody` with the
/// Deserialize option.
pub struct Json<T> {
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

/// This trait can be implemented by types that are based on the
/// incoming request object (for example, currently logged in user).
/// doing so makes it possible for you to ask for those types in the
/// handler function you provide.
#[async_trait]
pub trait Context: Sized {
    type Error: IntoApiError;
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
        pub struct WithBody;
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