// parse m3u8 and download
use std::{
  error::Error,
  fs::{self, File},
  io::{self, Write},
};

use futures::future::join_all;
use log::{debug, info, warn};
use reqwest::{header, Client};

use crate::{api, AsyncResult};

// const HLS_ENC_TAG: &str = "#EXT-X-KEY";
const HLS_SEG_TAG: &str = "#EXTINF";

pub struct Downloader {
  client: Client,
}

impl Downloader {
  pub fn new() -> Self {
    let mut headers = header::HeaderMap::new();
    headers.insert(
      "User-Agent",
      header::HeaderValue::from_static(api::USER_AGENT),
    );
    headers.insert("Referer", header::HeaderValue::from_static(api::HOST));
    headers.insert("Origin", header::HeaderValue::from_static(api::HOST));
    let client = Client::builder().default_headers(headers).build().unwrap();

    Self { client }
  }

  async fn download_segment(index: i32, url: String) -> Result<String, Box<dyn Error>> {
    info!("START segment {}", index);
    let client = reqwest::Client::new();
    let bytes = client
      .get(url)
      .header("User-Agent", api::USER_AGENT)
      .header("Referer", api::HOST)
      .header("Origin", api::HOST)
      .send()
      .await?
      .bytes()
      .await?;
    let filename = format!("./segments/seg{:04}.ts", index);

    let mut file = File::create(&filename).unwrap();
    let encrypted = bytes.to_vec();
    file.write_all(&encrypted).unwrap();
    info!("END segment {} length: {}", index, bytes.len());
    Ok(filename)
  }

  pub async fn start(&self, url: &String) -> AsyncResult<()> {
    let content = self
      .client
      .get(url)
      .header("Referer", api::HOST)
      .send()
      .await?
      .text()
      .await?;

    let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let n = lines.len();
    let mut index = 0;
    let mut segment_urls: Vec<String> = Vec::new();

    while index < n {
      let line = &lines[index];
      if line.starts_with(HLS_SEG_TAG) {
        index += 1;
        let segment_url = &lines[index];
        segment_urls.push(segment_url.clone());
      }

      index += 1;
    }

    debug!("segment urls: {:?}", segment_urls);

    let mut handles = Vec::new();
    for (idx, segment_url) in segment_urls.iter().enumerate() {
      let url = segment_url.clone();
      let handle = Downloader::download_segment(idx as i32, url);
      handles.push(handle);
    }

    let segment_res = join_all(handles).await;

    fs::remove_file("./segments/all.ts").unwrap_or({
      warn!("all.ts does not exist (this is expected)");
    });

    let mut all_ts = fs::OpenOptions::new()
      .create_new(true)
      .append(true)
      .open("./segments/all.ts")
      .unwrap();

    for segment in segment_res {
      match segment {
        Ok(filename) => {
          debug!("COMBINE {}", filename);
          let mut segment_ts = fs::OpenOptions::new().read(true).open(filename).unwrap();
          io::copy(&mut segment_ts, &mut all_ts).unwrap();
        }
        Err(e) => {
          debug!("{}", e);
        }
      }
    }

    Ok(())
  }
}
