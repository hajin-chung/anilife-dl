use std::env::{self, Args};

use anilife_rs::{api, http::create_http_client, sanitize_filename, upload::upload};

#[macro_use]
extern crate log;

fn print_help() {
  println!("anime-dl");
  println!("Usage: ");
  println!("  anime-dl --search <query>");
  println!("  anime-dl --anime <anime_id> --list");
  println!("  anime-dl --anime <anime_id> --<episode_num1>,<episode_num2>,...");
  println!("  anime-dl --anime <anime_id> --all");
  println!("  anime-dl --upload <filename>");
  println!("Options: ");
  println!("  -h --help      Show this screen");
  println!("  -s --search    Search anime with title");
  println!("  -l --list      List episodes of anime");
  println!("  -d --download  Download episode of that index");
  println!("  --all          Download all episodes");
  println!("  -u --upload    Upload file to youtube");
}

fn print_error(message: &str) {
  error!("{}", message);
}

enum CommandType {
  Search,
  List,
  Download,
  DownloadAll,
  Upload,
  Help,
  None,
}

fn parse_args(mut args: Args) -> Result<(CommandType, Vec<String>), String> {
  if args.len() == 1 {
    return Ok((CommandType::Help, vec![]));
  }

  let mut command_type: CommandType = CommandType::None;
  let mut params: Vec<String> = Vec::new();

  while let Some(arg) = args.next() {
    match arg.as_str() {
      "-h" | "--help" => {
        command_type = CommandType::Help;
      }
      "-s" | "--search" => {
        command_type = CommandType::Search;
        let query = match args.next() {
          Some(q) => q,
          None => {
            print_error("Search query is missing");
            return Err("error".to_string());
          }
        };
        params.push(query);
      }
      "-a" | "--anime" => {
        command_type = CommandType::List;
        let anime_id = match args.next() {
          Some(i) => i,
          None => {
            print_error("Anime id is missing");
            return Err("error".to_string());
          }
        };
        params.push(anime_id);
      }
      "-l" | "--list" => {
        command_type = CommandType::List;
      }
      "-d" | "--download" => {
        command_type = CommandType::Download;
        let episode_nums = match args.next() {
          Some(i) => i,
          None => {
            print_error("Episdoe num is missing");
            return Err("error".to_string());
          }
        };
        let episode_num_vec: Vec<String> = episode_nums.split(',').map(|e| e.to_string()).collect();

        params = [params, episode_num_vec].concat();
      }
      "-u" | "--upload" => {
        command_type = CommandType::Upload;
        let filename = match args.next() {
          Some(i) => i,
          None => {
            print_error("file name is missing");
            return Err("error".to_string());
          }
        };
        params.push(filename);
      }
      "--all" => {
        command_type = CommandType::DownloadAll;
      }
      _ => {}
    }
  }

  Ok((command_type, params))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let _log2 = log2::open(".log.tmp").start();
  let client = create_http_client();
  let args = env::args();
  let (command_type, args) = parse_args(args).unwrap();

  match command_type {
    CommandType::Help => {
      print_help();
    }
    CommandType::Search => {
      let query = &args[0];

      let (anime_list, _search_url) = api::search(&client, query).await?;
      println!("Results on {}", query);
      anime_list.iter().for_each(|anime| {
        println!("{:4} | {}", anime.id, anime.title);
      });
    }
    CommandType::List => {
      let anime_id = &args[0];
      let anime = api::get_anime(&client, anime_id).await?;

      println!("{} episodes", anime_id);
      anime.episodes.iter().for_each(|episode| {
        println!("{:4} | {}", episode.num, episode.title);
      })
    }
    CommandType::Download => {
      let anime_id = &args[0];

      for episode_num in args.iter().skip(1) {
        let anime = api::get_anime(&client, anime_id).await?;
        let episode = match anime
          .episodes
          .iter()
          .find(|episode| episode.num.eq(episode_num))
        {
          Some(e) => e,
          None => {
            print_error(format!("Episode with episode num {} not found", episode_num).as_str());
            return Ok(());
          }
        };

        let hls_url = api::get_episode_hls(&client, &episode.url, &anime.info.url).await?;

        let filename = format!("{}-{}-{}", anime.info.title, episode.num, episode.title);
        let filename = sanitize_filename(&filename);
        api::download_episode(&client, &hls_url, &filename).await?;
      }
    }
    CommandType::DownloadAll => {
      let anime_id = &args[0];
      let anime = api::get_anime(&client, anime_id).await?;
      for episode in anime.episodes {
        let hls_url = api::get_episode_hls(&client, &episode.url, &anime.info.url).await?;

        let filename = format!("{}-{}-{}", anime.info.title, episode.num, episode.title);
        let filename = sanitize_filename(&filename);
        api::download_episode(&client, &hls_url, &filename).await?;
      }
    }
    CommandType::Upload => {
      let filename = &args[0];
      upload(filename).await?;
    }
    CommandType::None => {}
  }

  Ok(())
}
