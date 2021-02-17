use http::{ Request };
use async_trait::async_trait;

/// Implement this for anything that you want to be able to pass into a request
/// handler that doesn't want to consume the body of the request. This is
/// useful for implementing request guards which prevent the handler from being
/// called unless specific conditions are met (for instance a user is logged
/// in).
///
/// # Example
///
/// ```
/// # use seamless::handler::RequestParam;
/// # use seamless::http::Request;
/// # use seamless::api::ApiError;
/// # struct State;
/// # impl State {
/// #     async fn get_user(&self, req: &Request<()>) -> Result<User,ApiError> {
/// #         Ok(User { id: "Foo".to_owned() })
/// #     }
/// # }
/// // This represents a valid user of the API.
/// pub struct User { pub id: String }
///
/// // Make it possible to ask for the current user in a request:
/// #[seamless::async_trait]
/// impl RequestParam for User {
///     type Error = ApiError;
///     async fn request_param(req: &Request<()>) -> Result<Self,Self::Error> {
///         // We can put things (like DB connections) into requests before they
///         // are handed to the API, and then pluck them out here to use:
///         let state = req.extensions()
///             .get::<State>()
///             .map(|s| s.clone())
///             .unwrap();
///         state.get_user(req).await
///     }
/// }
/// ```
#[async_trait]
pub trait RequestParam where Self: Sized {
    /// An error indicating what went wrong in the event that we fail to extract
    /// our parameter from the provided request.
    ///
    /// To be usable with [`crate::api::Api`], the error should implement `Into<ApiError>`
    /// (the [`macro@crate::ApiError`] macro can be used to gelp with this).
    ///
    /// It can be simpler just to set this to `ApiError` directly.
    type Error: 'static;
    /// Given a [`http::Request<()>`], return a value of type `T` back, or
    /// else return an error of type `E` describing what went wrong. Any errors
    /// here will lead to the route bailing out and the handler not being run.
    async fn request_param(req: &Request<()>) -> Result<Self,Self::Error>;
}

// Option<Body> means we'll return None to the handler if request_param would fail.
// This will never error.
#[async_trait]
impl <T: RequestParam> RequestParam for Option<T> {
    type Error = std::convert::Infallible;
    async fn request_param(req: &Request<()>) -> Result<Self,Self::Error> {
        Ok(T::request_param(req).await.ok())
    }
}

// Result<Context,Err> means we'll return the result of attempting to obtain the context.
// This will never error.
#[async_trait]
impl <T: RequestParam> RequestParam for Result<T,<T as RequestParam>::Error> {
    type Error = <T as RequestParam>::Error;
    async fn request_param(req: &Request<()>) -> Result<Self,Self::Error> {
        Ok(T::request_param(req).await)
    }
}