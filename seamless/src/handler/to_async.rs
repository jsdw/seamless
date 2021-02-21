#![doc(hidden)]

use super::response::HandlerResponse;
use std::future::Future;
use std::pin::Pin;

// Normalize handling of async and sync things
#[doc(hidden)]
pub trait ToAsync<Res,A> {
    type Output: Future<Output=Res> + Send + 'static;
    fn to_async(self) -> Self::Output;
}

#[doc(hidden)]
pub struct Async;
impl <Res, F> ToAsync<Res,Async> for F
where
  F: Future<Output=Res> + Send + 'static
{
    type Output = F;
    fn to_async(self) -> Self::Output {
        self
    }
}

#[doc(hidden)]
pub struct Sync;
impl <Res> ToAsync<Res,Sync> for Res
where
  // Res has to be constrained to be HandlerResponse
  // only so that no output can potentially implement both
  // the Sync and Async version of the trait.
  Res: Send + 'static + HandlerResponse,
{
    type Output = Pin<Box<dyn Future<Output=Res> + Send + 'static>>;
    fn to_async(self) -> Self::Output {
        Box::pin(async move { self })
    }
}