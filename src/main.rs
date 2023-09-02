use std::{env, error::Error, fs};

use env_logger::Env;
use http::create_http_client;
use log::error;
use regex::Regex;

pub mod api;
pub mod cli;
pub mod http;
pub mod video;

use cli::{parse_args, print_help, CommandType};

pub type AsyncResult<T> = Result<T, Box<dyn Error>>;

trait FileName {
  fn sanitize(&self) -> String;
  fn zero_pad(&self, width: usize) -> String;
}

impl FileName for String {
  fn sanitize(&self) -> String {
    let forbidden_pattern = r#"[<>:"/\\|?*]|[\x00-\x1F]"#;
    let re = Regex::new(forbidden_pattern).unwrap();
    let sanitized_filename = re.replace_all(self, "").to_string();
    sanitized_filename
  }

  fn zero_pad(&self, width: usize) -> String {
    let padding = width.saturating_sub(self.len());
    let padded_string = format!("{}{}", "0".repeat(padding), self);
    padded_string
  }
}

extern crate log;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  env_logger::Builder::from_env(Env::default().default_filter_or("info"))
    .init();
  let client = create_http_client();
  let args = env::args();
  let command = parse_args(args).unwrap();

  match command.t {
    CommandType::Help => {
      print_help();
    }
    CommandType::Top => {
      let anime_list = match api::get_top(&client).await {
        Ok(a) => a,
        Err(e) => {
          error!("Failed to get top anime");
          return Err(e);
        }
      };

      anime_list.iter().for_each(|anime| {
        println!("{:4} | {}", anime.id, anime.title);
      });
    }
    CommandType::New => {
      let anime_list = match api::get_new(&client).await {
        Ok(a) => a,
        Err(e) => {
          error!("Failed to get new anime");
          return Err(e);
        }
      };

      anime_list.iter().for_each(|anime| {
        println!("{:4} | {}", anime.id, anime.title);
      });
    }
    CommandType::Search => {
      let query = command.args.query;
      let (anime_list, _search_url) = match api::search(&client, &query).await {
        Ok(a) => a,
        Err(e) => {
          error!("Failed to search anime {}", query);
          return Err(e);
        }
      };

      println!("Results on {}", query);
      anime_list.iter().for_each(|anime| {
        println!("{:4} | {}", anime.id, anime.title);
      });
    }
    CommandType::List => {
      let anime_id = command.args.anime_id;
      let anime = match api::get_anime(&client, &anime_id).await {
        Ok(a) => a,
        Err(e) => {
          error!("Failed to get anime with id {}", anime_id);
          return Err(e);
        }
      };

      println!("{} episodes", anime_id);
      anime.episodes.iter().for_each(|episode| {
        println!("{:4} | {}", episode.num, episode.title);
      })
    }
    CommandType::Download => {
      let anime_id = command.args.anime_id;
      let episode_nums = command.args.episode_nums;
      let max_concurrent = command.args.max_concurrent;

      let anime = match api::get_anime(&client, &anime_id).await {
        Ok(a) => a,
        Err(e) => {
          error!("Failed to get anime with id {}", anime_id);
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
            error!("Episode with episode num {} not found", episode_num);
            return Ok(());
          }
        };

        let hls_url =
          match api::get_episode_hls(&client, &episode.url, &anime.info.url)
            .await
          {
            Ok(h) => h,
            Err(e) => {
              error!("unable to get episode hls");
              return Err(e);
            }
          };

        let path = format!("./{}", anime.info.title);
        let filename =
          format!("{}-{}.ts", episode.num.zero_pad(2), episode.title)
            .to_string()
            .sanitize();
        fs::create_dir_all(&path).unwrap();
        let filename = format!("{}/{}", path, filename);
        api::download_episode(&client, &hls_url, &filename, max_concurrent)
          .await?;
      }
    }
    CommandType::DownloadAll => {
      let anime_id = command.args.anime_id;
      let max_concurrent = command.args.max_concurrent;
      let anime = match api::get_anime(&client, &anime_id).await {
        Ok(a) => a,
        Err(e) => {
          error!("Failed to get anime with id {}", anime_id);
          return Err(e);
        }
      };

      for episode in anime.episodes {
        let hls_url =
          match api::get_episode_hls(&client, &episode.url, &anime.info.url)
            .await
          {
            Ok(h) => h,
            Err(e) => {
              error!("unable to get episode hls");
              return Err(e);
            }
          };

        let path = format!("./{}", &anime.info.title.sanitize());
        let filename =
          format!("{}-{}.ts", episode.num.zero_pad(2), episode.title)
            .to_string()
            .sanitize();
        fs::create_dir_all(&path).unwrap();
        let filename = format!("{}/{}", path, filename);
        api::download_episode(&client, &hls_url, &filename, max_concurrent)
          .await?;
      }
    }
    CommandType::Concat => {
      video::concat_ts();
    }
  }

  Ok(())
}
