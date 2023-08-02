//!
//! This module handles messages beeing sent between servers
//!

use std::sync::mpsc::Sender;

use crate::custom_errors::{errors::CRITICAL, server_error::ServerError};

use crate::message::Message;

///
/// This function is called when a server receives a privmsg from another server. It will receive it and send it to its main
/// thread where it will be processed. If it arrives here, it was already checked that the user exists.
///
pub fn handle_privmsg_server(
    message: Message,
    sender: &Sender<Message>,
) -> Result<(), ServerError> {
    println!("Privmsg in server handler, message: {:?}", message);
    sender.send(message).map_err(|_| -> ServerError {
        ServerError {
            kind: CRITICAL.to_string(),
            message: "Could not send message".to_string(),
        }
    })?;

    Ok(())
}
