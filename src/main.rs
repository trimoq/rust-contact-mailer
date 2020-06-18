extern crate dotenv;
extern crate futures;
extern crate gotham;
extern crate hyper;
extern crate serde;
extern crate serde_json;
extern crate log;
extern crate env_logger;

use futures::{future, Future, Stream};
use gotham::handler::{HandlerFuture, IntoHandlerError};
use gotham::helpers::http::response::create_empty_response;
use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};
use gotham::router::Router;
use gotham::state::{FromState, State};
use hyper::{Body, Response, HeaderMap, Method, StatusCode, Uri, Version};
use log::{info};
use hyper::header::HeaderValue;

mod mail;

const SMTP_PASSWORD_KEY: &str = "MK_RUST_MAILER_SMTP_PASSWORD";

fn print_request_elements(state: &State) {
    let method = Method::borrow_from(state);
    let uri = Uri::borrow_from(state);
    let http_version = Version::borrow_from(state);
    let headers = HeaderMap::borrow_from(state);
    println!("Method: {:?}", method);
    println!("URI: {:?}", uri);
    println!("HTTP Version: {:?}", http_version);
    println!("Headers: {:?}", headers);
}

fn post_handler(mut state: State) -> Box<HandlerFuture> {
    let smtp_password = dotenv::var(SMTP_PASSWORD_KEY).unwrap();

    let mail_config = mail::Config {
        password: smtp_password,
    };

    print_request_elements(&state);
    let f = Body::take_from(&mut state)
        .concat2()
        .then(|full_body| match full_body {
            Ok(valid_body) => {
                let body_content = String::from_utf8(valid_body.to_vec()).unwrap();
                println!("Body: {}", body_content);

                let mail_data: mail::ContactMail =
                    serde_json::from_str(body_content.as_str()).unwrap();

                mail::send_contact_mail(mail_config, mail_data);
                let mut res = create_empty_response(&state, StatusCode::OK);
                {
                  let headers = res.headers_mut();
                  headers.insert("Access-Control-Allow-Origin", "https://www.marcelkoch.net".parse().unwrap());
                  headers.insert("Access-Control-Allow-Methods", "POST, OPTIONS, HEAD".parse().unwrap());
                  headers.insert("Access-Control-Allow-Headers", "Origin, Content-Type, X-Auth-Token".parse().unwrap());
                };
                future::ok((state, res))
            }
            Err(e) => future::err((state, e.into_handler_error())),
        });

    Box::new(f)
}

pub fn options_handler(state: State) -> (State, Response<Body>) {
  let mut res = create_empty_response(&state, StatusCode::OK);

  let request_headers = HeaderMap::borrow_from(&state);
    let origin_header = request_headers.get("origin");
    // let header_value_to_check = HeaderValue::from_static(r"https:\/\/(.*\.)?marcelkoch\.net");
    match origin_header {
        Some(header_value_to_check) if header_value_to_check.to_str().unwrap().ends_with("marcelkoch.net") => {
            let response_headers = res.headers_mut();
            response_headers.insert("Access-Control-Allow-Origin", HeaderValue::from(origin_header.unwrap()));
            response_headers.insert("Access-Control-Allow-Methods", "POST, OPTIONS, HEAD".parse().unwrap());
            response_headers.insert("Access-Control-Allow-Headers", "Origin, Content-Type, X-Auth-Token".parse().unwrap());
        },
        _ => {}
    }

  (state, res)
}

fn router() -> Router {
    build_simple_router(|route| {
        route.post("/").to(post_handler);
        route.options("/").to(options_handler);
    })
}

fn main() {
    env_logger::init();

    info!("Starting MK Rust Contact Mailer, listening on port: 7878");

    let addr = "0.0.0.0:7878";
    gotham::start(addr, router())
}
