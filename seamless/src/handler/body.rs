use http::{ Request, method::Method };
use serde::{ de::DeserializeOwned };
use crate::api::{ ApiBody, ApiBodyInfo, ApiError };
use crate::stream::{ AsyncReadBody };
use async_trait::async_trait;
use futures::{ AsyncReadExt };

/// This trait is implemented by anything that represents the incoming request type.
/// Only one argument implementing this can be asked for in a given handler. The type
/// that implements this is used to determine the input type expected by the handler
/// for the sake of generating API information.
#[async_trait]
pub trait HandlerBody: Sized {
    /// Given a request containing arbitrary bytes, this function needs to return an
    /// instance of the type that this trait is implemented on (typically by deserializing
    /// it from the bytes provided), or else it should return an error describing what
    /// went wrong.
    async fn handler_body(req: Request<&mut dyn AsyncReadBody>) -> Result<Self,ApiError>;
    /// Which HTTP method is required for this Body to be valid. By default, if a body
    /// is present in the handler we'll expect the method to be POST. Implement this function
    /// to override that.
    fn handler_method() -> Method { Method::POST }
}

/// If the last argument to a handler is this, we'll assume
/// that the user needs to provide JSON that decodes to `T`.
/// Notably, `T` needs to implement `ApiBody` with the 
/// Deserialize option.
pub struct FromJson<T: ApiBody>(pub T);
#[async_trait]
impl <T: DeserializeOwned + ApiBody> HandlerBody for FromJson<T> {
    async fn handler_body(req: Request<&mut dyn AsyncReadBody>) -> Result<Self,ApiError> {
        let content_type = req.headers()
            .get(http::header::CONTENT_TYPE)
            .ok_or_else(content_type_not_json_err)?;
        let content_type_is_json = content_type
            .to_str()
            .map(|s| s.to_ascii_lowercase() == "application/json")
            .unwrap_or(false);
        if !content_type_is_json {
            return Err(content_type_not_json_err())
        }

        // Stream our body into a vector of bytes:
        let mut body = vec![];
        req.into_body().read_to_end(&mut body).await
            .map_err(|e| ApiError {
                code: 400,
                internal_message: e.to_string(),
                external_message: e.to_string(),
                value: None
            })?;

        // Assume JSON and parse:
        let json = serde_json::from_slice(&body)
            .map_err(|e| ApiError {
                code: 400,
                internal_message: e.to_string(),
                external_message: e.to_string(),
                value: None
            })?;
        Ok(FromJson(json))
    }
}
impl <T> ApiBody for FromJson<T> where T: ApiBody {
    fn api_body_info() -> ApiBodyInfo {
        T::api_body_info()
    }
}

fn content_type_not_json_err() -> ApiError {
    ApiError {
        code: 415,
        internal_message: "Content-Type must be application/json".to_string(),
        external_message: "Content-Type must be application/json".to_string(),
        value: None
    }
}

/// If the last argument to a handler is this, we'll assume
/// that the user can provide arbitrary binary data, and
/// we'll make that data available within the handler as bytes.
pub struct FromBinary(pub Vec<u8>);
#[async_trait]
impl HandlerBody for FromBinary {
    async fn handler_body(req: Request<&mut dyn AsyncReadBody>) -> Result<Self,ApiError> {
        let mut body = vec![];
        req.into_body().read_to_end(&mut body).await
            .map_err(|e| ApiError {
                code: 400,
                internal_message: e.to_string(),
                external_message: e.to_string(),
                value: None
            })?;
        Ok(FromBinary(body))
    }
}
impl ApiBody for FromBinary {
    fn api_body_info() -> ApiBodyInfo {
        ApiBodyInfo {
            description: "Binary data".to_owned(),
            ty: crate::api::ApiBodyType::String
        }
    }
}