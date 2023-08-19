// parse m3u8 and download

use std::{fs::File, io::Write};

use log::info;
use reqwest::{header, Client};
use tokio::task::JoinSet;

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

  async fn download_segment(index: i32, url: String) -> AsyncResult<i32> {
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

    let mut file = File::create(format!("./segments/seg{:04}.ts", index)).unwrap();
    let encrypted = bytes.to_vec();
    file.write_all(&encrypted).unwrap();
    info!("segment {} length: {}", index, bytes.len());
    Ok(index)
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

    info!("segment urls: {:?}", segment_urls);

    let mut set = JoinSet::new();
    for (idx, segment_url) in segment_urls.iter().enumerate() {
      let url = segment_url.clone();
      set.spawn(async move { Downloader::download_segment(idx as i32, url) });
    }

    while let Some(_index) = set.join_next().await {}

    Ok(())
  }
}
