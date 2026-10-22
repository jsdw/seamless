#![doc(hidden)]
use super::response::{Body, HandlerResponse};
use super::to_async::ToAsync;
use crate::api::{ApiBody, ApiBodyInfo, ApiBodyType, ApiError};
use crate::handler::{request::AsyncReadBody, HandlerBody, HandlerParam};
use http::{Method, Request, Response};
use std::future::Future;
use std::pin::Pin;

// Internally we resolve the provided handler functions into this:
#[doc(hidden)]
pub struct Handler {
    pub method: Method,
    #[allow(clippy::type_complexity)]
    pub handler: Box<
        dyn for<'a> Fn(
                Request<&'a mut dyn AsyncReadBody>,
            ) -> Fut<'a, Result<Response<Body>, ApiError>>
            + Send
            + Sync,
    >,
    pub request_type: ApiBodyInfo,
    pub response_type: ApiBodyInfo,
}

impl Handler {
    /// Run the handler on a request.
    pub fn call<'a>(
        &self,
        req: Request<&'a mut dyn AsyncReadBody>,
    ) -> Fut<'a, Result<Response<Body>, ApiError>> {
        (self.handler)(req)
    }
}

// A type alias for an overly complicated boxed Future type that can be sent across threads.
type Fut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// This trait is implemented for all handler functions which are
/// applicable. Handler functions expect [`HandlerParam`] arguments first, and
/// then optionally an argument that implements [`HandlerBody`] (eg `FromJson` or
/// `FromBinary`) if the handler requires a body to be provided. Arguments
/// are resolved in the order that they are provided.
#[doc(hidden)]
pub trait IntoHandler<A> {
    fn into_handler(self) -> Handler;
}

// We want the handler functions to support different numbers of params as well
// as either having a body or not; this macro generates the impls for this.
macro_rules! impl_into_handler {
    ( $( $($param:ident)* ;)* ) => {

        // Markers to differentiate the trait impls:
        #[doc(hidden)]
        pub struct WithBody;
        #[doc(hidden)]
        pub struct WithoutBody;

        // Impl each trait with and without the body, and with each number of params provided.
        // The body must always be the last argument. These impls work because the traits in correct
        // usage will not overlap, and so the correct impl can be chosen.
        $(

        impl <HandlerFn, BodyParam, Res, Output, $($param,)* A>
            IntoHandler<(WithBody, HandlerFn, BodyParam, Res, Output, $($param,)* A)> for HandlerFn
        where
            // This is the rough shape we want a handler function to have:
            HandlerFn: Fn($($param,)* BodyParam) -> Res + Clone + Sync + Send + 'static,

            // The last argument to handler functions should be a HandlerBody:
            BodyParam: HandlerBody + ApiBody + Send,

            // Any other argument given must be a HandlerParam:
            $( $param: HandlerParam + Send, )*

            // The thing returned from the handler can be sync or async:
            Res: ToAsync<Output, A> + Send,
            // The _Output_ from the Res needs to convert into an http::Response or ApiError:
            Output: HandlerResponse + Send + 'static,
            // What will the response eventually look like?
            <Output as HandlerResponse>::ResponseBody: ApiBody
        {
            fn into_handler(self) -> Handler {
                Handler {
                    method: BodyParam::handler_method(),
                    handler: Box::new(move |req: Request<&mut dyn AsyncReadBody>| {
                        let inner_handler = self.clone();
                        Box::pin(async move {
                            let (parts, body) = req.into_parts();
                            let bodyless_req = Request::from_parts(parts, ());

                            $(
                            #[allow(non_snake_case)]
                            let $param = $param::handler_param(&bodyless_req)
                                .await
                                .map_err(Into::into)?;
                            )*

                            let (parts, _) = bodyless_req.into_parts();
                            let req = Request::from_parts(parts, body);
                            let body = BodyParam::handler_body(req).await?;
                            inner_handler($($param,)* body)
                                .to_async()
                                .await
                                .handler_response()
                                .await
                        })
                    }),
                    request_type: BodyParam::api_body_info(),
                    response_type: <Output as HandlerResponse>::ResponseBody::api_body_info()
                }
            }
        }

        impl <HandlerFn, Res, Output, $($param,)* A>
            IntoHandler<(WithoutBody, HandlerFn, Res, Output, $($param,)* A)> for HandlerFn
        where
            // This is the rough shape we want a handler function to have (this time, no "body" at the end):
            HandlerFn: Fn($($param,)*) -> Res + Clone + Sync + Send + 'static,

            // Any argument given must be a HandlerParam:
            $( $param: HandlerParam + Send, )*

            // The thing returned from the handler can be sync or async:
            Res: ToAsync<Output, A> + Send,
            // The _Output_ from the Res needs to convert into an http::Response or ApiError:
            Output: HandlerResponse + Send + 'static,
            // What will the response eventually look like?
            <Output as HandlerResponse>::ResponseBody: ApiBody
        {
            fn into_handler(self) -> Handler {
                Handler {
                    method: Method::GET,
                    handler: Box::new(move |req| {
                        let inner_handler = self.clone();
                        Box::pin(async move {
                            let (parts, _) = req.into_parts();
                            #[allow(unused)]
                            let bodyless_req = Request::from_parts(parts, ());

                            $(
                            #[allow(non_snake_case)]
                            let $param = $param::handler_param(&bodyless_req)
                                .await
                                .map_err(Into::into)?;
                            )*

                            inner_handler($($param),*)
                                .to_async()
                                .await
                                .handler_response()
                                .await
                        })
                    }),
                    request_type: ApiBodyInfo {
                        description: "No request body is expected".to_owned(),
                        ty: ApiBodyType::Null
                    },
                    response_type: <Output as HandlerResponse>::ResponseBody::api_body_info()
                }
            }
        }

    )*}
}

impl_into_handler!(
    ;
    C1;
    C1 C2;
    C1 C2 C3;
    C1 C2 C3 C4;
    C1 C2 C3 C4 C5;
    C1 C2 C3 C4 C5 C6;
    C1 C2 C3 C4 C5 C6 C7;
    C1 C2 C3 C4 C5 C6 C7 C8;
);
