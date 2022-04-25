use anyhow::{anyhow, Context, Result};
use aws_sdk_dynamodb::model::{AttributeValue, KeysAndAttributes, Select};
use aws_sdk_s3::types::ByteStream;
use aws_sdk_sesv2::client::Builder;
use aws_sdk_sesv2::model::{Body, Content, Destination, EmailContent};
use aws_sdk_sesv2::types::Blob;
use bytes::Bytes;
use dotenvy::dotenv;
use lambda_runtime::{run, service_fn, LambdaEvent};
use log::LevelFilter;
use regex::Regex;
use reqwest;
use serde::Deserialize;
use serde_json::{json, Value};
use simple_logger::SimpleLogger;
use std::collections::{HashMap, HashSet};
use std::str::Split;
use std::{env, fs};
use uuid::Uuid;

// JSON types
#[derive(Deserialize, Debug)]
struct Payload {
    Records: Vec<Record>,
}

#[derive(Deserialize, Debug)]
struct Record {
    Sns: Sns,
}

#[derive(Deserialize, Debug)]
struct Sns {
    Message: String,
}

#[derive(Deserialize, Debug)]
struct Message {
    mail: Mail,
    content: String,
}

#[derive(Deserialize, Debug)]
struct Mail {
    commonHeaders: CommonHeaders,
}

#[derive(Deserialize, Debug)]
struct CommonHeaders {
    subject: String,
}

// Nice types
type PropertyId = String; // TODO: share

static TABLE_NAME: &str = "HemnetProperties";
// TODO: share
static PROPERTY_ID_FIELD_NAME: &str = "PropertyId";
// TODO: share
static IMAGE_IDS_FIELD_NAME: &str = "ImageIds";
static S3_BUCKET_NAME: &str = "hemnet-property-images"; // TODO: share

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    SimpleLogger::new()
        .without_timestamps()
        .with_level(LevelFilter::Info)
        .init()?;

    let result = match env::var("AWS_LAMBDA_RUNTIME_API") {
        // Running in lambda
        Ok(_) => {
            run(service_fn(|event: LambdaEvent<Value>| {
                handler(event.payload)
            }))
            .await
        }
        // Running locally
        _ => {
            dotenv().ok();

            let json_example = env::var("EVENT_EXAMPLE").unwrap_or("example-1".to_string()); // TODO: Add to readme
            let path = "./example-events/".to_string() + &json_example + ".json";
            let event_string = fs::read_to_string(path)?;
            let event = serde_json::from_str(&event_string)?;

            handler(event).await.map_err(Box::from)
        }
    };

    if let Err(err) = result {
        eprintln!("Something went wrong");
        eprintln!("{:?}", err);
        // TODO: Make lambda retry
        // panic!("panikkk");
        // std::process::exit(1);
    };

    Ok(())
}

async fn handler(event: Value) -> Result<()> {
    println!("Running email-images");
    println!("Event: {:?}", event);

    let from_email_address = env::var("FROM_EMAIL_ADDRESS")?;
    let to_email_addresses = env::var("TO_EMAIL_ADDRESSES")?;
    let split_to_email_addresses: Vec<String> = to_email_addresses
        .split(",")
        .map(|x| x.to_owned())
        .collect();

    let aws_config = aws_config::load_from_env().await;
    let dynamodb_client = aws_sdk_dynamodb::Client::new(&aws_config);
    let s3_client = aws_sdk_s3::Client::new(&aws_config);
    let ses_client = aws_sdk_sesv2::Client::new(&aws_config);

    println!("hej1");
    let payload: Payload = serde_json::from_value(event)?;
    println!("hej2");
    println!("hej3: {:?}", payload.Records[0].Sns);

    let message: Message = serde_json::from_str(&payload.Records[0].Sns.Message)?;
    println!("Message: {:?}", message);
    println!("Subject: {:?}", message.mail.commonHeaders.subject);
    println!("Content: {:?}", message.content);

    let property_id: PropertyId = get_property_id(&message.content)?;

    println!("property_id: {:?}", property_id);

    let image_ids = get_property_image_ids(&dynamodb_client, property_id).await?;

    println!("image_ids: {:?}", image_ids);
    println!("image_ids length: {:?}", image_ids.len());

    let images = download_images(&s3_client, image_ids).await?;

    println!("images: {:?}", images.len());

    send_email(
        &ses_client,
        from_email_address,
        split_to_email_addresses,
        &message.content,
        images,
    )
    .await?;

    Ok(())
}

fn get_property_id(content: &String) -> Result<PropertyId> {
    let cleaned_content = content.replace("=\r\n", "").replace("\r\n", "");

    let regex_str = "https://bilder.hemnet.se/images/itemgallery.+?([a-z0-9]+).jpg";
    let regex = Regex::new(regex_str)?;

    let capture_group = regex
        .captures(&cleaned_content)
        .ok_or(anyhow!("Regex error: No captures"))?
        .get(1)
        .ok_or(anyhow!("Regex error: Regex no capture group"))?;

    let property_id: PropertyId = capture_group.as_str().to_string();

    Ok(property_id)
}

async fn get_property_image_ids(
    dynamodb_client: &aws_sdk_dynamodb::Client,
    property_id: PropertyId,
) -> Result<Vec<String>> {
    let property = dynamodb_client
        .get_item()
        .key(
            PROPERTY_ID_FIELD_NAME,
            AttributeValue::S(property_id.to_string()),
        )
        .table_name(TABLE_NAME)
        .send()
        .await?
        .item
        .ok_or(anyhow!(format!(
            "Could not find property id {} in dynamodb",
            property_id
        )))?;

    let image_ids = property
        .get(IMAGE_IDS_FIELD_NAME)
        .ok_or(anyhow!(format!(
            "Could not get image ids. Got {:?}",
            property
        )))?
        .as_ss()
        .map_err(|field| {
            anyhow!(
                "Expected field {} to be a string array, got: {:?}",
                IMAGE_IDS_FIELD_NAME,
                field
            )
        })?;

    Ok(image_ids.to_owned())
}

async fn download_images(
    s3_client: &aws_sdk_s3::Client,
    image_ids: Vec<String>,
) -> Result<Vec<ByteStream>> {
    let mut images = vec![];

    for image_id in image_ids {
        let image = s3_client
            .get_object()
            .bucket(S3_BUCKET_NAME)
            .key(image_id)
            .send()
            .await?
            .body;

        images.push(image);
    }

    Ok(images)
}

async fn send_email(
    ses_client: &aws_sdk_sesv2::Client,
    from_email_address: String,
    to_email_addresses: Vec<String>,
    content: &String,
    images: Vec<ByteStream>,
) -> Result<()> {
    let parsed_message =
        mail_parser::Message::parse(&content.as_bytes()).ok_or(anyhow!("Could not parse email"))?;

    println!("x: {:?}", parsed_message);

    let message_body = parsed_message
        .get_html_body(0)
        .ok_or(anyhow!("Could not get html body"))?;

    let subject = parsed_message.get_subject().unwrap_or("Hemnet slutpris");

    let mut message = mail_builder::MessageBuilder::new();
    message.sender(from_email_address.to_string());
    message.subject(subject);
    message.html_body(message_body.to_string());

    let mut i = 1;
    for image in images {
        let image_bytes = image.collect().await?.into_bytes().as_ref().to_owned();
        message.binary_inline("image/jpg", format!("image{}.jpg", i), image_bytes);
        i = i + 1;
    }

    let mut body_data = Vec::new();
    message.write_to(&mut body_data).unwrap();

    let msg = aws_sdk_sesv2::model::RawMessage::builder()
        .data(Blob::new(body_data))
        .build();

    let email_content = EmailContent::builder().raw(msg).build();

    let destination = Destination::builder()
        .set_to_addresses(Some(to_email_addresses))
        .build();

    ses_client
        .send_email()
        .from_email_address(from_email_address)
        .destination(destination)
        .content(email_content)
        .send()
        .await
        .context("Could not send email")?;

    Ok(())
}
