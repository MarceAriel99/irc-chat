//!
//! The connection listener listens to new connections and once they
//! are completed saves the new client information for the server to
//! use it
//!

use super::{channel::Channel, user::User};
use crate::{
    commands::{LOGIN, REGISTRATION, SERVER},
    custom_errors::{
        errors::{CRITICAL, NONCRITICAL},
        server_error::ServerError,
    },
    message::Message,
    server_utils::connection_handler::ConnectionHandler,
};
use std::{
    collections::HashMap,
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

pub struct ConnectionListener {
    pub address: String,
    pub users: Arc<Mutex<HashMap<String, User>>>,
    pub channels: Arc<Mutex<HashMap<String, Channel>>>,
    pub user_clients:
        Arc<Mutex<HashMap<String, (Option<JoinHandle<Result<(), ServerError>>>, Sender<Message>)>>>,
    pub server_clients:
        Arc<Mutex<HashMap<String, (Option<JoinHandle<Result<(), ServerError>>>, Sender<Message>)>>>,
    pub server_name: String,
    pub sender_to_server: Sender<Message>,
}

impl ConnectionListener {
    ///
    /// Reads every new connection and handles it
    ///
    pub fn read_new_connections(&mut self) -> Result<(), ServerError> {
        let listener = TcpListener::bind(self.address.clone()).map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Couldnt bind to address".to_string(),
            }
        })?;

        for stream in listener.incoming() {
            let stream = match stream {
                Ok(stream) => stream,
                Err(error) => {
                    return Err(ServerError {
                        kind: CRITICAL.to_string(),
                        message: error.to_string(),
                    })
                }
            };

            println!("New connection received");

            let users_clone = self.users.clone();
            let users_clients_clone = self.user_clients.clone();
            let server_clients_clone = self.server_clients.clone();
            let channels_clone = self.channels.clone();
            let sender_to_server_clone = self.sender_to_server.clone();
            let server_name_clone = self.server_name.clone();

            let _ = thread::spawn(move || {
                match Self::handle_connection(
                    stream,
                    users_clone,
                    channels_clone,
                    server_name_clone,
                    sender_to_server_clone,
                    users_clients_clone,
                    server_clients_clone,
                ) {
                    Ok(_) => {
                        println!("New connection");
                        Ok(())
                    }
                    Err(err) => Err(err),
                }
            });
        }

        Ok(())
    }

    ///
    /// Handles specific connection
    ///
    fn handle_connection(
        stream: TcpStream,
        users: Arc<Mutex<HashMap<String, User>>>,
        channels: Arc<Mutex<HashMap<String, Channel>>>,
        server_name: String,
        sender_to_server_clone: Sender<Message>,
        users_clients: Arc<
            Mutex<HashMap<String, (Option<JoinHandle<Result<(), ServerError>>>, Sender<Message>)>>,
        >,
        server_clients: Arc<
            Mutex<HashMap<String, (Option<JoinHandle<Result<(), ServerError>>>, Sender<Message>)>>,
        >,
    ) -> Result<(), ServerError> {
        println!("Handling connection in another thread");
        let (sender_read_new_connections, receiver_from_connection_hanlder): (
            Sender<Message>,
            Receiver<Message>,
        ) = mpsc::channel();
        let (tx_server, rx_user): (Sender<Message>, Receiver<Message>) = mpsc::channel();

        // Create connection handler from new connection
        let mut connection_handler = ConnectionHandler {
            stream,
            users,
            sender_to_server: sender_to_server_clone.clone(),
            sender_to_read_new_connections: Some(sender_read_new_connections),
            receiver: rx_user,
            channels,
            server_name,
        };

        // Spawn new thread for new client
        let handle = thread::spawn(move || {
            // connection succeeded
            match connection_handler.handle_client() {
                Ok(_) => {
                    println!("Client disconnected");
                    Ok(())
                }
                Err(err) => Err(err),
            }
        });

        // Wait for client to send message to server
        let message = Self::read_from_receiver(receiver_from_connection_hanlder)?;

        print!("Handling connection {:?} in new thread", message);

        // Check message command and handle it
        match message.command.as_str() {
            LOGIN => Self::save_user_connection(
                users_clients,
                message.prefix.clone().unwrap(),
                handle,
                tx_server,
            )?,
            REGISTRATION => Self::handle_registration(
                users_clients,
                message.prefix.clone().unwrap(),
                handle,
                tx_server,
                sender_to_server_clone,
                message.clone(),
            )?,
            SERVER => Self::handle_server(
                server_clients,
                handle,
                tx_server,
                sender_to_server_clone,
                message.clone(),
            )?,
            _ => {}
        }

        Ok(())
    }

    ///
    /// Reads from receiver specified
    ///
    fn read_from_receiver(receiver: Receiver<Message>) -> Result<Message, ServerError> {
        let message = receiver.recv().map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Couldn't receive".to_string(),
            }
        })?;

        Ok(message)
    }

    ///
    /// Saves user information from new connection
    ///
    fn save_user_connection(
        users_clients: Arc<
            Mutex<HashMap<String, (Option<JoinHandle<Result<(), ServerError>>>, Sender<Message>)>>,
        >,
        nickname: String,
        handle: JoinHandle<Result<(), ServerError>>,
        sender_to_client_handler: Sender<Message>,
    ) -> Result<(), ServerError> {
        let mut users_clients = users_clients.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Couldnt lock users clients".to_string(),
            }
        })?;
        users_clients.insert(nickname, (Some(handle), sender_to_client_handler));
        Ok(())
    }

    ///
    /// Handle registration. Saves new connection information and notifies server
    /// of the registration so it notifies all other servers.
    ///
    fn handle_registration(
        users_clients: Arc<
            Mutex<HashMap<String, (Option<JoinHandle<Result<(), ServerError>>>, Sender<Message>)>>,
        >,
        nickname: String,
        handle: JoinHandle<Result<(), ServerError>>,
        sender_to_client_handler: Sender<Message>,
        sender_to_server: Sender<Message>,
        message: Message,
    ) -> Result<(), ServerError> {
        Self::save_user_connection(users_clients, nickname, handle, sender_to_client_handler)?;
        //Notify struct server so it notifies other servers
        sender_to_server.send(message).map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Couldnt send".to_string(),
            }
        })?;
        Ok(())
    }

    ///
    /// Handles server connection. Saves server information and notifies server
    ///
    fn handle_server(
        server_clients: Arc<
            Mutex<HashMap<String, (Option<JoinHandle<Result<(), ServerError>>>, Sender<Message>)>>,
        >,
        handle: JoinHandle<Result<(), ServerError>>,
        sender_to_client_handler: Sender<Message>,
        sender_to_server: Sender<Message>,
        message: Message,
    ) -> Result<(), ServerError> {
        server_clients.lock().unwrap().insert(
            message.params[0][0].clone(),
            (Some(handle), sender_to_client_handler),
        );
        sender_to_server.send(message).map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Couldnt send".to_string(),
            }
        })?;
        Ok(())
    }
}
