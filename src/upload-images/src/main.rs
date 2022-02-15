use anyhow::{anyhow, Result};
use aws_sdk_dynamodb::model::ReturnConsumedCapacity::Total;
use aws_sdk_dynamodb::model::{AttributeValue, KeysAndAttributes, ReturnConsumedCapacity, Select};
use lambda_runtime::{run, service_fn, LambdaEvent};
use log::LevelFilter;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use simple_logger::SimpleLogger;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::env;
use std::process::id;
// use std::error;
// use std::fmt;

/// This is a made-up example of what a response structure may look like.
/// There is no restriction on what it can be. The runtime requires responses
/// to be serialized into json. The runtime pays no attention
/// to the contents of the response payload.
// #[derive(Serialize)]
// struct Response {
//     req_id: String,
//     msg: String,
// }

// #[derive(Debug)]
// enum Error {
//     SetLogger(log::SetLoggerError),
//     RegexCreate(regex::Error),
//     RegexNoCaptures,
//     RegexNoCaptureGroup,
//     RequestError(reqwest::Error),
// }
//
// impl fmt::Display for Error {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         match self {
//             // The wrapped error contains additional information and is available
//             // via the source() method.
//             Error::SetLogger(..) => write!(f, "failed to initialize logger"),
//             Error::RegexCreate(..) => write!(f, "couldn't create regex"),
//             Error::RegexNoCaptures => write!(f, "no captures"),
//             Error::RegexNoCaptureGroup => write!(f, "no capture group"),
//             Error::RequestError(..) => write!(f, "request error"),
//         }
//     }
// }
//
// // The cause is the underlying implementation error type. Is implicitly
// // cast to the trait object `&error::Error`. This works because the
// // underlying type already implements the `Error` trait.
// impl error::Error for Error {
//     fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
//         match self {
//             Error::SetLogger(e) => Some(e),
//             Error::RegexCreate(e) => Some(e),
//             Error::RegexNoCaptures => None,
//             Error::RegexNoCaptureGroup => None,
//             Error::RequestError(e) => Some(e),
//         }
//     }
// }
//
// // Implement the conversion from `Error` to `error::Error`.
// // This will be automatically called by `?` if a `Error`
// // needs to be converted into a `error::Error`.
// impl From<regex::Error> for Error {
//     fn from(err: regex::Error) -> Error {
//         Error::RegexCreate(err)
//     }
// }
// impl From<reqwest::Error> for Error {
//     fn from(err: reqwest::Error) -> Error {
//         Error::RequestError(err)
//     }
// }
// impl From<log::SetLoggerError> for Error {
//     fn from(err: log::SetLoggerError) -> Error {
//         Error::SetLogger(err)
//     }
// }

#[derive(Deserialize, Debug)]
struct Properties {
    properties: HashSet<Property>,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Hash)]
struct Property {
    id: i32,
}

async fn get(url: &str) -> Result<reqwest::Response, reqwest::Error> {
    reqwest::get(url).await?.error_for_status()
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    // Required to enable CloudWatch error logging by the runtime.
    // Can be replaced with any other method of initializing `log`.
    SimpleLogger::new()
        .without_timestamps()
        .with_level(LevelFilter::Info)
        .init()?;

    let result = match env::var("AWS_LAMBDA_RUNTIME_API") {
        // Running in lambda
        Ok(_) => run(service_fn(|_: LambdaEvent<Value>| handler())).await,
        // Running locally
        _ => handler().await.map_err(Box::from),
    };

    if let Err(err) = result {
        log::error!("{:?}", err);
        // panic!("panikkk");
        // std::process::exit(1);
    };

    Ok(())
}

async fn handler() -> Result<()> {
    log::info!("Running upload_images_handler");

    let aws_config = aws_config::load_from_env().await;
    let dynamodb_client = aws_sdk_dynamodb::Client::new(&aws_config);

    let search_key = fetch_hemnet_search_key().await?;
    let mut properties = fetch_hemnet_property_ids(&search_key).await?;
    let filtered_properties =
        filter_already_uploaded_property_ids(dynamodb_client, &mut properties.properties).await?;

    log::info!("filtered_properties: {:?}", filtered_properties);

    Ok(())
}

async fn fetch_hemnet_search_key() -> Result<String> {
    let regex_str = "search_key&quot;:&quot;([a-z0-9]*)&";
    let regex = Regex::new(regex_str)?;

    let url = "https://www.hemnet.se/bostader?by=creation&order=desc&subscription=33094966";
    let body = get(url).await?.text().await?;

    let captures = regex
        .captures(&body)
        .ok_or(anyhow!("Regex error: No captures"))?;

    let capture_group = captures
        .get(1)
        .ok_or(anyhow!("Regex error: Regex no capture group"))?;

    let search_key = capture_group.as_str().to_string();

    log::info!("Search key: {}", search_key);

    Ok(search_key)
}

async fn fetch_hemnet_property_ids(search_key: &str) -> Result<Properties> {
    let url = format!("https://www.hemnet.se/bostader/search/{}", search_key);
    let properties = get(&url).await?.json::<Properties>().await?;

    log::info!("Properties: {:?}", properties);

    Ok(properties)
}

async fn filter_already_uploaded_property_ids(
    dynamodb_client: aws_sdk_dynamodb::Client,
    properties: &mut HashSet<Property>,
) -> Result<&mut HashSet<Property>> {
    let table_name = "Test";
    let id_field_name = "Id";

    let mut keys = vec![];

    for property in properties.iter() {
        keys.push(HashMap::from([(
            id_field_name.to_string(),
            AttributeValue::N(property.id.to_string()),
        )]))
    }
    let keys_and_attributes = KeysAndAttributes::builder().set_keys(Some(keys)).build();

    let batch_get_item_responses = dynamodb_client
        .batch_get_item()
        .request_items(table_name, keys_and_attributes)
        .send()
        .await?
        .responses
        .ok_or(anyhow!("Empty response from dynamodb"))?;

    let responses = batch_get_item_responses
        .get(table_name)
        .ok_or(anyhow!(format!("Table \"{}\" does not exist", table_name)))?;

    for response in responses {
        let id = response
            .get(id_field_name)
            .ok_or(anyhow!("No {} field", id_field_name))?
            .as_n()
            .map_err(|field| {
                anyhow!(
                    "Expected field {} to be a number, got: {:?}",
                    id_field_name,
                    field
                )
            })?
            .parse::<i32>()?;

        properties.remove(&Property { id });
    }

    Ok(properties)
}
