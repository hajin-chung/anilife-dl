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
  println!("  -l --list      List episodes of anime");
  println!("  -d --download  Download episode of that index");
  println!("  --all          Download all episodes");
  println!("  -u --upload    Upload file to youtube");
}

pub enum Command {
  Search(String),
  List(String),
  Download(String, Vec<String>),
  DownloadAll(String),
  Upload(String),
  Help,
  None,
}

pub fn parse_args(mut args: Args) -> Result<Command, String> {
  if args.len() == 1 {
    return Ok(Command::Help);
  }

  let mut command = Command::None;
  let mut selected_anime_id = "".to_string();

  while let Some(arg) = args.next() {
    match arg.as_str() {
      "-h" | "--help" => {
        command = Command::Help;
      }
      "-s" | "--search" => {
        let query = match args.next() {
          Some(q) => q,
          None => {
            eprintln!("Search query is missing");
            return Err("error".to_string());
          }
        };
        command = Command::Search(query);
      }
      "-a" | "--anime" => {
        let anime_id = match args.next() {
          Some(i) => i,
          None => {
            eprintln!("Anime id is missing");
            return Err("error".to_string());
          }
        };
        selected_anime_id = anime_id;
        command = Command::List(selected_anime_id.clone());
      }
      "-l" | "--list" => {
        command = Command::List(selected_anime_id.clone());
      }
      "-d" | "--download" => {
        let episode_nums = match args.next() {
          Some(i) => i,
          None => {
            eprintln!("Episdoe num is missing");
            return Err("error".to_string());
          }
        };
        let episode_num_vec: Vec<String> = episode_nums.split(',').map(|e| e.to_string()).collect();
        command = Command::Download(selected_anime_id.clone(), episode_num_vec);
      }
      "-u" | "--upload" => {
        let filename = match args.next() {
          Some(i) => i,
          None => {
            eprintln!("file name is missing");
            return Err("error".to_string());
          }
        };
        command = Command::Upload(filename);
      }
      "--all" => {
        command = Command::DownloadAll(selected_anime_id.clone());
      }
      _ => {}
    }
  }

  Ok(command)
}
