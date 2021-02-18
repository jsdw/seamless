//! This module describes how various handler output types can be converted into
//! a valid response, allowing various types to be used. Responses can be async or
//! sync, but they must be wrapped in either a `Result` or `Option` (impls of traits
//! not wrapped in something would conflict with those that do, so we don't implement
//! those).

use std::future::Future;
use std::pin::Pin;
use crate::api::ApiError;

// A type alias for an overly complicated boxed Future type that can be sent across threads.
type Fut<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub trait HandlerResponse<Res,Err,A>
where Err: Into<ApiError>
{
    type Output: Future<Output=Result<Res,Err>> + Send + 'static;
    fn handler_response(self) -> Self::Output;
}

// Handle Async responses:
#[doc(hidden)]
pub struct Async;
impl <Res, Err, F, T> HandlerResponse<Res,Err,Async> for F
where
  F: Future<Output=T> + Send + 'static,
  T: HandlerResponseWrapper<Res,Err> + Send + 'static,
  Res: 'static,
  Err: Into<ApiError> + 'static
{
    type Output = Fut<Result<Res,Err>>;
    fn handler_response(self) -> Self::Output {
        Box::pin(async move { self.await.wrap_response() })
    }
}

// handle sync responses:
#[doc(hidden)]
pub struct Sync;
impl <Res, Err, T> HandlerResponse<Res,Err,Sync> for T
where
    T: HandlerResponseWrapper<Res,Err> + Send + 'static,
    Res: 'static,
    Err: Into<ApiError> + 'static
{
    type Output = Fut<Result<Res,Err>>;
    fn handler_response(self) -> Self::Output {
        Box::pin(async move { self.wrap_response() })
    }
}

// Describe how the response can be converted into our standard shape:
pub trait HandlerResponseWrapper<T,E> {
    fn wrap_response(self) -> Result<T,E>;
}
impl <T,E> HandlerResponseWrapper<T,E> for Result<T,E> {
    fn wrap_response(self) -> Result<T,E> {
        self
    }
}
impl <T> HandlerResponseWrapper<T,ApiError> for Option<T> {
    fn wrap_response(self) -> Result<T,ApiError> {
        self.ok_or_else(|| ApiError::path_not_found())
    }
}
