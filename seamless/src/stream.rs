/*!
This module provides the [`AsyncReadBody`] trait, which is a shorthand for anything
which implements `AsyncRead + Send + Unpin + 'static` and is expected as the body of
incoming requests. 

Also provided is the [`Bytes`] struct, which implements [`AsyncReadBody`] and so can be
passed as the body to incoming requests, and provides convenience functions to convert 
from things like [`Vec<u8>`] or [`Stream<Item = Vec<u8>>`] into itself, so that they can
also be passed as the body to incoming requests fairly easily.
*/
use futures::{ Stream, TryStreamExt, io::{ AsyncRead, Cursor } };
use std::pin::Pin;
use std::task::{ Poll, Context };

/// Any valid body will implement this trait
pub trait AsyncReadBody: AsyncRead + Send + Unpin + 'static {}
impl <T: AsyncRead + Send + Unpin + 'static> AsyncReadBody for T {}

/// A collection of bytes that can be read from asynchronously via the
/// [`futures::AsyncRead`] trait.
pub struct Bytes(
    Pin<Box<dyn AsyncReadBody>>
); 

impl std::fmt::Debug for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ByteStream").finish()
    }
}

impl AsyncRead for Bytes {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        self.0.as_mut().poll_read(cx, buf)
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
        Bytes(Box::pin(Cursor::new(bytes)))
    }
    /// Turn a thing implementing [`futures::AsyncRead`] into [`Bytes`].
    pub fn from_reader<S: AsyncReadBody>(reader: S) -> Bytes {
        Bytes(Box::pin(reader))
    }
    /// Turn a thing implementing [`futures::Stream`] into [`Bytes`].
    pub fn from_stream<S: Stream<Item = std::io::Result<Vec<u8>>> + 'static + Send + Unpin>(stream: S) -> Bytes {
        Bytes(Box::pin(stream.into_async_read()))
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