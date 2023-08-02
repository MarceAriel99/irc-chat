use std::{error::Error, fmt};

#[derive(Debug)]
pub struct ClientError {
    pub kind: String,
    pub message: String,
}

impl Error for ClientError {}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
