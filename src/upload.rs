use std::{fs::File, io::Read};

use log::info;
use reqwest::StatusCode;

use crate::AsyncResult;

// TODO: auth logic

pub async fn upload(filename: &String, title: &String) -> AsyncResult<()> {
  let access_token = "123";

  // TODO: stream content to body
  let mut video_file = File::options().read(true).open(filename).unwrap();
  let mut video_buf: Vec<u8> = Vec::new();
  let video_length = video_file.read_to_end(&mut video_buf).unwrap();
  let video_type = "video/*";
  info!("read file: {} length: {}", filename, video_length);

  let client = reqwest::Client::new();
  let init_body = format!(
    r#"{{snippet:{{title: {},description:"",tags:[],categoryId: 22}},status:{{privacyStatus:"private",embeddable:true,license:"youtube"}}}})"#,
    title
  );
  let init_res = client
    .post("https://www.googleapis.com/upload/youtube/v3/videos?uploadType=resumable&part=snippet,status,contentDetails")
    .header("Authorization", format!("Bearer {access_token}"))
    .header("Content-Length", init_body.len())
    .header("Content-Type", "application/json; charset=UTF-8")
    .header("X-Upload-Content-Length", video_length)
    .header("X-Upload-Content-Type", video_type)
    .body(init_body).send().await?;
  let upload_url = init_res
    .headers()
    .get("Location")
    .unwrap()
    .to_str()
    .unwrap();
  info!("upload url {}", upload_url);

  // TODO: retry logic
  let upload_res = client
    .put(upload_url)
    .header("Content-Length", video_length)
    .header("Content-Type", video_type)
    .body(video_buf)
    .send()
    .await?;
  let status = upload_res.status();

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
