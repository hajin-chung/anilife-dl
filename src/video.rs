use std::{
  fs::{self},
  io::{self},
};

use log::{error, info};

use crate::cli::print_progress;

pub fn concat_ts() {
  info!("Combining...");

  fs::remove_file("./all.ts").unwrap_or_else(|err_msg| {
    error!("{}", err_msg);
  });

  let mut file_names: Vec<String> = fs::read_dir("./")
    .unwrap()
    .map(|entry| entry.unwrap())
    .map(|entry| entry.file_name().to_str().unwrap().to_string())
    .collect();
  let mut count = 0;
  let len = file_names.len();

  file_names.sort();
  let mut all_ts = fs::OpenOptions::new()
    .create_new(true)
    .append(true)
    .open("./all.ts")
    .unwrap();

  file_names.iter().for_each(|file_name| {
    if file_name.ends_with(".ts") {
      let mut video_ts =
        fs::OpenOptions::new().read(true).open(file_name).unwrap();
      io::copy(&mut video_ts, &mut all_ts).unwrap();

      count += 1;
      print_progress(file_name, count, len);
    }
  });
}
