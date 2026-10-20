use crate::api::{ApiBody, ApiBodyInfo, ApiError};
use bytes::Bytes;
use futures::io::{AsyncRead, AsyncReadExt};
use futures::stream::{self, BoxStream, Stream, StreamExt};
use serde::Serialize;
use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

type HttpResponse = http::Response<Body>;

/// Anything that you'd like to be able to return from a handler function must implement
/// this trait, which decides how to take the result of a handler function and encode it
/// into an [`http::Response<Body>`], or failing to do so, returns an [`struct@ApiError`].
pub trait HandlerResponse {
    /// The type that we should look at to work out what the response body will look like.
    type ResponseBody: ApiBody;
    /// This describes how the type can be converted into an `http::Response`.
    fn handler_response(self) -> impl Future<Output = Result<HttpResponse, ApiError>> + Send;
}

/// The body of an [`http::Response`] handed back from [`crate::api::Api::handle()`].
///
/// A body is either a complete set of bytes that are already in memory, or a stream of
/// chunks of bytes which are handed back as they become available.
///
/// # Errors while streaming
///
/// By the time a streaming body is being read from, the response status and headers
/// will likely have been sent already, so any error that the stream emits cannot be turned
/// into a nice [`struct@ApiError`] response. Instead, the HTTP framework you're using will
/// typically abort the response. Anything that might fail in a way that the client should
/// be told about (eg a file not existing) should therefore be checked before a streaming
/// body is handed back.
pub struct Body(Inner);

enum Inner {
    Full(Bytes),
    Stream(BoxStream<'static, std::io::Result<Bytes>>),
}

impl std::fmt::Debug for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Inner::Full(bytes) => f.debug_tuple("Body::Full").field(bytes).finish(),
            Inner::Stream(_) => f.debug_tuple("Body::Stream").finish(),
        }
    }
}

impl Default for Body {
    fn default() -> Self {
        Body::empty()
    }
}

impl Body {
    /// An empty body.
    pub fn empty() -> Body {
        Body(Inner::Full(Bytes::new()))
    }

    /// Create a body from some bytes that are already in memory.
    pub fn from_bytes(bytes: impl Into<Bytes>) -> Body {
        Body(Inner::Full(bytes.into()))
    }

    /// Create a streaming body from a [`futures::Stream`] of chunks of bytes.
    pub fn from_stream<S, B>(stream: S) -> Body
    where
        S: Stream<Item = std::io::Result<B>> + Send + 'static,
        B: Into<Bytes>,
    {
        Body(Inner::Stream(stream.map(|r| r.map(Into::into)).boxed()))
    }

    /// Create a streaming body from something implementing [`futures::AsyncRead`].
    /// Bytes are read from it in chunks of up to 8KiB as the body is streamed.
    /// The stream ends after the reader returns an error.
    pub fn from_reader<R>(reader: R) -> Body
    where
        R: AsyncRead + Send + 'static,
    {
        const CHUNK_SIZE: usize = 8 * 1024;
        let state = Some((Box::pin(reader), vec![0u8; CHUNK_SIZE]));
        let stream = stream::unfold(state, |state| async move {
            let (mut reader, mut buf) = state?;
            match reader.read(&mut buf).await {
                Ok(0) => None,
                Ok(n) => {
                    let chunk = Bytes::copy_from_slice(&buf[..n]);
                    Some((Ok(chunk), Some((reader, buf))))
                }
                // Stop reading after an error by setting state to None:
                Err(e) => Some((Err(e), None)),
            }
        });
        Body(Inner::Stream(stream.boxed()))
    }

    /// If the body is entirely in memory already, this returns a reference to the bytes.
    /// Else it returns `None` if the body is a stream and thus not yet available.
    pub fn as_bytes(&self) -> Option<&Bytes> {
        match &self.0 {
            Inner::Full(bytes) => Some(bytes),
            Inner::Stream(_) => None,
        }
    }

    /// If the body is entirely in memory already, return it as [`Bytes`]. Otherwise,
    /// hand back the streaming body unchanged in the `Err` variant.
    pub fn into_bytes(self) -> Result<Bytes, Body> {
        match self.0 {
            Inner::Full(bytes) => Ok(bytes),
            inner @ Inner::Stream(_) => Err(Body(inner)),
        }
    }

    /// Convert the body into a [`BodyStream`], which implements [`futures::Stream`] and
    /// hands back chunks of bytes as they become available.
    pub fn into_stream(self) -> BodyStream {
        match self.0 {
            Inner::Full(bytes) => BodyStream(BodyStreamInner::Full(bytes)),
            Inner::Stream(stream) => BodyStream(BodyStreamInner::Stream(stream)),
        }
    }

    /// Read the entire body into a [`Vec<u8>`]. This is mostly useful for tests; prefer to
    /// stream bodies through to the client where possible.
    pub async fn into_vec(self) -> std::io::Result<Vec<u8>> {
        match self.0 {
            Inner::Full(bytes) => Ok(bytes.to_vec()),
            Inner::Stream(mut stream) => {
                let mut out = Vec::new();
                while let Some(chunk) = stream.next().await {
                    out.extend_from_slice(&chunk?);
                }
                Ok(out)
            }
        }
    }
}

/// A [`futures::Stream`] of chunks of bytes, obtained via [`Body::into_stream()`].
pub struct BodyStream(BodyStreamInner);

enum BodyStreamInner {
    // Empty once the bytes have been handed out (or if there were none to begin
    // with); we never hand back an empty chunk.
    Full(Bytes),
    Stream(BoxStream<'static, std::io::Result<Bytes>>),
}

impl std::fmt::Debug for BodyStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            BodyStreamInner::Full(bytes) => f.debug_tuple("BodyStream::Full").field(bytes).finish(),
            BodyStreamInner::Stream(_) => f.debug_tuple("BodyStream::Stream").finish(),
        }
    }
}

impl Stream for BodyStream {
    type Item = std::io::Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // BodyStream is Unpin, since both Bytes and BoxStream are.
        match &mut self.get_mut().0 {
            BodyStreamInner::Full(bytes) if bytes.is_empty() => Poll::Ready(None),
            // Taking leaves an empty `Bytes` behind, which doesn't allocate.
            BodyStreamInner::Full(bytes) => Poll::Ready(Some(Ok(std::mem::take(bytes)))),
            BodyStreamInner::Stream(stream) => stream.poll_next_unpin(cx),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            BodyStreamInner::Full(bytes) => {
                let n = usize::from(!bytes.is_empty());
                (n, Some(n))
            }
            BodyStreamInner::Stream(stream) => stream.size_hint(),
        }
    }
}

impl From<Vec<u8>> for Body {
    fn from(bytes: Vec<u8>) -> Self {
        Body::from_bytes(bytes)
    }
}

impl From<Bytes> for Body {
    fn from(bytes: Bytes) -> Self {
        Body::from_bytes(bytes)
    }
}

impl From<String> for Body {
    fn from(s: String) -> Self {
        Body::from_bytes(s)
    }
}

impl From<&'static [u8]> for Body {
    fn from(bytes: &'static [u8]) -> Self {
        Body::from_bytes(bytes)
    }
}

impl From<&'static str> for Body {
    fn from(s: &'static str) -> Self {
        Body::from_bytes(s)
    }
}

/// Wrap responses in this to return them as JSON
pub struct ToJson<T: ApiBody>(pub T);

impl<T: ApiBody + Serialize + Send> HandlerResponse for ToJson<T> {
    type ResponseBody = T;
    async fn handler_response(self) -> Result<HttpResponse, ApiError> {
        let body = serde_json::to_vec(&self.0).map_err(|e| {
            ApiError::server_error(format!("Could not serialize response to JSON: {e}"))
        })?;
        http::Response::builder()
            .header("content-type", "application/json")
            .body(Body::from(body))
            .map_err(|e| ApiError::server_error(format!("Could not build JSON response: {e}")))
    }
}

impl<T> ApiBody for ToJson<T>
where
    T: ApiBody,
{
    fn api_body_info() -> ApiBodyInfo {
        T::api_body_info()
    }
}

/// Return this from a handler to respond with arbitrary binary data, which can
/// optionally be streamed rather than being loaded into memory all at once.
///
/// The `content-type` defaults to `application/octet-stream`, and can be changed
/// with [`ToBinary::content_type()`].
///
/// See [`Body`] for a note on how errors that occur while streaming are handled.
///
/// # Examples
///
/// ```
/// # use seamless::{ Api, handler::response::ToBinary };
/// # let mut api = Api::new();
/// // Return some bytes that we already have:
/// api.add("hello")
///    .handler(|| ToBinary::new("hello world").content_type("text/plain"));
///
/// // Stream some bytes back without loading them all into memory first:
/// api.add("count")
///    .handler(|| {
///        let chunks = futures::stream::iter(1..=1000)
///            .map(|n| Ok::<_, std::io::Error>(format!("{n}\n")));
///        ToBinary::stream(chunks).content_type("text/plain")
///    });
/// # use futures::StreamExt;
/// ```
pub struct ToBinary {
    body: Body,
    content_type: Cow<'static, str>,
    content_length: Option<usize>,
}

impl std::fmt::Debug for ToBinary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToBinary")
            .field("body", &self.body)
            .field("content_type", &self.content_type)
            .field("content_length", &self.content_length)
            .finish()
    }
}

impl ToBinary {
    /// Respond with the given body. This can be anything which converts into a [`Body`],
    /// including a [`Body`] itself, `Vec<u8>`, [`Bytes`] or `String`.
    pub fn new(body: impl Into<Body>) -> ToBinary {
        let body = body.into();

        // If the body length is known up front, tell the client:
        let mut content_length = None;
        if let Some(bytes) = body.as_bytes() {
            content_length = Some(bytes.len());
        }

        ToBinary {
            body,
            content_type: Cow::Borrowed("application/octet-stream"),
            content_length,
        }
    }

    /// Respond with a stream of chunks of bytes. Shorthand for
    /// `ToBinary::new(Body::from_stream(stream))`.
    pub fn stream<S, B>(stream: S) -> ToBinary
    where
        S: Stream<Item = std::io::Result<B>> + Send + 'static,
        B: Into<Bytes>,
    {
        ToBinary::new(Body::from_stream(stream))
    }

    /// Respond with bytes read from some [`futures::AsyncRead`] implementation. Shorthand
    /// for `ToBinary::new(Body::from_reader(reader))`.
    pub fn reader<R>(reader: R) -> ToBinary
    where
        R: AsyncRead + Send + 'static,
    {
        ToBinary::new(Body::from_reader(reader))
    }

    /// Set the `content-type` header of the response.
    pub fn content_type(mut self, content_type: impl Into<Cow<'static, str>>) -> ToBinary {
        self.content_type = content_type.into();
        self
    }

    /// Set the `content-length` header of the response. This is useful when streaming
    /// something whose size is known ahead of time (eg a file), so that clients can
    /// report progress. It's up to you to ensure that the length is correct.
    pub fn content_length(mut self, len: usize) -> ToBinary {
        self.content_length = Some(len);
        self
    }
}

impl HandlerResponse for ToBinary {
    type ResponseBody = ToBinary;
    async fn handler_response(self) -> Result<HttpResponse, ApiError> {
        let mut res = http::Response::builder().header("content-type", &*self.content_type);
        if let Some(len) = self.content_length {
            res = res.header("content-length", len);
        }
        res.body(self.body)
            .map_err(|e| ApiError::server_error(format!("Could not build binary response: {e}")))
    }
}

impl ApiBody for ToBinary {
    fn api_body_info() -> ApiBodyInfo {
        ApiBodyInfo {
            description: "Binary data".to_owned(),
            ty: crate::api::ApiBodyType::Binary,
        }
    }
}

// Options are valid HandlerResponse's if their T's are
impl<T> HandlerResponse for Option<T>
where
    T: HandlerResponse + Send,
{
    type ResponseBody = <T as HandlerResponse>::ResponseBody;
    async fn handler_response(self) -> Result<HttpResponse, ApiError> {
        let res = self.ok_or_else(ApiError::path_not_found)?;
        res.handler_response().await
    }
}

// Results are valid HandlerResponse's if their T's are, and their E's convert to ApiError
impl<T, E> HandlerResponse for Result<T, E>
where
    T: HandlerResponse + Send,
    E: Into<ApiError> + Send + 'static,
{
    type ResponseBody = <T as HandlerResponse>::ResponseBody;
    async fn handler_response(self) -> Result<HttpResponse, ApiError> {
        let res = self.map_err(|e| e.into())?;
        res.handler_response().await
    }
}

#[cfg(test)]
mod test_body {
    use super::*;

    #[tokio::test]
    async fn to_json_serialize_error_is_500() {
        struct Bad;
        impl Serialize for Bad {
            fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("nope"))
            }
        }
        impl ApiBody for Bad {
            fn api_body_info() -> ApiBodyInfo {
                <String as ApiBody>::api_body_info()
            }
        }

        let err = ToJson(Bad)
            .handler_response()
            .await
            .expect_err("serialization failure should be an error, not a panic");
        assert_eq!(err.code, 500);
        assert_eq!(err.external_message, ApiError::SERVER_ERROR);
        assert!(err.internal_message.contains("nope"));
    }

    #[tokio::test]
    async fn full_body_is_bytes() {
        let body = Body::from(vec![1, 2, 3]);
        assert_eq!(body.as_bytes(), Some(&Bytes::from(vec![1, 2, 3])));
        assert_eq!(body.into_bytes().unwrap(), Bytes::from(vec![1, 2, 3]));

        let body = Body::empty();
        assert_eq!(body.as_bytes(), Some(&Bytes::new()));
        assert_eq!(body.into_bytes().unwrap(), Bytes::new());
    }

    #[tokio::test]
    async fn full_body_streams_once() {
        let chunks: Vec<_> = Body::from(vec![1, 2, 3])
            .into_stream()
            .map(|c| c.unwrap())
            .collect()
            .await;
        assert_eq!(chunks, vec![Bytes::from(vec![1, 2, 3])]);

        let chunks: Vec<_> = Body::empty().into_stream().collect().await;
        assert!(chunks.is_empty());
    }

    #[tokio::test]
    async fn full_body_stream_size_hint() {
        let mut stream = Body::from(vec![1, 2, 3]).into_stream();
        assert_eq!(stream.size_hint(), (1, Some(1)));
        assert!(stream.next().await.is_some());
        assert_eq!(stream.size_hint(), (0, Some(0)));
        assert!(stream.next().await.is_none());

        assert_eq!(Body::empty().into_stream().size_hint(), (0, Some(0)));
    }

    #[tokio::test]
    async fn stream_body_is_not_bytes() {
        let body = Body::from_stream(stream::iter(vec![Ok(vec![1]), Ok(vec![2, 3])]));
        assert!(body.as_bytes().is_none());
        let body = body
            .into_bytes()
            .expect_err("stream body should not be bytes");
        assert_eq!(body.into_vec().await.unwrap(), vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn stream_body_preserves_chunks() {
        let body = Body::from_stream(stream::iter(vec![Ok(vec![1]), Ok(vec![2, 3])]));
        let chunks: Vec<_> = body.into_stream().map(|c| c.unwrap()).collect().await;
        assert_eq!(chunks, vec![Bytes::from(vec![1]), Bytes::from(vec![2, 3])]);
    }

    #[tokio::test]
    async fn stream_body_propagates_errors() {
        let body = Body::from_stream(stream::iter(vec![
            Ok(vec![1]),
            Err(std::io::Error::other("boom")),
        ]));
        let err = body.into_vec().await.expect_err("error expected");
        assert_eq!(err.to_string(), "boom");
    }

    #[tokio::test]
    async fn reader_body_reads_everything() {
        // More than one chunk's worth of data:
        let data: Vec<u8> = (0..20_000u32).map(|n| n as u8).collect();
        let body = Body::from_reader(futures::io::Cursor::new(data.clone()));
        assert!(body.as_bytes().is_none());

        let body = Body::from_reader(futures::io::Cursor::new(data.clone()));
        assert_eq!(body.into_vec().await.unwrap(), data);
    }
}
