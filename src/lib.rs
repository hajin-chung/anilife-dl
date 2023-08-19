use std::error::Error;

pub mod api;
pub mod dl;

pub type AsyncResult<T> = Result<T, Box<dyn Error>>;
