use std::{
  fs::{self, File},
  io::{prelude::*, BufReader},
  net::TcpListener,
  path::Path,
};

use log::{debug, info};
use regex::Regex;
use reqwest::StatusCode;
use serde_json::Value;

use crate::AsyncResult;

const CLIENT_ID: &str = "553403901759-bq0ckshrbpkttm4d6mv260uaa5l1i1l3.apps.googleusercontent.com";
const LOCALHOST: &str = "127.0.0.1:4713";

async fn get_access_token() -> AsyncResult<String> {
  let client_secret_path = "/tmp/anilife-rs-secret".to_string();
  let client_secret = fs::read_to_string(client_secret_path).unwrap();
  let client_secret = client_secret.trim();

  let redirect_uri = "http://localhost:4713/callback";
  let scope = "https://www.googleapis.com/auth/youtube.upload";
  let auth_uri = format!("https://accounts.google.com/o/oauth2/v2/auth?response_type=code&client_id={}&redirect_uri={}&scope={}", CLIENT_ID, redirect_uri, scope);
  println!("GOTO: {}", auth_uri);

  let listener = TcpListener::bind(LOCALHOST).unwrap();
  let mut code: String = String::new();
  for stream in listener.incoming() {
    let mut stream = stream.unwrap();
    let buf_reader = BufReader::new(&mut stream);
    let http_request: Vec<_> = buf_reader
      .lines()
      .map(|res| res.unwrap())
      .take_while(|line| !line.is_empty())
      .collect();

    let head = &http_request[0];
    let fields: Vec<&str> = head.split(' ').collect();

    if fields[1].starts_with("/callback") {
      let code_pattern = Regex::new(r#"code=(?<code>.+?)\&"#).unwrap();
      let Some((_, [matched_code])) = code_pattern
        .captures(&fields[1])
        .map(|caps| caps.extract()) else {continue};
      code = matched_code.to_string();
      break;
    }
  }

  let client = reqwest::Client::new();
  let body = format!("code={code}&client_id={CLIENT_ID}&client_secret={client_secret}&redirect_uri=http://localhost:4713/callback&grant_type=authorization_code");
  let res = client
    .post("https://oauth2.googleapis.com/token")
    .header("Content-Type", "application/x-www-form-urlencoded")
    .body(body)
    .send()
    .await?
    .json::<Value>()
    .await?;

  debug!("{}", res);
  let access_token = match &res["access_token"] {
    Value::String(s) => s,
    _ => {
      return Err("access token parsing error".to_string().into());
    }
  };

  Ok(access_token.to_owned())
}

pub async fn upload(filename: &String) -> AsyncResult<()> {
  let access_token = get_access_token().await?;
  println!("got access token");

  // TODO: stream content to body
  let mut video_file = File::options().read(true).open(filename).unwrap();
  let mut video_buf: Vec<u8> = Vec::new();
  let video_title = Path::new(filename).file_name().unwrap().to_str().unwrap();
  let video_length = video_file.read_to_end(&mut video_buf).unwrap();
  let video_type = "video/*";
  info!("read file: {} length: {}", filename, video_length);

  let client = reqwest::Client::new();
  let init_body = format!(
    r#"{{"snippet":{{"title":"{}","description":"","tags":[],"categoryId":22}},"status":{{"privacyStatus":"private","embeddable":true,"license":"youtube"}}}}"#,
    video_title
  );
  println!("getting upload url");
  let init_res = client
    .post("https://www.googleapis.com/upload/youtube/v3/videos?uploadType=resumable&part=snippet,status,contentDetails")
    .header("Authorization", format!("Bearer {access_token}"))
    .header("Content-Length", init_body.len())
    .header("Content-Type", "application/json; charset=UTF-8")
    .header("X-Upload-Content-Length", video_length)
    .header("X-Upload-Content-Type", video_type)
    .body(init_body).send().await?;
  let headers = init_res.headers().clone();
  let body = init_res.text().await?;
  debug!("upload url body: {}", body);

  let upload_url = headers.get("Location").unwrap().to_str().unwrap();
  info!("upload url {}", upload_url);

  println!("uploading...");
  // TODO: retry logic
  let upload_res = client
    .put(upload_url)
    .header("Content-Length", video_length)
    .header("Content-Type", video_type)
    .body(video_buf)
    .send()
    .await?;
  let status = upload_res.status();
  println!("done");

  match status {
    StatusCode::OK => {
      info!("upload success");
    }
    _ => {
      info!("upload fail");
    }
  }

  Ok(())
}
