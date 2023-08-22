use std::{env, error::Error};

use http::create_http_client;
use regex::Regex;

pub mod api;
pub mod cli;
pub mod http;
pub mod upload;

use cli::{parse_args, print_help, Command};

pub type AsyncResult<T> = Result<T, Box<dyn Error>>;

pub fn sanitize_filename(filename: &str) -> String {
  let forbidden_pattern = r#"[<>:"/\\|?*]|[\x00-\x1F]"#;
  let re = Regex::new(forbidden_pattern).unwrap();
  let sanitized_filename = re.replace_all(filename, "").to_string();
  sanitized_filename
}

extern crate log;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let _log2 = log2::open(".log.tmp").start();
  let client = create_http_client();
  let args = env::args();
  let command = parse_args(args).unwrap();

  match command {
    Command::Help => {
      print_help();
    }
    Command::Search(query) => {
      let (anime_list, _search_url) = match api::search(&client, &query).await {
        Ok(a) => a,
        Err(e) => {
          eprintln!("Failed to search anime {}", query);
          return Err(e);
        }
      };

      println!("Results on {}", query);
      anime_list.iter().for_each(|anime| {
        println!("{:4} | {}", anime.id, anime.title);
      });
    }
    Command::List(anime_id) => {
      let anime = match api::get_anime(&client, &anime_id).await {
        Ok(a) => a,
        Err(e) => {
          eprintln!("Failed to get anime with id {}", anime_id);
          return Err(e);
        }
      };

      println!("{} episodes", anime_id);
      anime.episodes.iter().for_each(|episode| {
        println!("{:4} | {}", episode.num, episode.title);
      })
    }
    Command::Download(anime_id, episode_nums) => {
      let anime = match api::get_anime(&client, &anime_id).await {
        Ok(a) => a,
        Err(e) => {
          eprintln!("Failed to get anime with id {}", anime_id);
          return Err(e);
        }
      };

      for episode_num in episode_nums {
        let episode = match anime
          .episodes
          .iter()
          .find(|episode| episode.num.eq(&episode_num))
        {
          Some(e) => e,
          None => {
            eprintln!("Episode with episode num {} not found", episode_num);
            return Ok(());
          }
        };

        let hls_url = match api::get_episode_hls(&client, &episode.url, &anime.info.url).await {
          Ok(h) => h,
          Err(e) => {
            eprintln!("unable to get episode hls");
            return Err(e);
          }
        };

        let filename = format!("{}-{}-{}", anime.info.title, episode.num, episode.title);
        let filename = sanitize_filename(&filename);
        api::download_episode(&client, &hls_url, &filename).await?;
      }
    }
    Command::DownloadAll(anime_id) => {
      let anime = match api::get_anime(&client, &anime_id).await {
        Ok(a) => a,
        Err(e) => {
          eprintln!("Failed to get anime with id {}", anime_id);
          return Err(e);
        }
      };

      for episode in anime.episodes {
        let hls_url = match api::get_episode_hls(&client, &episode.url, &anime.info.url).await {
          Ok(h) => h,
          Err(e) => {
            eprintln!("unable to get episode hls");
            return Err(e);
          }
        };

        let filename = format!("{}-{}-{}", anime.info.title, episode.num, episode.title);
        let filename = sanitize_filename(&filename);
        api::download_episode(&client, &hls_url, &filename).await?;
      }
    }
    Command::Upload(filename) => {
      upload::upload(&filename).await?;
    }
    Command::None => {}
  }

  Ok(())
}
