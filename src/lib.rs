use std::error::Error;

pub mod http;
pub mod api;

pub type AsyncResult<T> = Result<T, Box<dyn Error>>;
