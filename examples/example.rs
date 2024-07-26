use rust_web_server::handler::{Context, Response};
use rust_web_server::server::Server;
use std::net::SocketAddr;

use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::{body::{Body, Bytes, Frame}, server::conn::http1, Method};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

/*
 * Handler functions should return a type implementing the trait IntoResponse.
 * For now it can be String, Bytes, Response or Result<impl IntoResponse, hyper::Error>.
 */

// sleep 5 seconds and return "hi"
async fn hi(_: Context) -> String {
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    "hi".to_string()
}

// echo the body received
async fn echo(context: Context) -> Result<Bytes, hyper::Error> {
    Ok(context.req.into_body().collect().await?.to_bytes())
}

// echo the body but in uppercase
async fn uppercase(context: Context) -> Response {

    let frame_stream = context.req.into_body().map_frame(|frame| {
        let frame = if let Ok(data) = frame.into_data() {
            data.iter()
                .map(|byte| byte.to_ascii_uppercase())
                .collect::<Bytes>()
        } else {
            Bytes::new()
        };

        Frame::data(frame)
    });

    Response::new(frame_stream.boxed())
}

// echo the body but reversed
async fn reversed(context: Context) -> Result<Response, hyper::Error> {
    let req = context.req;

    let upper = req.body().size_hint().upper().unwrap_or(u64::MAX);
    if upper > 1024 * 64 { // 64Kb
        let mut resp = Response::new(full("body to big >:["));
        *resp.status_mut() = hyper::StatusCode::PAYLOAD_TOO_LARGE;
        return Ok(resp);
    }

    let whole_body = req.collect().await?.to_bytes();

    let reversed_body = whole_body.iter()
        .rev()
        .cloned()
        .collect::<Vec<u8>>();

    Ok(Response::new(full(reversed_body)))
}

/* echo the integer route parameter with name "intParam"
 * note that there is no mechanism to make sure the parameter name used in the handler
 * is the same as the name used in the route registered (see below), if they are different 
 * it panics.
 */
async fn echo_int(context: Context) -> String {
    let int_param = context.params.get("a intParam").unwrap();
    "With parameter: ".to_string() + int_param + "\n"
}

// echo the string route parameter with name "strParam"
async fn echo_string(context: Context) -> String {
    let string_param = context.params.get("strParam").unwrap();
    "With parameter: ".to_string() + string_param + "\n"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;

        let io = TokioIo::new(stream);

        let mut server = Server::new();
        server.add_route(Method::GET, "/hi", &hi);
        server.add_route(Method::POST, "/echo", &echo);
        server.add_route(Method::POST, "/uppercase", &uppercase);
        server.add_route(Method::POST, "/reversed", &reversed);

        // surround the parameter name with ':' to make it an integer parameter
        server.add_route(Method::GET, "/echo/:intParam:", &echo_int);

        // surround the parameter name with '$' to make it a string parameter
        server.add_route(Method::GET, "/echo/$strParam$", &echo_string);

        // note that if there is more than one match for a routepath, the one that was registered
        // first will be used, for example, if you register the following route and make a GET
        // request to "/echo/rust", the "echo_string" handler will be executed because it was added
        // first to the router.
        server.add_route(Method::GET, "/echo/rust", &hi);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, server)
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}