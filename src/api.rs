use std::{
  fs::{self, File},
  io::{self, stdout, Write},
};

use base64::{engine::general_purpose, Engine as _};
use crossterm::{
  cursor, style,
  terminal::{Clear, ClearType},
  QueueableCommand,
};
use futures::{stream::FuturesUnordered, StreamExt};
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
  pub id: String,
  pub title: String,
  pub url: String,
}

pub struct LifeAnime {
  pub info: LifeAnimeInfo,
  pub episodes: Vec<LifeEpisodeInfo>,
}

pub struct LifeEpisodeInfo {
  pub title: String,
  pub url: String,
  pub num: String,
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
      let id = url.split("/").last().unwrap_or("0").to_string();

      LifeAnimeInfo { id, url, title }
    })
    .collect();

  Ok((anime, url))
}

pub async fn get_anime(client: &Client, id: &String) -> AsyncResult<LifeAnime> {
  let url = format!("https://anilife.live/detail/id/{}", id).to_string();
  let res = client.get(url).send().await?;
  let anime_url = res.url().to_string();
  let html = res.text().await?;
  let document = Html::parse_document(&html);

  let anime_title_selector = Selector::parse(".entry-title").unwrap();
  let selector = Selector::parse(".eplister li").unwrap();
  let a_selector = Selector::parse("a").unwrap();
  let num_selector = Selector::parse(".epl-num").unwrap();
  let title_selector = Selector::parse(".epl-title").unwrap();

  let title = match document.select(&anime_title_selector).next() {
    Some(e) => e.inner_html(),
    None => "Unkown Title".to_string(),
  };

  let episodes: Vec<LifeEpisodeInfo> = document
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

  Ok(LifeAnime {
    info: LifeAnimeInfo {
      id: id.to_string(),
      title,
      url: anime_url,
    },
    episodes,
  })
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
  info!("{:?}", player_urls);

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

  let hls_url = match &video_data[0]["url"] {
    Value::String(url) => url,
    _ => return Err("hls url not found".into()),
  };

  Ok(hls_url.clone())
}

fn print_progress(filename: &String, count: usize, len: usize) -> io::Result<()> {
  let mut stdout = stdout();
  stdout
    .queue(cursor::RestorePosition)?
    .queue(Clear(ClearType::CurrentLine))?
    .queue(style::Print(format!("{} [{}/{}]", filename, count, len)))?;
  stdout.flush()?;
  Ok(())
}

struct Segment {
  index: i32,
  filename: String,
}

pub async fn download_episode(client: &Client, url: &String, filename: &String) -> AsyncResult<()> {
  fs::create_dir_all("./segments").unwrap();

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

  let mut count: usize = 0;
  let mut segments = Vec::new();
  let mut stdout = stdout();

  stdout.queue(cursor::SavePosition)?;
  while let Some(segment) = futures.next().await {
    if segment.is_some() {
      count += 1;
      print_progress(filename, count, segment_urls.len())?;
      segments.push(segment.unwrap());
    }
  }
  info!("successful segments {} / {}", segments.len(), segment_urls.len());

  fs::remove_file("./segments/all.ts").unwrap_or({
    warn!("all.ts does not exist (this is expected)");
  });

  let mut all_ts = fs::OpenOptions::new()
    .create_new(true)
    .append(true)
    .open("./segments/all.ts")
    .unwrap();

  println!("\nCombining...");
  segments.sort_by_key(|a| a.index);
  segments.iter().for_each(|segment| {
    info!("COMBINE {}", segment.filename);
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

  fs::remove_dir_all("./segments")?;

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

  segment_urls
}
