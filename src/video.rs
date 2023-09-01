use std::{fs, io};

pub fn concat_ts() {
  let mut all_ts = fs::OpenOptions::new()
    .create_new(true)
    .append(true)
    .open("./all.ts")
    .unwrap();

  println!("\nCombining...");

  fs::read_dir("./")
    .unwrap()
    .map(|entry| entry.unwrap())
    .for_each(|entry| {
      if entry.file_name().to_str().unwrap().ends_with(".ts") {
        let mut video_ts = fs::OpenOptions::new()
          .read(true)
          .open(&entry.file_name())
          .unwrap();
        io::copy(&mut video_ts, &mut all_ts).unwrap();
      }
    });
}
