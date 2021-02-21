use crate::api::{ ApiBody, ApiBodyInfo, ApiError };
use async_trait::async_trait;
use serde::Serialize;

type HttpResponse = http::Response<Vec<u8>>;

/// Anything that you'd like to be able to return from a handler function must implement
/// this trait, which decides how to take the result of a handler function and encode it
/// into an [`http::Response<Vec<u8>>`], or failing to do so, returns an [`struct@ApiError`].
#[async_trait]
pub trait HandlerResponse {
    /// The type that we should look at to work out what the response body will look like.
    type ResponseBody: ApiBody;
    /// This describes how the type can be converted into an `http::Response`.
    async fn handler_response(self) -> Result<HttpResponse, ApiError>;
}

/// Wrap responses in this to return them as JSON
pub struct ToJson<T: ApiBody>(pub T);

#[async_trait]
impl <T: ApiBody + Serialize + Send> HandlerResponse for ToJson<T> {
    type ResponseBody = T;
    async fn handler_response(self) -> Result<HttpResponse, ApiError> {
        let body = serde_json::to_vec(&self.0).unwrap();
        let res = http::Response::builder()
            .header("content-type", "application/json")
            .body(body)
            .unwrap();
        Ok(res)
    }
}

impl <T> ApiBody for ToJson<T> where T: ApiBody {
    fn api_body_info() -> ApiBodyInfo {
        T::api_body_info()
    }
}

// Options are valid HandlerResponse's if their T's are
#[async_trait]
impl <T> HandlerResponse for Option<T>
where
    T: HandlerResponse + Send
{
    type ResponseBody = <T as HandlerResponse>::ResponseBody;
    async fn handler_response(self) -> Result<HttpResponse, ApiError> {
        let res = self.ok_or_else(|| ApiError::path_not_found())?;
        res.handler_response().await.map_err(|e| e.into())
    }
}

// Results are valid HandlerResponse's if their T's are, and their E's convert to ApiError
#[async_trait]
impl <T, E> HandlerResponse for Result<T,E>
where
    T: HandlerResponse + Send,
    E: Into<ApiError> + Send + 'static,
{
    type ResponseBody = <T as HandlerResponse>::ResponseBody;
    async fn handler_response(self) -> Result<HttpResponse, ApiError> {
        let res = self.map_err(|e| e.into())?;
        res.handler_response().await.map_err(|e| e.into())
    }
}

