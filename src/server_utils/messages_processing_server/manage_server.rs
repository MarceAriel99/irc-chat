//!
//! This module handles server related messages between servers
//!

use std::{
    io::Write,
    net::TcpStream,
    sync::mpsc::{Receiver, Sender},
};

use crate::custom_errors::{errors::CRITICAL, server_error::ServerError};

use crate::message::Message;

///
/// This function is called when it receives an squit from a server. It check ig it is an operator if not it will send
/// the answer to the asking server, if it is an operator it will close in the main thread.
///
pub fn handle_squit(
    message: Message,
    sender: &Sender<Message>,
    receiver: &Receiver<Message>,
    mut stream: &TcpStream,
) -> Result<(), ServerError> {
    println!("Squit in server handler");
    sender.send(message).map_err(|_| -> ServerError {
        ServerError {
            kind: CRITICAL.to_string(),
            message: "Could not send message".to_string(),
        }
    })?;
    let answer = receiver.recv().map_err(|_| -> ServerError {
        ServerError {
            kind: CRITICAL.to_string(),
            message: "Could not receive message".to_string(),
        }
    })?;
    if answer.params[0][0] == "You are not an operator" {
        stream
            .write_all(answer.as_string().as_bytes())
            .map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Could not write to stream".to_string(),
                }
            })?;
    }
    Ok(())
}
