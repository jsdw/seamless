use http::{ Request, method::Method };
use serde::{ de::DeserializeOwned };
use crate::api::{ ApiBody, ApiBodyInfo, ApiError };
use async_trait::async_trait;

/// This trait is implemented by anything that represents the incoming request type.
/// Only one argument implementing this can be asked for in a given handler. The type
/// that implements this is used to determine the input type expected by the handler
/// for the sake of generating API information.
#[async_trait]
pub trait HandlerBody: Sized {
    /// An error indicating what went wrong in the event that we fail to extract
    /// our body from the provided request.
    type Error: Into<ApiError> + 'static;
    /// Given a request containing arbitrary bytes, this function needs to return an
    /// instance of the type that this trait is implemented on (typically by deserializing
    /// it from the bytes provided), or else it should return an error describing what
    /// went wrong.
    async fn handler_body(req: Request<Vec<u8>>) -> Result<Self,Self::Error>;
    /// Which HTTP method is required for this Body to be valid. By default, if a body
    /// is present in the handler we'll expect the method to be POST. Implement this function
    /// to override that.
    fn handler_method() -> Method { Method::POST }
}

/// If the last argument to a handler is this, we'll assume
/// that the user needs to provide JSON that decodes to `T`.
/// Notably, `T` needs to implement `ApiBody` with the
/// Deserialize option.
pub struct Json<T> {
    /// the type that has been deserialized from JSON.
    pub json: T
}
#[async_trait]
impl <T> HandlerBody for Json<T> where T: DeserializeOwned {
    type Error = ApiError;
    async fn handler_body(req: Request<Vec<u8>>) -> Result<Self,ApiError> {
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
    fn api_body_info() -> ApiBodyInfo {
        T::api_body_info()
    }
}

/// If the last argument to a handler is this, we'll assume
/// that the user can provide arbitrary binary data, and
/// we'll make that data available within the handler as bytes.
pub struct Binary {
    /// The bytes that were provided in the incoming [`http::Request`]
    pub bytes: Vec<u8>
}
#[async_trait]
impl HandlerBody for Binary {
    type Error = ApiError;
    async fn handler_body(req: Request<Vec<u8>>) -> Result<Self,ApiError> {
        let bytes = req.into_body();
        Ok(Binary { bytes })
    }
}
impl ApiBody for Binary {
    fn api_body_info() -> ApiBodyInfo {
        ApiBodyInfo {
            description: "Binary data".to_owned(),
            ty: crate::api::ApiBodyType::String
        }
    }
}