use std::{collections::HashMap, future::Future, pin::Pin};

use http_body_util::{BodyExt, Empty};
use hyper::{body::Bytes, service::Service, Method, Request, Response as HyperResponse};

use crate::handler::{Handler, Response, Context};

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
