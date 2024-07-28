use std::{collections::HashMap, future::Future, pin::Pin};

use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::{body::Bytes, Request, Response as HyperResponse};

pub type Params = HashMap<String, String>;

pub struct Context {
    pub req: Request<hyper::body::Incoming>,
    pub params: Params,
    pub next: Option<&'static dyn Handler>,
}

impl Context {
    pub fn next(mut self) -> Pin<Box<dyn Future<Output = Result<Response, hyper::Error>> + Send>> {
        let next = self.next.unwrap();
        self.next = None;
        next.invoke(self)
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

impl<D: IntoResponse> IntoResponse for Result<D, hyper::Error> {
    fn into_response(self) -> Result<Response, hyper::Error> {
        match self {
            Ok(d) => d.into_response(),
            Err(e) => Err(e)
        }
    }
}

pub trait Handler: Send + Sync {
    fn invoke(&'static self, req: Context) -> Pin<Box<dyn Future<Output = Result<Response, hyper::Error>> + Send>>;
}

impl<F: Send + Sync, Fut> Handler for F 
where 
    F: Fn(Context) -> Fut,
    Fut: Future + Send,
    Fut::Output: IntoResponse
{
    fn invoke(&'static self, context: Context) -> Pin<Box<dyn Future<Output = Result<Response, hyper::Error>> + Send>> {
        Box::pin(async move {
            (self)(context).await.into_response()
        })
    }
}

pub fn add_middleware(h: &'static dyn Handler, mid: &'static dyn Handler) -> &'static dyn Handler {
    Box::leak(Box::new(
        |c: Context| {
            let mut c = c;
            c.next = Some(h);
            mid.invoke(c)
        }
    ))
}
