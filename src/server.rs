use std::{collections::HashMap, future::Future, pin::Pin};

use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::{body::Bytes, service::Service, Method, Request, Response as HyperResponse};

pub struct Context {
    pub req: Request<hyper::body::Incoming>,
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
        let body = Full::new(Bytes::from(self)).map_err(|e| match e {}).boxed();
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

pub trait Handler: Send + Sync + 'static {
    fn invoke(&'static self, req: Context) -> Pin<Box<dyn Future<Output = Result<Response, hyper::Error>> + Send>>;
}

impl<F: Send + Sync + 'static, Fut> Handler for F 
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


pub struct Server {
    routes: HashMap<(Method, String), &'static dyn Handler>,
}

impl Server {
    pub fn new() -> Server {
        Server{ routes: HashMap::new() }
    }

    pub fn add_route(&mut self, method: Method, path: String, handler: &'static dyn Handler) {
        self.routes.insert((method, path), handler);
    }
}


impl Service<Request<hyper::body::Incoming>> for Server {
    type Response = Response;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response, hyper::Error>> + Send>>;

    fn call(&self, req: Request<hyper::body::Incoming>) -> Self::Future {
        if let Some(h) = self.routes.get(&(req.method().clone(), req.uri().path().to_string())) {
            h.invoke(Context{ req })
        } else {
            let mut not_found = HyperResponse::new(
                Empty::<Bytes>::new()
                    .map_err(|never| match never {})
                    .boxed()
            );
            *not_found.status_mut() = hyper::StatusCode::NOT_FOUND;
            Box::pin(async {Ok(not_found)})
        }
    }
}
