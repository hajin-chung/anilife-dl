use std::{fs, io};

pub fn concat_ts(path: String) {
  println!("{}", path);
  let mut all_ts = fs::OpenOptions::new()
    .create_new(true)
    .append(true)
    .open(format!("./{}/all.ts", path))
    .unwrap();

  println!("\nCombining...");

  fs::read_dir(path)
    .unwrap()
    .map(|entry| entry.unwrap())
    .for_each(|entry| {
      let mut video_ts = fs::OpenOptions::new()
        .read(true)
        .open(&entry.file_name())
        .unwrap();
      io::copy(&mut video_ts, &mut all_ts).unwrap();
    });
}
