use std::error;
use std::result;

/// Just put any error in a box.
pub type Result<T> = result::Result<T, Box<dyn error::Error + Send + Sync>>;
