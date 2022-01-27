use lambda_runtime::{handler_fn, Context, Error};
use log::LevelFilter;
// use serde::{Deserialize, Serialize};
use regex::Regex;
use serde_json::Value;
use simple_logger::SimpleLogger;
use std::env;
use uuid::Uuid;

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
    fetch_hemnet_search_key().await?;
    Ok(())
}

async fn fetch_hemnet_search_key() -> Result<String, String> {
    let body =
        reqwest::get("https://www.hemnet.se/bostader?by=creation&order=desc&subscription=33094966")
            .await
            .map_err(|err| "GET search key failed".to_string())?
            .text()
            .await
            .map_err(|err| "GET search key text failed".to_string())?;

    let look_for_text = "search_key&quot;:&quot;";
    let regex_string = format!("{}{}", look_for_text, "([a-z0-9]*)");

    Regex::new(&regex_string)
        .map_err(|x| "Couldn't create regex".to_string())
        .and_then(|regex| regex.captures(&body).ok_or("No captures".to_string()))
        .and_then(|x| x.get(1).ok_or("No capture group".to_string()))
        .map(|y| {
            let search_key = y.as_str().to_string();
            log::info!("Search key: {}", search_key);
            return search_key;
        })
}
