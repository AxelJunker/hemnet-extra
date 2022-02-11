use lambda_runtime::{run, service_fn, LambdaEvent};
use log::LevelFilter;
// use serde::{Deserialize, Serialize};
use anyhow::{anyhow, Context, Result};
use regex::Regex;
use reqwest;
use serde_json::Value;
use simple_logger::SimpleLogger;
use std::env;
use std::error;
use std::fmt;

/// This is also a made-up example. Requests come into the runtime as unicode
/// strings in json format, which can map to any structure that implements `serde::Deserialize`
/// The runtime pays no attention to the contents of the request payload.
// #[derive(Deserialize)]
// struct Request {
//     name: String,
// }
// struct Request {
//     version: String,
//     id: String,
//     source: String,
//     account: String,
//     time: String,
//     region: String,
// }

/// This is a made-up example of what a response structure may look like.
/// There is no restriction on what it can be. The runtime requires responses
/// to be serialized into json. The runtime pays no attention
/// to the contents of the response payload.
// #[derive(Serialize)]
// struct Response {
//     req_id: String,
//     msg: String,
// }

#[derive(Debug)]
enum Error {
    SetLogger(log::SetLoggerError),
    RegexCreate(regex::Error),
    RegexNoCaptures,
    RegexNoCaptureGroup,
    RequestError(reqwest::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // The wrapped error contains additional information and is available
            // via the source() method.
            Error::SetLogger(..) => write!(f, "failed to initialize logger"),
            Error::RegexCreate(..) => write!(f, "couldn't create regex"),
            Error::RegexNoCaptures => write!(f, "no captures"),
            Error::RegexNoCaptureGroup => write!(f, "no capture group"),
            Error::RequestError(err) => write!(f, "request error"),
        }
    }
}

// The cause is the underlying implementation error type. Is implicitly
// cast to the trait object `&error::Error`. This works because the
// underlying type already implements the `Error` trait.
impl error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::SetLogger(e) => Some(e),
            Error::RegexCreate(e) => Some(e),
            Error::RegexNoCaptures => None,
            Error::RegexNoCaptureGroup => None,
            Error::RequestError(e) => Some(e),
        }
    }
}

// Implement the conversion from `Error` to `error::Error`.
// This will be automatically called by `?` if a `Error`
// needs to be converted into a `error::Error`.
impl From<regex::Error> for Error {
    fn from(err: regex::Error) -> Error {
        Error::RegexCreate(err)
    }
}
impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Error {
        Error::RequestError(err)
    }
}
impl From<log::SetLoggerError> for Error {
    fn from(err: log::SetLoggerError) -> Error {
        Error::SetLogger(err)
    }
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    match env::var("AWS_LAMBDA_RUNTIME_API") {
        // Running in lambda
        Ok(_) => run(service_fn(|_: LambdaEvent<Value>| handler())).await,
        // Running locally
        _ => handler().await.map_err(Box::from),
    }
}

async fn handler() -> Result<()> {
    // Required to enable CloudWatch error logging by the runtime.
    // Can be replaced with any other method of initializing `log`.
    SimpleLogger::new()
        .without_timestamps()
        .with_level(LevelFilter::Info)
        .init()?;

    log::info!("Running upload_images_handler");

    let result = fetch_hemnet_search_key().await;

    if let Err(err) = result {
        log::error!("{:?}", err);
        // panic!("panikkk");
        // std::process::exit(1);
    }

    Ok(())
}

async fn fetch_hemnet_search_key() -> Result<String> {
    let regex_str = "search_key&quot;:&quot;([a-z0-9]*)&";
    let regex = Regex::new(regex_str)?;

    let url = "https://www.hemnet.se/bostader?by=creation&order=desc&subscription=adsf33094966";
    let body = get_body(url).await?;

    let captures = regex
        .captures(&body)
        .ok_or(anyhow!("Regex error: No captures"))?;

    let capture_group = captures
        .get(0)
        .ok_or(anyhow!("Regex error: Regex no capture group"))?;

    let search_key = capture_group.as_str().to_string();

    log::info!("Search key: {}", search_key);

    Ok(search_key)
}

async fn get_body(url: &str) -> Result<String, reqwest::Error> {
    reqwest::get(url).await?.error_for_status()?.text().await
}
