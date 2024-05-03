use bytes::Bytes;
use dotenv::dotenv;
use image::io::Reader as ImageReader;
use reqwest::blocking::{multipart, Client};
use reqwest::header::{HeaderMap, AUTHORIZATION};
use std::env;
use std::fs::File;
use std::io::Write;

fn get_image_from_web(url: &str) -> Result<Bytes, Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client.get(url).send()?;
    if response.status().is_success() {
        let bytes = response.bytes()?;
        Ok(bytes)
    } else {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to download image",
        )))
    }
}

fn compare_with_image_file(
    target_image: &Bytes,
    file_path: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let img = match ImageReader::open(file_path) {
        Ok(reader) => reader.decode()?.to_rgb8().to_vec(),
        Err(_) => vec![],
    };

    let target_img = ImageReader::new(std::io::Cursor::new(target_image))
        .with_guessed_format()?
        .decode()?;
    let target_img = target_img.to_rgb8().to_vec();

    Ok(img == target_img)
}

fn save_image_to_file(bytes: Bytes, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(file_path)?;
    file.write_all(&bytes.to_vec())?;
    Ok(())
}

fn send_slack(
    slack_token: &str,
    slack_channels: String,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let form = multipart::Form::new()
        .file("file", file_path)?
        .text("channels", slack_channels);

    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, format!("Bearer {}", slack_token).parse()?);

    // Ref: https://api.slack.com/methods/files.upload
    let response = client
        .post("https://slack.com/api/files.upload")
        .headers(headers)
        .multipart(form)
        .send()?;

    println!("{}", response.text()?);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let url = env::var("URL").expect("URL must be set");
    let slack_token = env::var("SLACK_TOKEN").expect("SLACK_TOKEN must be set");
    let slack_channels = env::var("SLACK_CHANNELS").expect("SLACK_CHANNELS must be set");
    let file_path = "image.png";

    let web_bytes = get_image_from_web(url.as_str())?;
    let result = compare_with_image_file(&web_bytes, file_path)?;
    if result {
        println!("No change");
    } else {
        println!("Images are different");
        save_image_to_file(web_bytes, file_path)?;
        send_slack(
            slack_token.as_str(),
            slack_channels.to_string(),
            file_path,
        )?;
    }
    Ok(())
}
