use anyhow::{anyhow, Result};
use aws_sdk_dynamodb::model::ReturnConsumedCapacity::Total;
use aws_sdk_dynamodb::model::{AttributeValue, KeysAndAttributes, ReturnConsumedCapacity, Select};
use aws_smithy_http::byte_stream::ByteStream;
use lambda_runtime::{run, service_fn, LambdaEvent};
use log::LevelFilter;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use simple_logger::SimpleLogger;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::env;
use std::process::id;
use uuid::Uuid;
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

// JSON types
#[derive(Deserialize, Debug)]
struct PropertiesJson {
    properties: HashSet<PropertyJson>,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Hash)]
struct PropertyJson {
    id: PropertyId,
}

#[derive(Deserialize, Debug)]
struct ImagesForListingJson {
    data: ListingJson,
}

#[derive(Deserialize, Debug)]
struct ListingJson {
    listing: ListingJson2,
}

// #[derive(Deserialize, Debug)]
// #[serde(tag = "__typename")]
// enum ImageTypeJson {
//     ActivePropertyListing { streetAddress: String, images: ImageJson },
//     NoImages,
// }

#[derive(Deserialize, Debug)]
struct ListingJson2 {
    // __typename: String, // ImagesForListingJson
    // #[serde(rename(deserialize = "de_name"))]
    streetAddress: String,
    #[serde(default)]
    images: Option<ImageJson>,
}

#[derive(Deserialize, Debug)]
struct ImageJson {
    images: Vec<ImageUrlJson>,
}

#[derive(Deserialize, Debug)]
struct ImageUrlJson {
    url: String,
}

// Nice types
type PropertyIds = HashSet<i32>;
type PropertyId = i32;

type Properties<'a> = Vec<Property<'a>>;

#[derive(Debug)]
struct Property<'a> {
    id: &'a PropertyId,
    street_address: String,
    images: Vec<Image>,
}

#[derive(Debug)]
struct Image {
    id: Uuid,
    bytes: ByteStream,
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
        // TODO: Make lambda retry
        // panic!("panikkk");
        // std::process::exit(1);
    };

    Ok(())
}

async fn handler() -> Result<()> {
    log::info!("Running upload_images_handler");

    let aws_config = aws_config::load_from_env().await;
    let dynamodb_client = aws_sdk_dynamodb::Client::new(&aws_config);
    let s3_client = aws_sdk_s3::Client::new(&aws_config);

    let search_key = fetch_search_key().await?;
    log::info!("Search key: {}", search_key);

    let mut property_ids = fetch_property_ids(&search_key).await?;
    log::info!("Property ids: {:?}", property_ids);

    let filtered_properties =
        filter_already_handled_property_ids(dynamodb_client, &mut property_ids).await?;

    log::info!("Filtered property ids: {:?}", filtered_properties);

    let mut property_options = vec![];
    for property_id in filtered_properties.iter() {
        let property = get_property_by_property_id(property_id).await?;
        property_options.push(property);
    }
    let properties: Properties = property_options.into_iter().flatten().collect();
    log::info!("Properties: {:?}", properties);

    for property in properties {
        upload_images(&s3_client, property.images).await?;
        // TODO: insert in dynamodb
    }

    Ok(())
}

async fn fetch_search_key() -> Result<String> {
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

    Ok(search_key)
}

async fn fetch_property_ids(search_key: &str) -> Result<PropertyIds> {
    let url = format!("https://www.hemnet.se/bostader/search/{}", search_key);
    let properties_search_result = get(&url).await?.json::<PropertiesJson>().await?;
    let property_ids = HashSet::from_iter(
        properties_search_result
            .properties
            .iter()
            .map(|property| property.id),
    );

    Ok(property_ids)
}

async fn filter_already_handled_property_ids(
    dynamodb_client: aws_sdk_dynamodb::Client,
    property_ids: &mut PropertyIds,
) -> Result<&mut PropertyIds> {
    let table_name = "Test";
    let id_field_name = "Id";

    let mut keys = vec![];

    for property_id in property_ids.iter() {
        let key = HashMap::from([(
            id_field_name.to_string(),
            AttributeValue::N(property_id.to_string()),
        )]);

        keys.push(key)
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

        property_ids.remove(&id);
    }

    Ok(property_ids)
}

async fn get_property_by_property_id<'a>(
    property_id: &'a PropertyId,
) -> Result<(Option<Property<'a>>)> {
    let json = json!({
        "operationName": "imagesForListing",
        "variables":{
            "id":property_id
        },
        "query":"query imagesForListing($id: ID!) {\n  listing(id: $id) {\n    id\n    __typename\n    streetAddress\n    isSaved\n    brokerAgency {\n      name\n      id\n      brokerCustomization {\n        compactLogoUrl\n        largeLogoUrl: logoUrl(format: BROKER_CUSTOMIZATION_LARGE)\n        __typename\n      }\n      __typename\n    }\n    ... on ActivePropertyListing {\n      liveStreams(scope: ENDED) {\n        embedUrl\n        __typename\n      }\n      videoAttachment: attachment(type: VIDEO) {\n        id\n        attachmentType\n        ... on VideoAttachment {\n          videoHemnetUrl\n          __typename\n        }\n        __typename\n      }\n      threeDAttachment: attachment(type: THREE_D) {\n        id\n        ... on ThreeDAttachment {\n          url\n          __typename\n        }\n        __typename\n      }\n      isForeclosure\n      listingBrokerGalleryUrl\n      images(limit: 500) {\n        images {\n          url(format: ITEMGALLERY_CUT)\n          portraitUrl: url(format: ITEMGALLERY_PORTRAIT_CUT)\n          fullscreenUrl: url(format: WIDTH1024)\n          originalHeight\n          originalWidth\n          labels\n          __typename\n        }\n        __typename\n      }\n      __typename\n    }\n    ... on ProjectUnit {\n      liveStreams(scope: ENDED) {\n        embedUrl\n        __typename\n      }\n      videoAttachment: attachment(type: VIDEO) {\n        id\n        attachmentType\n        ... on VideoAttachment {\n          videoHemnetUrl\n          __typename\n        }\n        __typename\n      }\n      threeDAttachment: attachment(type: THREE_D) {\n        id\n        ... on ThreeDAttachment {\n          url\n          __typename\n        }\n        __typename\n      }\n      isForeclosure\n      listingBrokerGalleryUrl\n      images(limit: 500) {\n        images {\n          url(format: ITEMGALLERY_CUT)\n          portraitUrl: url(format: ITEMGALLERY_PORTRAIT_CUT)\n          fullscreenUrl: url(format: WIDTH1024)\n          originalHeight\n          originalWidth\n          labels\n          __typename\n        }\n        __typename\n      }\n      __typename\n    }\n  }\n}\n"
    });

    let url = "https://www.hemnet.se/graphql";

    let client = reqwest::Client::new();
    let images_for_listing: ImagesForListingJson = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(json.to_string())
        .send()
        .await?
        .error_for_status()?
        .json::<ImagesForListingJson>()
        .await?;

    let street_address = images_for_listing.data.listing.streetAddress;

    let mut properties: Properties;

    match images_for_listing.data.listing.images {
        Some(_images) => {
            let image_urls: Vec<String> =
                _images.images.into_iter().map(|image| image.url).collect();

            let mut images: Vec<Image> = vec![];

            for image_url in image_urls {
                let image = download_image(&image_url).await?;
                let image_id = Uuid::new_v4();
                images.push(Image {
                    id: image_id,
                    bytes: image,
                });
            }

            let property = Property {
                id: property_id,
                street_address,
                images,
            };

            Ok(Some(property))
        }
        None => Ok(None),
    }
}

async fn download_image(image_url: &str) -> Result<(ByteStream)> {
    let image_stream = get(&image_url).await?.bytes().await?;
    let stream = ByteStream::from(image_stream);

    Ok(stream)
}

async fn upload_images(s3_client: &aws_sdk_s3::Client, images: Vec<Image>) -> Result<()> {
    for image in images {
        upload_image(&s3_client, image).await?;
    }
    Ok(())
}

async fn upload_image(s3_client: &aws_sdk_s3::Client, image: Image) -> Result<()> {
    s3_client
        .put_object()
        .body(image.bytes)
        .bucket("hemnet_images")
        .key(image.id.to_string())
        .send()
        .await?;

    Ok(())
}
async fn save_property_metadata<'a>(
    dynamodb_client: aws_sdk_dynamodb::Client,
    property: Property<'a>,
) -> Result<()> {
    Ok(())
}
