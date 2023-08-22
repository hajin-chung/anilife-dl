use reqwest::{header, Client};

use crate::api;

pub fn create_http_client() -> Client {
  let mut headers = header::HeaderMap::new();
  headers.insert(
    "User-Agent",
    header::HeaderValue::from_static(api::USER_AGENT),
  );

  Client::builder().default_headers(headers).build().unwrap()
}
