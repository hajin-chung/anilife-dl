use std::error::Error;

use regex::Regex;

pub mod api;
pub mod http;

pub type AsyncResult<T> = Result<T, Box<dyn Error>>;

pub fn sanitize_filename(filename: &str) -> String {
  let forbidden_pattern = r#"[<>:"/\\|?*]|[\x00-\x1F]"#;
  let re = Regex::new(forbidden_pattern).unwrap();
  let sanitized_filename = re.replace_all(filename, "").to_string();
  sanitized_filename
}
