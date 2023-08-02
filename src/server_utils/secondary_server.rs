//!
//! Secondary Server represents the server that connects to the
//! main one (Main Server)
//!

use crate::{
    commands::SERVER_EXISTS, custom_errors::errors::CRITICAL, message::Message,
    server_utils::connection_handler::ConnectionHandler,
};
use std::{
    collections::HashMap,
    net::TcpStream,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

use super::{channel::Channel, server_rol::ServerRol, user::User};
use crate::custom_errors::server_error::ServerError;

pub struct SecondaryServer {
    main_sender: Sender<Message>,
    _main_join_handler: JoinHandle<()>,
    main_name: String,
}

impl SecondaryServer {
    ///
    /// Creates a Secondary Server. Connects to main server
    ///
    pub fn new(
        users: Arc<Mutex<HashMap<String, User>>>,
        sender_to_server: Sender<Message>,
        channels: Arc<Mutex<HashMap<String, Channel>>>,
        server_name: String,
        main_server_data: (String, String),
    ) -> Result<Self, ServerError> {
        let main_server_data = connect_to_main_server(
            users,
            sender_to_server,
            channels,
            server_name,
            main_server_data,
        )?;

        let secondary_server = SecondaryServer {
            main_sender: main_server_data.2,
            _main_join_handler: main_server_data.1,
            main_name: main_server_data.0,
        };

        Ok(secondary_server)
    }
}

impl ServerRol for SecondaryServer {
    ///
    /// Receives a message with server command
    ///
    fn handle_server(
        &mut self,
        _message: Message,
        _handle: JoinHandle<Result<(), ServerError>>,
        _sender: Sender<Message>,
        _server_name: String,
        _users: Arc<Mutex<HashMap<String, User>>>,
        _channels: Arc<Mutex<HashMap<String, Channel>>>,
    ) -> Result<(), ServerError> {
        println!("trying to connect server to main server");
        Ok(())
    }

    ///
    /// Sends the received message to the main server
    ///
    fn notify(&self, message: Message) -> Result<(), ServerError> {
        self.main_sender.send(message).map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Couldn't send 3".to_string(),
            }
        })?;
        Ok(())
    }

    ///
    /// Sends a message to the main server so that it can send it to the correct server
    ///
    fn send_message_to_server(
        &self,
        message: Message,
        _server_name: String,
    ) -> Result<(), ServerError> {
        self.notify(message)
    }

    ///
    /// Checks if a server exists in the main server
    ///
    fn check_server_existance(&mut self, message: Message) -> Result<(), ServerError> {
        let mut request = message;
        request.command = SERVER_EXISTS.to_string();
        self.main_sender.send(request).map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Could not send message".to_string(),
            }
        })?;

        Ok(())
    }

    ///
    /// Will send the message to the main server unless it is the server that is told to skip
    ///
    fn notify_all_but(
        &mut self,
        message: Message,
        server_name_skiping: &str,
    ) -> Result<(), ServerError> {
        if server_name_skiping == self.main_name.as_str() {
            println!(
                "JOIN, SECONDARY Not sending to main {}",
                self.main_name.as_str()
            );
            return Ok(());
        };

        self.notify(message)
    }
}

///
/// Connects to main server and returns its data
///
fn connect_to_main_server(
    users: Arc<Mutex<HashMap<String, User>>>,
    sender_to_server: Sender<Message>,
    channels: Arc<Mutex<HashMap<String, Channel>>>,
    server_name: String,
    main_server_data: (String, String),
) -> Result<(String, JoinHandle<()>, Sender<Message>), ServerError> {
    println!("Connecting to main server{:?}", main_server_data);
    let address = main_server_data.1;
    println!("Connecting to main server in addres:{:?}", address);

    // Connect to address
    let stream = TcpStream::connect(address).map_err(|_| -> ServerError {
        ServerError {
            kind: CRITICAL.to_string(),
            message: "Couldn't connect to main server".to_string(),
        }
    })?;

    // Communication channel from secondary server to main server handler thread
    let (sender_to_handler, receiver_from_server): (Sender<Message>, Receiver<Message>) =
        mpsc::channel();

    // Create connection handler form new connection
    let mut connection_handler = ConnectionHandler {
        stream,
        users,
        sender_to_server,
        receiver: receiver_from_server,
        channels,
        server_name,
        sender_to_read_new_connections: None,
    };

    let handle = thread::spawn(move || {
        // connection succeeded
        match connection_handler.connect_to_main_server() {
            Ok(_) => {
                println!("Server disconnected");
            }
            Err(err) => println!("Error: {}", err),
        }
    });

    Ok((main_server_data.0, handle, sender_to_handler))
}
