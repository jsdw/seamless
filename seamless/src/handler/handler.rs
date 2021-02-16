use http::{ Request, Response, method::Method };
use serde::{ Serialize };
use std::future::Future;
use std::pin::Pin;
use crate::api::{ ApiBody, ApiBodyInfo };
use crate::handler::{ RequestParam, RequestBody };

// Internally we resolve the provided handler functions into this:
#[doc(hidden)]
pub struct Handler<E> {
    pub method: Method,
    pub handler: Box<dyn Fn(Request<Vec<u8>>) -> Fut<Result<Response<Vec<u8>>,E>> + Send + Sync>,
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
pub trait IntoHandler<E, A> {
    #[doc(hidden)]
    fn into_handler(self) -> Handler<E>;
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

        // Impl each trait with and without the body, and with each number of contexts provided:
        $(

        impl <Req, Res, ReqErr, OutputErr, HandlerErr, F, HandlerFn $(, $ctx)* $(, $err)*> IntoHandler<OutputErr, (WithBody,Req,Res,ReqErr,HandlerErr,F $(, $ctx)* $(, $err)*)> for HandlerFn
        where
            Req: RequestBody<Error=ReqErr> + ApiBody + Send,
            // Each param must be a RequestParam and can return a unique error:
            $( $ctx: RequestParam<Error=$err> + Send, )*
            Res: ApiBody + Serialize + 'static,
            // Each param returns a unique error which must be convertable into the output error:
            $( $err: Into<OutputErr> + Send + 'static, )*
            HandlerErr: Into<OutputErr> + 'static,
            ReqErr: Into<OutputErr> + 'static,
            F: Future<Output = Result<Res,HandlerErr>> + Send + 'static,
            HandlerFn: Fn($($ctx,)* Req) -> F + Clone + Sync + Send + 'static
        {
            fn into_handler(self) -> Handler<OutputErr> {
                #[allow(unused_variables)]
                let handler = move |req: Request<Vec<u8>>| {
                    let inner_handler = self.clone();
                    async move {

                        let (parts, body) = req.into_parts();
                        let bodyless_req = Request::from_parts(parts, ());

                        $(
                        #[allow(non_snake_case)]
                        let $ctx = $ctx::get_param(&bodyless_req)
                            .await
                            .map_err(|e| { let e: OutputErr = e.into(); e })?;
                        )*

                        let (parts, _) = bodyless_req.into_parts();
                        let req = Request::from_parts(parts, body);
                        let body = Req::get_body(req).await.map_err(|e| { let e: OutputErr = e.into(); e })?;
                        let handler_res = inner_handler($($ctx,)* body)
                            .await
                            .map_err(|e| { let e: OutputErr = e.into(); e })?;

                        let response = Response::builder()
                            .header("Content-Type", "application/json")
                            .body(handler_res.to_json_vec())
                            .unwrap();

                        Ok(response)
                    }
                };

                Handler {
                    method: Req::get_method(),
                    handler: Box::new(move |req| Box::pin(handler(req))),
                    request_type: Req::api_body_info(),
                    response_type: Res::api_body_info()
                }
            }
        }

        impl <Res, OutputErr, HandlerErr, F, HandlerFn $(, $ctx)* $(, $err)*> IntoHandler<OutputErr, (WithoutBody,Res,HandlerErr,F $(, $ctx)* $(, $err)*)> for HandlerFn
        where
            $( $ctx: RequestParam<Error=$err> + Send, )*
            Res: ApiBody + Serialize + 'static,
            $( $err: Into<OutputErr> + Send + 'static, )*
            HandlerErr: Into<OutputErr> + 'static,
            F: Future<Output = Result<Res,HandlerErr>> + Send + 'static,
            HandlerFn: Fn($($ctx),*) -> F + Clone + Sync + Send + 'static
        {
            fn into_handler(self) -> Handler<OutputErr> {
                #[allow(unused_variables)]
                let handler = move |req: Request<Vec<u8>>| {
                    let inner_handler = self.clone();
                    async move {

                        let (parts, body) = req.into_parts();
                        let bodyless_req = Request::from_parts(parts, ());

                        $(
                        #[allow(non_snake_case)]
                        let $ctx = $ctx::get_param(&bodyless_req)
                            .await
                            .map_err(|e| { let e: OutputErr = e.into(); e })?;
                        )*

                        let handler_res = inner_handler($($ctx),*)
                            .await
                            .map_err(|e| { let e: OutputErr = e.into(); e })?;

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
                    response_type: Res::api_body_info()
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
);