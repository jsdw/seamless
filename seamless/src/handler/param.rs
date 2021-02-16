use http::{ Request };
use async_trait::async_trait;

/// When a route matches, [`Self::get_param()`] is called for each
/// argument that is provided to the handler and implements this trait.
/// A successful result is passed to the handler function. An erroneous
/// result leads to the handler function not being called, and instead an
/// error being returned from [`crate::Api::handle()`].
#[async_trait]
pub trait RequestParam where Self: Sized {
    /// An error indicating what went wrong in the event that we fail to extract
    /// our parameter from the provided request.
    type Error: 'static;
    /// Given a [`http::Request<()>`], return a value of type `T` back, or
    /// else return an error of type `E` describing what went wrong. Any errors
    /// here will lead to the route bailing out and the handler not being run.
    async fn get_param(req: &Request<()>) -> Result<Self,Self::Error>;
}

// Option<Body> means we'll return None to the handler if get_param would fail.
// This will never error.
#[async_trait]
impl <T: RequestParam> RequestParam for Option<T> {
    type Error = std::convert::Infallible;
    async fn get_param(req: &Request<()>) -> Result<Self,Self::Error> {
        Ok(T::get_param(req).await.ok())
    }
}

// Result<Context,Err> means we'll return the result of attempting to obtain the context.
// This will never error.
#[async_trait]
impl <T: RequestParam> RequestParam for Result<T,<T as RequestParam>::Error> {
    type Error = <T as RequestParam>::Error;
    async fn get_param(req: &Request<()>) -> Result<Self,Self::Error> {
        Ok(T::get_param(req).await)
    }
}