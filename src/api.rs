use std::{
  fmt,
  fs::{self, File},
  io::{self, Write},
};

use base64::{engine::general_purpose, Engine as _};
use futures::{stream::FuturesUnordered, StreamExt};
use indicatif::ProgressBar;
use log::{debug, info, warn};
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::Value;

use crate::AsyncResult;

pub const HOST: &str = "https://anilife.live";
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36";

// const HLS_ENC_TAG: &str = "#EXT-X-KEY";
const HLS_SEG_TAG: &str = "#EXTINF";

pub fn build_url(path: &String) -> String {
  HOST.to_string() + path
}

pub fn build_url_from_str(path: &str) -> String {
  HOST.to_string() + path
}

pub struct LifeAnimeInfo {
  pub title: String,
  pub url: String,
}

pub struct LifeEpisodeInfo {
  pub title: String,
  pub url: String,
  pub num: String,
}

impl fmt::Display for LifeAnimeInfo {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.title)
  }
}

impl fmt::Display for LifeEpisodeInfo {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{} - {}", self.num, self.title)
  }
}

impl fmt::Debug for LifeAnimeInfo {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "anime {{ title: {}, url: {} }}", self.title, self.url)
  }
}

impl fmt::Debug for LifeEpisodeInfo {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(
      f,
      "anime {{ title: {}, url: {}, num: {} }}",
      self.title, self.url, self.num
    )
  }
}

pub async fn search(client: &Client, query: &String) -> AsyncResult<(Vec<LifeAnimeInfo>, String)> {
  let search_path = format!("/search?keyword={}", query);
  let url = build_url(&search_path);
  let html = client.get(&url).send().await?.text().await?;
  let document = Html::parse_document(&html);

  let selector = Selector::parse(".bsx").unwrap();
  let a_selector = Selector::parse("a").unwrap();
  let title_selector = Selector::parse("h2[itemprop]").unwrap();

  let anime: Vec<LifeAnimeInfo> = document
    .select(&selector)
    .map(|element| {
      let url_element = element.select(&a_selector).next();
      let title_element = element.select(&title_selector).next();
      (url_element, title_element)
    })
    .filter(|(url_element, title_element)| url_element.is_some() && title_element.is_some())
    .map(|(url_element, title_element)| {
      let url_str = url_element.unwrap().value().attr("href").unwrap_or("");
      let url = build_url_from_str(url_str);
      let title = title_element.unwrap().inner_html();

      LifeAnimeInfo { url, title }
    })
    .collect();

  Ok((anime, url))
}

pub async fn get_episodes(
  client: &Client,
  url: &String,
  referer: &String,
) -> AsyncResult<(Vec<LifeEpisodeInfo>, String)> {
  let res = client.get(url).header("Referer", referer).send().await?;
  let episode_url = res.url().to_string();
  let html = res.text().await?;
  let document = Html::parse_document(&html);

  let selector = Selector::parse(".eplister li").unwrap();
  let a_selector = Selector::parse("a").unwrap();
  let num_selector = Selector::parse(".epl-num").unwrap();
  let title_selector = Selector::parse(".epl-title").unwrap();

  let episode: Vec<LifeEpisodeInfo> = document
    .select(&selector)
    .map(|elem| {
      let url_elem = elem.select(&a_selector).next();
      let title_elem = elem.select(&title_selector).next();
      let num_elem = elem.select(&num_selector).next();

      (url_elem, title_elem, num_elem)
    })
    .filter(|(u, t, n)| u.is_some() && t.is_some() && n.is_some())
    .map(|(url_elem, title_elem, num_elem)| {
      let url = url_elem
        .unwrap()
        .value()
        .attr("href")
        .unwrap_or("")
        .to_string();
      let num = num_elem.unwrap().inner_html();
      let title = title_elem.unwrap().inner_html();

      LifeEpisodeInfo { url, title, num }
    })
    .collect();

  Ok((episode, episode_url))
}

pub async fn get_episode_hls(
  client: &Client,
  url: &String,
  referer: &String,
) -> AsyncResult<String> {
  let episode_html = client
    .get(url)
    .header("Referer", referer)
    .send()
    .await?
    .text()
    .await?;
  let player_url_re = Regex::new(r#"(?<path>https:\/\/anilife.live\/h\/live\?p=.+)""#).unwrap();
  let player_urls: Vec<String> = player_url_re
    .captures_iter(&episode_html)
    .map(|caps| caps["path"].to_string())
    .collect();

  if player_urls.len() == 0 {
    warn!("no players");
    return Err("no players".into());
  }
  debug!("{:?}", player_urls);

  let player_url = &player_urls[0];
  let player_html = client
    .get(player_url)
    .header("Referer", referer)
    .send()
    .await?
    .text()
    .await?;

  let aldata_re = Regex::new(r#"var _aldata = '(.+?)'"#).unwrap();
  let Some((_, [encoded_player_data])) = aldata_re
      .captures(&player_html)
      .map(|caps| caps.extract()) else {return Err("_aldata not found".into())};
  let player_data_json = general_purpose::STANDARD
    .decode(encoded_player_data)
    .unwrap();
  let player_data: Value = serde_json::from_slice(&player_data_json)?;
  let video_url = match &player_data["vid_url_1080"] {
    Value::String(url) => format!("https://{}", url),
    _ => return Err("video url not found".into()),
  };

  let video_data = client
    .get(video_url)
    .header("Referer", player_url)
    .send()
    .await?
    .json::<serde_json::Value>()
    .await?;
  debug!("{}", video_data);

  let hls_url = match &video_data[0]["url"] {
    Value::String(url) => url,
    _ => return Err("hls url not found".into()),
  };

  Ok(hls_url.clone())
}

struct Segment {
  index: i32,
  filename: String,
}

pub async fn download_episode(client: &Client, url: &String, filename: &String) -> AsyncResult<()> {
  let content = client
    .get(url)
    .header("Referer", HOST)
    .send()
    .await?
    .text()
    .await?;

  let segment_urls = parse_hls(content);

  let mut futures = FuturesUnordered::new();
  for (idx, segment_url) in segment_urls.iter().enumerate() {
    let url = segment_url.clone();
    let handle = download_segment(idx as i32, url);
    futures.push(handle);
  }

  let bar = ProgressBar::new(segment_urls.len() as u64);
  let mut segments = Vec::new();
  while let Some(segment) = futures.next().await {
    bar.inc(1);
    if segment.is_some() {
      segments.push(segment.unwrap());
    }
  }
  debug!("successful segments {}", segments.len());
  bar.finish();

  fs::remove_file("./segments/all.ts").unwrap_or({
    warn!("all.ts does not exist (this is expected)");
  });

  let mut all_ts = fs::OpenOptions::new()
    .create_new(true)
    .append(true)
    .open("./segments/all.ts")
    .unwrap();

  segments.sort_by_key(|a| a.index);
  segments.iter().for_each(|segment| {
    debug!("COMBINE {}", segment.filename);
    let mut segment_ts = fs::OpenOptions::new()
      .read(true)
      .open(&segment.filename)
      .unwrap();
    io::copy(&mut segment_ts, &mut all_ts).unwrap();
  });

  match fs::rename("./segments/all.ts", format!("./{}.ts", filename)) {
    Err(e) => debug!("{}", e),
    Ok(_) => (),
  }

  Ok(())
}

async fn download_segment(index: i32, url: String) -> Option<Segment> {
  info!("START segment {}", index);
  let client = reqwest::Client::new();
  let res = client
    .get(url)
    .header("User-Agent", USER_AGENT)
    .header("Referer", HOST)
    .header("Origin", HOST)
    .send()
    .await;

  if res.is_err() {
    return None;
  }

  let bytes = res.unwrap().bytes().await.unwrap();

  let filename = format!("./segments/seg{:04}.ts", index);

  let mut file = File::create(&filename).unwrap();
  let encrypted = bytes.to_vec();
  file.write_all(&encrypted).unwrap();
  info!("END segment {} length: {}", index, bytes.len());

  Some(Segment { index, filename })
}

fn parse_hls(content: String) -> Vec<String> {
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
  segment_urls
}
