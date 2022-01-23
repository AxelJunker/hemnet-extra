// This example requires the following input to succeed:
// { "name": "some name" }

use lambda_runtime::{handler_fn, Context, Error};
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use simple_logger::SimpleLogger;
use std::env;

/// This is also a made-up example. Requests come into the runtime as unicode
/// strings in json format, which can map to any structure that implements `serde::Deserialize`
/// The runtime pays no attention to the contents of the request payload.
#[derive(Deserialize)]
// struct Request {
//     name: String,
// }
struct Request {
    version: String,
    id: String,
    source: String,
    account: String,
    time: String,
    region: String,
}

/// This is a made-up example of what a response structure may look like.
/// There is no restriction on what it can be. The runtime requires responses
/// to be serialized into json. The runtime pays no attention
/// to the contents of the response payload.
#[derive(Serialize)]
struct Response {
    req_id: String,
    msg: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // required to enable CloudWatch error logging by the runtime
    // can be replaced with any other method of initializing `log`
    SimpleLogger::new()
        .with_utc_timestamps()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    match env::var("AWS_LAMBDA_RUNTIME_API") {
        Ok(_) => lambda_runtime::run(handler_fn(upload_images_handler)).await,
        _ => upload_images_handler_local().await,
    }
}

pub(crate) async fn upload_images_handler(_: Value, _: Context) -> Result<(), Error> {
    log::info!("Running upload_images_handler");

    Ok(())
}

async fn upload_images_handler_local() -> Result<(), Error> {
    log::info!("Running upload_images_handler_localzz");

    Ok(())
}
