/*!
This module provides some functionality for creating a valid request body to send to Seamless as
part of an [`http::Request`]. Essentially, anything implementing `AsyncRead + Send + Unpin` can be
provided as a request body, but this module provides a `Bytes` struct which can easily convert a
[`Vec<u8>`] or [`futures::Stream`] into such a body.
*/
use futures::{ Stream, TryStreamExt, io::{ AsyncRead, Cursor } };
use std::pin::Pin;
use std::task::{ Poll, Context };

/// Any valid body will implement this trait
pub trait AsyncReadBody: AsyncRead + Send + Unpin {}
impl <T: AsyncRead + Send + Unpin> AsyncReadBody for T {}

/// A collection of bytes that can be read from asynchronously via the
/// [`futures::AsyncRead`] trait.
pub struct Bytes {
    variant: BytesVariant
}

/// To avoid an allocation around a vector of bytes, we use an enum which
/// we'll match on instead, and only box things when we need to.
enum BytesVariant {
    FromVec(Cursor<Vec<u8>>),
    FromReader(Box<dyn AsyncReadBody>)
}

impl std::fmt::Debug for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ByteStream").finish()
    }
}

impl AsyncRead for Bytes {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        // If Bytes is pinned, the enum variant is also pinned, so here
        // we just want to convert a Pin<&mut Bytes> into the relevant variant
        // to poll it. Very much the same sort of idea as:
        // 
        // https://github.com/rust-lang/futures-rs/blob/0.3.0-alpha.12/futures-core/src/stream/mod.rs#L88-L102
        unsafe {
            match &mut Pin::get_unchecked_mut(self).variant {
                BytesVariant::FromVec(v) => {
                    Pin::new_unchecked(v).poll_read(cx, buf)
                },
                BytesVariant::FromReader(r) => {
                    Pin::new_unchecked(r).poll_read(cx, buf)
                } 
            }
        }
    }
}

// For simple cases, it's easy to create this struct via `bytes.into()` so that one
// doesn't have to import anything from this file. Prefer to stream, though.
impl From<Vec<u8>> for Bytes {
    fn from(bytes: Vec<u8>) -> Self {
        Bytes::from_vec(bytes)
    } 
}

impl Bytes {
    /// Turn a vector of bytes into a [`Bytes`]. Prefer to stream where possible.
    pub fn from_vec(bytes: Vec<u8>) -> Bytes {
        Bytes { 
            variant: BytesVariant::FromVec(Cursor::new(bytes)) 
        }
    }
    /// Turn a thing implementing [`futures::AsyncRead`] into [`Bytes`].
    pub fn from_reader<S: AsyncReadBody + 'static>(reader: S) -> Bytes {
        Bytes { 
            variant: BytesVariant::FromReader(Box::new(reader)) 
        }
    }
    /// Turn a thing implementing [`futures::Stream`] into [`Bytes`].
    pub fn from_stream<S: Stream<Item = std::io::Result<Vec<u8>>> + 'static + Send + Unpin>(stream: S) -> Bytes {
        Bytes { 
            variant: BytesVariant::FromReader(Box::new(stream.into_async_read())) 
        }
    }
}

#[cfg(test)]
mod test_bytes {
    use super::*;
    use futures::AsyncReadExt;

    #[tokio::test]
    async fn can_read_from_vec() {
        let mut bytes = Bytes::from_vec(vec![1,2,3,4,5]);

        let mut output = vec![];
        let n = bytes.read_to_end(&mut output).await.expect("No error should occur reading back the bytes");

        assert_eq!(n, 5);
        assert_eq!(output, vec![1,2,3,4,5]);
    }

    #[tokio::test]
    async fn can_read_from_reader() {
        // We use another instance of Bytes as our reader:
        let mut bytes = Bytes::from_reader(Bytes::from_vec(vec![1,2,3,4,5]));

        let mut output = vec![];
        let n = bytes.read_to_end(&mut output).await.expect("No error should occur reading back the bytes");

        assert_eq!(n, 5);
        assert_eq!(output, vec![1,2,3,4,5]);
    }

    #[tokio::test]
    async fn can_read_from_stream() {
        let mut bytes = Bytes::from_stream(futures::stream::iter(vec![
            Ok(vec![1]),
            Ok(vec![2]),
            Ok(vec![3]),
            Ok(vec![4]),
            Ok(vec![5]),
        ]));

        let mut output = vec![];
        let n = bytes.read_to_end(&mut output).await.expect("No error should occur reading back the bytes");

        assert_eq!(n, 5);
        assert_eq!(output, vec![1,2,3,4,5]);
    }
}

/// This wraps other `AsyncRead` impls and caps how many bytes can be
/// read from the underlying reader before an `UnexpectedEof` error is
/// returned. The cap exists at the type level via `const MAX`.
pub (crate) struct CappedAsyncRead<T: AsyncRead, const MAX: usize> {
    inner: T,
    bytes_read: usize
}

impl <T: AsyncRead, const MAX: usize> AsyncRead for CappedAsyncRead<T, MAX> {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {        
        // Structural projection; Pin<CappedAsyncRead> to Pin<T>. Must not access the field in any other way.
        let inner = unsafe { 
            self.as_mut().map_unchecked_mut(|lr| &mut lr.inner) 
        };

        // Read some bytes into the provided buffer:
        let new_bytes_read = match inner.poll_read(cx, buf) {
            Poll::Ready(Ok(n)) => {
                n
            },
            Poll::Ready(Err(e)) => {
                return Poll::Ready(Err(e))
            },
            Poll::Pending => {
                return Poll::Pending
            }
        };

        // Bail if we've read more bytes than our limit allows. Non-structural projection here;
        // Pin<CappedAsyncRead> to &mut usize.
        let bytes_read = unsafe { &mut self.as_mut().get_unchecked_mut().bytes_read };
        *bytes_read += new_bytes_read;
        if *bytes_read > MAX {
            return Poll::Ready(
                Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Size limit exceeded"))
            )
        }

        // Return the number of bytes written on this run:
        Poll::Ready(Ok(new_bytes_read))
    }
}

impl <T: AsyncRead, const MAX: usize> CappedAsyncRead<T, MAX> {
    pub fn new(read: T) -> CappedAsyncRead<T, MAX> {
        CappedAsyncRead {
            inner: read,
            bytes_read: 0
        }
    }
}

#[cfg(test)]
mod test_capped_reader {
    use super::*;
    use futures::AsyncReadExt;

    #[tokio::test]
    async fn capped_reader_ok_with_0_bytes() {
        // no bytes to read:
        let input = vec![];
        let mut capped_reader = CappedAsyncRead::<_, 5>::new(&*input);

        let mut output = vec![];
        let n = capped_reader.read_to_end(&mut output).await.expect("No error should occur reading no bytes");
        assert_eq!(n, 0);
        assert_eq!(output, Vec::<u8>::new());
    }

    #[tokio::test]
    async fn capped_reader_errors_if_limit_exceeded() {
        // 6 bytes to read:
        let input = vec![1,2,3,4,5,6];
        // 5 byte limit though:
        let mut limit_to_5_bytes = CappedAsyncRead::<_, 5>::new(&*input);

        let mut output = vec![];
        let err = limit_to_5_bytes.read_to_end(&mut output).await.expect_err("Exceeded limit: error expected");
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
    }

    #[tokio::test]
    async fn capped_reader_ok_if_limit_not_exceeded() {
        // 5 bytes to read:
        let input = vec![1,2,3,4,5];
        // 5 byte limit, so all OK:
        let mut limit_to_5_bytes = CappedAsyncRead::<_, 5>::new(&*input);

        let mut output = vec![];
        let n = limit_to_5_bytes.read_to_end(&mut output).await.expect("Should successfully read all bytes");
        assert_eq!(n, 5);
        assert_eq!(output, vec![1,2,3,4,5]);
    }
}