use std::{env, error::Error, fs};

use http::create_http_client;
use log::error;
use regex::Regex;

pub mod api;
pub mod cli;
pub mod http;

use cli::{parse_args, print_help, CommandType};

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
  env_logger::init();
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
          format!("{}-{}-{}.ts", anime.info.title, episode.num, episode.title)
            .to_string();
        let filename = sanitize_filename(&filename);
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

        let path = format!("./{}", anime.info.title);
        let filename =
          format!("{}-{}-{}.ts", anime.info.title, episode.num, episode.title)
            .to_string();
        let filename = sanitize_filename(&filename);
        fs::create_dir_all(&path).unwrap();
        let filename = format!("{}/{}", path, filename);
        api::download_episode(&client, &hls_url, &filename, max_concurrent)
          .await?;
      }
    }
  }

  Ok(())
}
