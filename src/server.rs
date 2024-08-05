use std::{collections::HashMap, future::Future, pin::Pin};

use http_body_util::{BodyExt, Empty};
use hyper::{body::Bytes, service::Service, Method, Request, Response as HyperResponse};

use crate::{handler::{Context, Handler, Response}, router::Router};

pub struct Server {
    routers: HashMap<Method, Router>,
}

impl Server {
    pub fn new() -> Server {
        Server{ routers: HashMap::new() }
    }

    pub fn add_route(&mut self, method: Method, path: &str, handler: &'static dyn Handler) {
        let router = self.routers.entry(method).or_insert(Router::new());
        router.add_route(path, handler);
    }
}


impl Service<Request<hyper::body::Incoming>> for Server {
    type Response = Response;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response, hyper::Error>> + Send>>;

    fn call(&self, req: Request<hyper::body::Incoming>) -> Self::Future {
        if let Some(router) = self.routers.get(req.method()) {
            if let Some((h, params)) = router.get_handler(req.uri().path()) {
                return h.invoke(Context::new(req, params));
            }
        } 

        let mut not_found = HyperResponse::new(
            Empty::<Bytes>::new()
                .map_err(|never| match never {})
                .boxed()
        );
        *not_found.status_mut() = hyper::StatusCode::NOT_FOUND;
        Box::pin(async {Ok(not_found)})
    }
}
