use std::{collections::HashMap, future::Future, pin::Pin};

use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::{body::{Bytes, Incoming}, Request, Response as HyperResponse};

pub type Params = HashMap<String, String>;

pub struct Context {
    pub req: Option<Request<Incoming>>,
    pub body: Bytes,
    pub params: Params,
}

impl Context {
    pub fn new(req: Request<Incoming>, params: Params) -> Context {
        Context {
            req: Some(req),
            body: Bytes::new(),
            params
        }
    }

    pub async fn collect_body(&mut self) -> Result<(), hyper::Error> {
        if self.req.is_some() {
            self.body = self.req.take().unwrap().collect().await?.to_bytes();
        }

        Ok(())
    }
}

pub type Response = HyperResponse<BoxBody<Bytes, hyper::Error>>;

pub trait IntoResponse {
    fn into_response(self) -> Result<Response, hyper::Error>;
}

impl IntoResponse for Response {
    fn into_response(self) -> Result<Response, hyper::Error> {
        Ok(self)
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Result<Response, hyper::Error> {
        let body = Full::new(Bytes::from(self)).map_err(|e| match e {}).boxed();
        let response = HyperResponse::builder()
            .status(200)
            .body(body)
            .unwrap();

        Ok(response)
    }
}

impl IntoResponse for Bytes {
    fn into_response(self) -> Result<Response, hyper::Error> {
        let body = Full::new(self).map_err(|e| match e {}).boxed();
        let response = HyperResponse::builder()
            .status(200)
            .body(body)
            .unwrap();

        Ok(response)
    }
}

impl<D: IntoResponse> IntoResponse for Option<D> {
    fn into_response(self) -> Result<Response, hyper::Error> {
        match self {
            Some(res) => res.into_response(),
            None => Ok(
                HyperResponse::builder()
                    .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Empty::new().map_err(|e| match e {}).boxed())
                    .unwrap()
            )
        }
    }
}

impl<D: IntoResponse> IntoResponse for Result<D, hyper::Error> {
    fn into_response(self) -> Result<Response, hyper::Error> {
        match self {
            Ok(d) => d.into_response(),
            Err(e) => Err(e)
        }
    }
}

pub type HandlerRef = &'static dyn Handler;

pub trait Handler: Send + Sync {
    fn invoke(
        &'static self, 
        context: Context
    ) -> Pin<Box<dyn Future<Output = Result<Response, hyper::Error>> + Send>>;
}

impl<F: Send + Sync, Fut> Handler for F 
where 
    F: Fn(Context) -> Fut,
    Fut: Future + Send,
    Fut::Output: IntoResponse
{
    fn invoke(
        &'static self, 
        context: Context
    ) -> Pin<Box<dyn Future<Output = Result<Response, hyper::Error>> + Send>> {
        Box::pin(async move {
            (self)(context).await.into_response()
        })
    }
}

pub type MiddlewareRef = &'static dyn Middleware;

pub trait Middleware: Send + Sync {
    fn invoke(
        &'static self, 
        context: Context, 
        next: &'static dyn Handler
    ) -> Pin<Box<dyn Future<Output = Result<Response, hyper::Error>> + Send>>;
}

impl<F: Send + Sync, Fut> Middleware for F 
where 
    F: Fn(Context, &'static dyn Handler) -> Fut,
    Fut: Future + Send,
    Fut::Output: IntoResponse
{
    fn invoke(
        &'static self, 
        context: Context, 
        next: &'static dyn Handler
    ) -> Pin<Box<dyn Future<Output = Result<Response, hyper::Error>> + Send>> {
        Box::pin(async move {
            (self)(context, next).await.into_response()
        })
    }
}

pub fn add_middleware(h: &'static dyn Handler, mid: &'static dyn Middleware) -> &'static dyn Handler {
    Box::leak(Box::new(
        |c: Context| {
            mid.invoke(c, h)
        }
    ))
}
