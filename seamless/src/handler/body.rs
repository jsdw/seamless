use crate::api::{ApiBody, ApiBodyInfo, ApiBodyType, ApiError};
use crate::handler::request::{AsyncReadBody, CappedAsyncRead, CappedSizeLimitError};
use futures::AsyncReadExt;
use http::{Method, Request};
use serde::de::DeserializeOwned;
use std::future::Future;
use std::ops::{Deref, DerefMut};

/// This trait is implemented by anything that represents the incoming request type.
/// Only one argument implementing this can be asked for in a given handler. The type
/// that implements this is used to determine the input type expected by the handler
/// for the sake of generating API information.
pub trait HandlerBody: Sized {
    /// Given a request containing arbitrary bytes, this function needs to return an
    /// instance of the type that this trait is implemented on (typically by deserializing
    /// it from the bytes provided), or else it should return an error describing what
    /// went wrong.
    fn handler_body(req: Request<&mut dyn AsyncReadBody>) -> impl Future<Output = Result<Self, ApiError>> + Send;

    /// Which HTTP method is required for this Body to be valid. The built-in body types
    /// ([`FromJson`] and [`FromBinary`]) expect POST, and wrapper types such as [`Capped`]
    /// forward the method of the type they wrap.
    fn handler_method() -> Method;
}

/// A simple trait that makes it a little more ergonomic in some cases to extract the body
/// out of our various types like [`FromJson`] and [`FromBinary`]. Useful when we combine
/// types like [`Capped`] so that we can avoid multiple layers of unwrapping.
pub trait IntoBody {
    /// The body type that will be extracted. Whatever we extract should itself implement
    /// [`trait@ApiBody`] to correspond to the generated type information.
    type Target: ApiBody;

    /// Extract the body of the request with this, unwrapping as many layers of nesting as
    /// needed.
    fn into_body(self) -> Self::Target;
}

/// If the last argument to a handler is this, we'll assume
/// that the user needs to provide JSON that decodes to `T`.
/// Notably, `T` needs to implement `ApiBody` with the
/// Deserialize option.
pub struct FromJson<T>(pub T);

impl<T: DeserializeOwned + ApiBody> HandlerBody for FromJson<T> {
    async fn handler_body(req: Request<&mut dyn AsyncReadBody>) -> Result<Self, ApiError> {
        let is_json = req
            .headers()
            .get(http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(is_json_content_type);
        if !is_json {
            return Err(ApiError::external(415, "Content-Type must be application/json"));
        }

        // Stream our body into a vector of bytes:
        let mut body = vec![];
        req.into_body()
            .read_to_end(&mut body)
            .await
            .map_err(io_error_to_api_error)?;

        // Assume JSON and parse:
        let json = serde_json::from_slice(&body).map_err(|e| ApiError::external(400, e.to_string()))?;
        Ok(FromJson(json))
    }

    fn handler_method() -> Method {
        Method::POST
    }
}

impl<T> ApiBody for FromJson<T>
where
    T: ApiBody,
{
    fn api_body_info() -> ApiBodyInfo {
        T::api_body_info()
    }
}

impl<T> Deref for FromJson<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for FromJson<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: ApiBody> IntoBody for FromJson<T> {
    type Target = T;
    fn into_body(self) -> Self::Target {
        self.0
    }
}

/// True if a Content-Type header value names application/json,
/// ignoring parameters such as `; charset=utf-8`.
fn is_json_content_type(value: &str) -> bool {
    let media_type = value.split(';').next().unwrap_or("").trim();
    media_type.eq_ignore_ascii_case("application/json")
}

/// If the last argument to a handler is this, we'll assume
/// that the user can provide arbitrary binary data, and
/// we'll make that data available within the handler as bytes.
pub struct FromBinary(pub Vec<u8>);

impl HandlerBody for FromBinary {
    async fn handler_body(req: Request<&mut dyn AsyncReadBody>) -> Result<Self, ApiError> {
        let mut body = vec![];
        req.into_body()
            .read_to_end(&mut body)
            .await
            .map_err(io_error_to_api_error)?;
        Ok(FromBinary(body))
    }

    fn handler_method() -> Method {
        Method::POST
    }
}

impl ApiBody for FromBinary {
    fn api_body_info() -> ApiBodyInfo {
        ApiBodyInfo {
            description: "Binary data".to_owned(),
            ty: ApiBodyType::Binary,
        }
    }
}

impl Deref for FromBinary {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FromBinary {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<FromBinary> for Vec<u8> {
    fn from(b: FromBinary) -> Self {
        b.0
    }
}

impl IntoBody for FromBinary {
    type Target = Vec<u8>;
    fn into_body(self) -> Self::Target {
        self.0
    }
}

/// This wraps anything implementing [`HandlerBody`] and puts a type level cap on the size
/// that the request body is allowed to be before this is rejected. This works best when the
/// request body is streamed, as it will stop the streaming once said limit is reached.
pub struct Capped<T, const MAX_BYTES: usize>(pub T);

impl<T: HandlerBody, const MAX_BYTES: usize> HandlerBody for Capped<T, MAX_BYTES> {
    async fn handler_body(req: Request<&mut dyn AsyncReadBody>) -> Result<Self, ApiError> {
        let (parts, body) = req.into_parts();
        let mut body = CappedAsyncRead::<_, MAX_BYTES>::new(body);
        let req = Request::from_parts(parts, &mut body as &mut dyn AsyncReadBody);
        T::handler_body(req).await.map(Capped)
    }

    fn handler_method() -> Method {
        T::handler_method()
    }
}

impl<T: ApiBody, const MAX_BYTES: usize> ApiBody for Capped<T, MAX_BYTES> {
    fn api_body_info() -> ApiBodyInfo {
        T::api_body_info()
    }
}

impl<T, const MAX_BYTES: usize> Deref for Capped<T, MAX_BYTES> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, const MAX_BYTES: usize> DerefMut for Capped<T, MAX_BYTES> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: IntoBody, const MAX_BYTES: usize> IntoBody for Capped<T, MAX_BYTES> {
    type Target = T::Target;
    fn into_body(self) -> Self::Target {
        self.0.into_body()
    }
}

/// Map IO errors from reading HTTP bodies into suitable ApiErrors.
fn io_error_to_api_error(e: std::io::Error) -> ApiError {
    match e.downcast::<CappedSizeLimitError>() {
        Ok(e) => ApiError::external(413, e.to_string()),
        Err(e) => ApiError::external(400, e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct PutBody;

    impl HandlerBody for PutBody {
        async fn handler_body(_req: Request<&mut dyn AsyncReadBody>) -> Result<Self, ApiError> {
            Ok(PutBody)
        }
        fn handler_method() -> Method {
            Method::PUT
        }
    }

    impl ApiBody for PutBody {
        fn api_body_info() -> ApiBodyInfo {
            FromBinary::api_body_info()
        }
    }

    #[test]
    fn capped_forwards_handler_method() {
        assert_eq!(PutBody::handler_method(), Method::PUT);
        assert_eq!(<Capped<PutBody, 1024> as HandlerBody>::handler_method(), Method::PUT);
        assert_eq!(<Capped<FromBinary, 1024> as HandlerBody>::handler_method(), Method::POST);
        assert_eq!(<Capped<FromJson<String>, 1024> as HandlerBody>::handler_method(), Method::POST);
    }

    #[tokio::test]
    async fn capped_returns_413_when_body_too_large() {
        use crate::handler::request::Bytes;

        // Oversized binary body:
        let mut body = Bytes::from_vec(vec![0u8; 11]);
        let req = Request::post("/")
            .body(&mut body as &mut dyn AsyncReadBody)
            .unwrap();
        let err = <Capped<FromBinary, 10> as HandlerBody>::handler_body(req)
            .await
            .err()
            .expect("body exceeding the cap should be rejected");
        assert_eq!(err.code, 413);
        assert_eq!(err.external_message, "Size limit exceeded");

        // Oversized JSON body:
        let mut body = Bytes::from_vec(br#""hello world""#.to_vec());
        let req = Request::post("/")
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(&mut body as &mut dyn AsyncReadBody)
            .unwrap();
        let err = <Capped<FromJson<String>, 10> as HandlerBody>::handler_body(req)
            .await
            .err()
            .expect("body exceeding the cap should be rejected");
        assert_eq!(err.code, 413);
        assert_eq!(err.external_message, "Size limit exceeded");

        // A body exactly at the cap is fine:
        let mut body = Bytes::from_vec(vec![1u8; 10]);
        let req = Request::post("/")
            .body(&mut body as &mut dyn AsyncReadBody)
            .unwrap();
        let res = <Capped<FromBinary, 10> as HandlerBody>::handler_body(req)
            .await
            .unwrap_or_else(|e| panic!("body within the cap should be accepted: {}", e.internal_message));
        assert_eq!(res.0.0, vec![1u8; 10]);
    }

    #[test]
    fn json_content_type_matching() {
        assert!(is_json_content_type("application/json"));
        assert!(is_json_content_type("Application/JSON"));
        assert!(is_json_content_type("application/json; charset=utf-8"));
        assert!(is_json_content_type("application/json ; charset=UTF-8"));
        assert!(!is_json_content_type("text/plain"));
        assert!(!is_json_content_type("application/jsonx"));
        assert!(!is_json_content_type(""));
    }
}
