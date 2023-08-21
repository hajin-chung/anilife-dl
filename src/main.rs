use std::env;

use anilife_rs::{api, http::create_http_client};

#[macro_use]
extern crate log;

fn print_help() {
  println!("anime-dl");
  println!("Usage: ");
  println!("  anime-dl --search <query>");
  println!("  anime-dl --list <anime_id>");
  println!("  anime-dl --download <anime_id> <episode_num>");
  println!("Options: ");
  println!("  -h --help     Show this screen");
  println!("  -s --search   Search anime with title");
  println!("  -l --list     List episodes of anime");
  println!("  -d --download  Download episode of that index");
}

fn print_error(message: &str) {
  error!("{}", message);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let _log2 = log2::open(".log.tmp").start();
  let client = create_http_client();
  let mut args = env::args();

  if args.len() == 1 {
    print_help();
    return Ok(());
  }

  while let Some(arg) = args.next() {
    match arg.as_str() {
      "-h" | "--help" => {
        print_help();
        break;
      }
      "-s" | "--search" => {
        let query = match args.next() {
          Some(q) => q,
          None => {
            print_error("Search query is missing");
            break;
          }
        };

        let (anime_list, _search_url) = api::search(&client, &query).await?;
        println!("Results on {}", query);
        anime_list.iter().for_each(|anime| {
          println!("{:4} | {}", anime.id, anime.title);
        })
      }
      "-l" | "--list" => {
        let anime_id = match args.next() {
          Some(i) => i,
          None => {
            print_error("Anime id is missing");
            break;
          }
        };
        let anime = api::get_anime(&client, &anime_id).await?;

        println!("{} episodes", anime_id);
        anime.episodes.iter().for_each(|episode| {
          println!("{:4} | {}", episode.num, episode.title);
        })
      }
      "-d" | "--download" => {
        let anime_id = match args.next() {
          Some(i) => i,
          None => {
            print_error("Anime id is missing");
            break;
          }
        };
        let episode_num = match args.next() {
          Some(i) => i,
          None => {
            print_error("Episdoe num is missing");
            break;
          }
        };

        let anime = api::get_anime(&client, &anime_id).await?;
        let episode = match anime
          .episodes
          .iter()
          .find(|episode| episode.num == episode_num)
        {
          Some(e) => e,
          None => {
            print_error(format!("Episode with episode num {} not found", episode_num).as_str());
            break;
          }
        };

        let hls_url = api::get_episode_hls(&client, &episode.url, &anime.info.url).await?;

        let filename = format!("{}-{}-{}", anime.info.title, episode.num, episode.title);
        api::download_episode(&client, &hls_url, &filename).await?;
      }
      _ => {}
    }
  }

  Ok(())
}
