use std::env::Args;

pub fn print_help() {
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
  println!("  -t --top       get Top 20 animes");
  println!("  -n --new       recently updated animes");
  println!("  -l --list      List episodes of anime");
  println!("  -d --download  Download episode of that index");
  println!("  --all          Download all episodes");
  println!("  -u --upload    Upload file to youtube");
}

pub enum CommandType {
  Search,
  List,
  Download,
  DownloadAll,
  Top,
  New,
  Upload,
  Help,
}

#[derive(Default)]
pub struct CommandArgs {
  pub anime_id: String,
  pub query: String,
  pub episode_nums: Vec<String>,
  pub filename: String,
  pub max_concurrent: usize,
}

pub struct Command {
  pub t: CommandType,
  pub args: CommandArgs,
}

const DEFAULT_MAX_CONCURRENT: usize = 100;

pub fn parse_args(mut args: Args) -> Result<Command, String> {
  if args.len() == 1 {
    return Ok(Command {
      t: CommandType::Help,
      args: CommandArgs::default(),
    });
  }

  let mut command_type = CommandType::Help;
  let mut command_args = CommandArgs::default();
  command_args.max_concurrent = DEFAULT_MAX_CONCURRENT;

  while let Some(arg) = args.next() {
    match arg.as_str() {
      "-h" | "--help" => {
        command_type = CommandType::Help;
      }
      "-t" | "--top" => {
        command_type = CommandType::Top;
      }
      "-n" | "--new" => {
        command_type = CommandType::New;
      }
      "-s" | "--search" => {
        let query = match args.next() {
          Some(q) => q,
          None => {
            eprintln!("Search query is missing");
            return Err("error".to_string());
          }
        };

        command_type = CommandType::Search;
        command_args.query = query;
      }
      "-a" | "--anime" => {
        let anime_id = match args.next() {
          Some(i) => i,
          None => {
            eprintln!("Anime id is missing");
            return Err("error".to_string());
          }
        };

        command_type = CommandType::List;
        command_args.anime_id = anime_id;
      }
      "-l" | "--list" => {
        command_type = CommandType::List;
      }
      "-d" | "--download" => {
        let episode_nums = match args.next() {
          Some(i) => i,
          None => {
            eprintln!("Episdoe num is missing");
            return Err("error".to_string());
          }
        };
        let episode_num_vec: Vec<String> =
          episode_nums.split(',').map(|e| e.to_string()).collect();

        command_type = CommandType::Download;
        command_args.episode_nums = episode_num_vec;
      }
      "-u" | "--upload" => {
        let filename = match args.next() {
          Some(i) => i,
          None => {
            eprintln!("file name is missing");
            return Err("error".to_string());
          }
        };
        command_type = CommandType::Upload;
        command_args.filename = filename;
      }
      "-m" | "--max-concurrent" => {
        let max_concurrent = match args.next() {
          Some(m) => m.parse::<usize>().unwrap(),
          None => {
            eprintln!("max concurrent is missing");
            return Err("max concurrent is missing".to_string());
          }
        };
        command_args.max_concurrent = max_concurrent;
      }
      "--all" => {
        command_type = CommandType::DownloadAll;
      }
      _ => {}
    }
  }

  Ok(Command {
    t: command_type,
    args: command_args,
  })
}
