use std::error::Error;
use std::result;

/// Just put any error in a box.
pub type Result<T> = result::Result<T, Box<Error>>;
