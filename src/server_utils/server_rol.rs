//!
//! Server Rol represents Main Server and Secondary Server
//!

use crate::custom_errors::server_error::ServerError;
use crate::message::Message;
use std::{
    collections::HashMap,
    sync::{mpsc::Sender, Arc, Mutex},
    thread::JoinHandle,
};

use super::{channel::Channel, user::User};

pub trait ServerRol {
    fn handle_server(
        &mut self,
        message: Message,
        handle: JoinHandle<Result<(), ServerError>>,
        sender: Sender<Message>,
        server_name: String,
        users: Arc<Mutex<HashMap<String, User>>>,
        channels: Arc<Mutex<HashMap<String, Channel>>>,
    ) -> Result<(), ServerError>;
    fn send_message_to_server(
        &self,
        message: Message,
        server_name: String,
    ) -> Result<(), ServerError>;
    fn notify(&self, message: Message) -> Result<(), ServerError>;
    fn notify_all_but(&mut self, message: Message, server_name: &str) -> Result<(), ServerError>;
    fn check_server_existance(&mut self, message: Message) -> Result<(), ServerError>;
}
