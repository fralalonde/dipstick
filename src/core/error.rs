use std::result;
use std::error;

/// Just put any error in a box.
pub type Result<T> = result::Result<T, Box<error::Error + Send + Sync>>;

