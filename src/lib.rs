use std::error::Error;

pub mod api;
pub mod crypto;

pub type AsyncResult<T> = Result<T, Box<dyn Error>>;
