use std::fmt;

use base64::{engine::general_purpose, Engine as _};
use log::{debug, warn};
use regex::Regex;
use reqwest::{header, Client, IntoUrl, RequestBuilder};
use scraper::{Html, Selector};
use serde_json::Value;

use crate::AsyncResult;

pub const HOST: &str = "https://anilife.live";
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36";

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

pub struct LifeClient {
  client: Client,
}

impl LifeClient {
  pub fn new() -> Self {
    let mut headers = header::HeaderMap::new();
    headers.insert("User-Agent", header::HeaderValue::from_static(USER_AGENT));
    let client = Client::builder().default_headers(headers).build().unwrap();

    Self { client }
  }

  pub fn get<U: IntoUrl>(&self, url: U, referer: Option<&String>) -> RequestBuilder {
    let r = match referer {
      Some(r) => r,
      None => HOST,
    };

    self.client.get(url).header("Referer", r)
  }

  pub async fn search(&self, query: &String) -> AsyncResult<(Vec<LifeAnimeInfo>, String)> {
    let search_path = format!("/search?keyword={}", query);
    let url = build_url(&search_path);
    let html = self.get(&url, None).send().await?.text().await?;
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
    &self,
    url: &String,
    referer: &String,
  ) -> AsyncResult<(Vec<LifeEpisodeInfo>, String)> {
    let res = self.get(url, Some(referer)).send().await?;
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

  pub async fn get_episode_hls(&self, url: &String, referer: &String) -> AsyncResult<String> {
    let episode_html = self.get(url, Some(referer)).send().await?.text().await?;
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
    let player_html = self.get(player_url, Some(url)).send().await?.text().await?;

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

    let video_data = self
      .get(video_url, Some(player_url))
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
}
