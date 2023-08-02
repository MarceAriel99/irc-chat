use std::{error::Error, fmt};

#[derive(Debug)]
pub struct ServerError {
    pub kind: String,
    pub message: String,
}
impl Error for ServerError {}
impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
