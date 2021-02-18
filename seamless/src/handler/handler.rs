use http::{ Request, Response, method::Method };
use serde::{ Serialize };
use std::future::Future;
use std::pin::Pin;
use crate::api::{ ApiBody, ApiBodyInfo, ApiError };
use crate::handler::{ HandlerParam, HandlerBody };
use super::response::HandlerResponse;

// Internally we resolve the provided handler functions into this:
#[doc(hidden)]
pub struct Handler {
    pub method: Method,
    pub handler: Box<dyn Fn(Request<Vec<u8>>) -> Fut<Result<Response<Vec<u8>>,ApiError>> + Send + Sync>,
    pub request_type: ApiBodyInfo,
    pub response_type: ApiBodyInfo
}

// A type alias for an overly complicated boxed Future type that can be sent across threads.
type Fut<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// This trait is implemented for all handler functions which are
/// applicable. Handler functions expect context arguments first, and
/// then optionally an argument that implements `Body` (eg `Json` or
/// `Binary`) if the handler requires a body to be provided. Arguments
/// are resolved in the order that they are provided.
pub trait IntoHandler<A> {
    #[doc(hidden)]
    fn into_handler(self) -> Handler;
}

// We want the handler functions to support different numbers of contexts as well
// as either having a body or not; this macro generates the impls for this.
macro_rules! resolve_for_contexts {
    ( $( $($ctx:ident)*, $($err:ident)* ;)* ) => {

        // Markers to differentiate the trait impls:
        #[doc(hidden)]
        pub struct WithBody;
        #[doc(hidden)]
        pub struct WithoutBody;

        // Impl each trait with and without the body, and with each number of contexts provided.
        // The body must always be the last argument. These impls work because the traits in correct
        // usage will not overlap, and so the correct impl can be chosen.
        $(

        impl <HandlerFn, Body, BodyErr, Res, Output, HandlerErr, $($ctx,)* $($err,)* A>
            IntoHandler<(WithBody, HandlerFn, Body, BodyErr, Res, Output, HandlerErr, $($ctx,)* $($err,)* A)> for HandlerFn
        where
            // This is the rough shape we want a handler function to have:
            HandlerFn: Fn($($ctx,)* Body) -> Res + Clone + Sync + Send + 'static,

            // The last argument to handler functions should be a HandlerBody:
            Body: HandlerBody<Error=BodyErr> + ApiBody + Send,
            // Any error from this must convert to an ApiError:
            BodyErr: Into<ApiError> + 'static,

            // Any other argument given must be a HandlerParam:
            $( $ctx: HandlerParam<Error=$err> + Send, )*
            // Each of these params can return a unique error, but it needs to covnert into ApiError:
            $( $err: Into<ApiError> + Send + 'static, )*

            // The thing returned from the handler must be a valid HandlerResponse:
            Res: HandlerResponse<Output,HandlerErr,A> + Send,
            // The _Output_ from the HandlerResponse needs to implement ApiBody:
            Output: ApiBody + Serialize + 'static,
            // The _Error_ from the HandlerResponse needs to convert to a valid ApiError:
            HandlerErr: Into<ApiError> + 'static,
        {
            fn into_handler(self) -> Handler {
                #[allow(unused_variables)]
                let handler = move |req: Request<Vec<u8>>| {
                    let inner_handler = self.clone();
                    async move {

                        let (parts, body) = req.into_parts();
                        let bodyless_req = Request::from_parts(parts, ());

                        $(
                        #[allow(non_snake_case)]
                        let $ctx = $ctx::handler_param(&bodyless_req)
                            .await
                            .map_err(|e| { let e: ApiError = e.into(); e })?;
                        )*

                        let (parts, _) = bodyless_req.into_parts();
                        let req = Request::from_parts(parts, body);
                        let body = Body::handler_body(req).await.map_err(|e| { let e: ApiError = e.into(); e })?;
                        let handler_res = inner_handler($($ctx,)* body)
                            .handler_response()
                            .await
                            .map_err(|e| { let e: ApiError = e.into(); e })?;

                        let response = Response::builder()
                            .header("Content-Type", "application/json")
                            .body(handler_res.to_json_vec())
                            .unwrap();

                        Ok(response)
                    }
                };

                Handler {
                    method: Body::handler_method(),
                    handler: Box::new(move |req| Box::pin(handler(req))),
                    request_type: Body::api_body_info(),
                    response_type: Output::api_body_info()
                }
            }
        }

        impl <HandlerFn, Res, Output, HandlerErr, $($ctx,)* $($err,)* A>
            IntoHandler<(WithoutBody, HandlerFn, Res, Output, HandlerErr, $($ctx,)* $($err,)* A)> for HandlerFn
        where
            // This is the rough shape we want a handler function to have (this time, no "body" at the end):
            HandlerFn: Fn($($ctx,)*) -> Res + Clone + Sync + Send + 'static,

            // Any argument given must be a HandlerParam:
            $( $ctx: HandlerParam<Error=$err> + Send, )*
            // Each of these params can return a unique error, but it needs to covnert into ApiError:
            $( $err: Into<ApiError> + Send + 'static, )*

            // The thing returned from the handler must be a valid HandlerResponse:
            Res: HandlerResponse<Output,HandlerErr,A> + Send,
            // The _Output_ from the HandlerResponse needs to implement ApiBody:
            Output: ApiBody + Serialize + 'static,
            // The _Error_ from the HandlerResponse needs to convert to a valid ApiError:
            HandlerErr: Into<ApiError> + 'static,
        {
            fn into_handler(self) -> Handler {
                #[allow(unused_variables)]
                let handler = move |req: Request<Vec<u8>>| {
                    let inner_handler = self.clone();
                    async move {

                        let (parts, body) = req.into_parts();
                        let bodyless_req = Request::from_parts(parts, ());

                        $(
                        #[allow(non_snake_case)]
                        let $ctx = $ctx::handler_param(&bodyless_req)
                            .await
                            .map_err(|e| { let e: ApiError = e.into(); e })?;
                        )*

                        let handler_res = inner_handler($($ctx),*)
                            .handler_response()
                            .await
                            .map_err(|e| { let e: ApiError = e.into(); e })?;

                        let response = Response::builder()
                            .header("Content-Type", "application/json")
                            .body(handler_res.to_json_vec())
                            .unwrap();

                        Ok(response)
                    }
                };

                Handler {
                    method: Method::GET,
                    handler: Box::new(move |req| Box::pin(handler(req))),
                    request_type: ApiBodyInfo {
                        description: "No request body is expected".to_owned(),
                        ty: crate::api::ApiBodyType::Null
                    },
                    response_type: Output::api_body_info()
                }
            }
        }

    )*}
}

resolve_for_contexts!(
    ,;

    C1,
    E1;

    C1 C2,
    E1 E2;

    C1 C2 C3,
    E1 E2 E3;

    C1 C2 C3 C4,
    E1 E2 E3 E4;

    E1 E2 E3 E4 E5,
    C1 C2 C3 C4 C5;

    C1 C2 C3 C4 C5 C6,
    E1 E2 E3 E4 E5 E6;

    C1 C2 C3 C4 C5 C6 C7,
    E1 E2 E3 E4 E5 E6 E7;

    C1 C2 C3 C4 C5 C6 C7 C8,
    E1 E2 E3 E4 E5 E6 E7 E8;
);