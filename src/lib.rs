use std::error::Error;

pub mod http;
pub mod api;
pub mod dl;

pub type AsyncResult<T> = Result<T, Box<dyn Error>>;
