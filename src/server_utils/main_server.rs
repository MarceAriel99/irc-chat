//!
//! Main Server represents the principal server to which all other
//! servers (Secondary Servers) connect
//!

use std::{
    collections::HashMap,
    sync::{mpsc::Sender, Arc, Mutex},
    thread::JoinHandle,
};

use super::{channel::Channel, server_rol::ServerRol, user::User};
use crate::custom_errors::server_error::ServerError;
use crate::{
    commands::{SERVER, USERS_INFO},
    custom_errors::errors::{CRITICAL, NONCRITICAL},
    message::Message,
};

pub struct MainServer {
    servers: HashMap<String, (JoinHandle<Result<(), ServerError>>, Sender<Message>)>,
}

impl MainServer {
    ///
    /// Creates Main Server
    ///
    pub fn new() -> Self {
        MainServer {
            servers: HashMap::new(),
        }
    }

    ///
    /// Sends  the information of all the users known in the server
    ///
    fn send_users(
        &self,
        users: Arc<Mutex<HashMap<String, User>>>,
        sender: &Sender<Message>,
    ) -> Result<(), ServerError> {
        let users = users.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Couldn't lock users".to_string(),
            }
        })?;
        for user in users.values() {
            let params = vec![
                vec![
                    user.nickname.clone(),
                    user.address.clone(),
                    user.username.clone(),
                    user.server_name.clone(),
                    user.password.clone(),
                ],
                vec![user.real_name.clone()],
            ];
            let message = Message {
                prefix: Some(user.clone().nickname),
                command: USERS_INFO.to_string(),
                params,
            };
            println!("sending: {:?}", message);
            sender.send(message).map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Couldn't send message".to_string(),
                }
            })?;
        }
        Ok(())
    }

    ///
    /// Sends all the channels known in the server
    ///
    fn send_channels(
        &self,
        sender: &Sender<Message>,
        channels: Arc<Mutex<HashMap<String, Channel>>>,
    ) -> Result<(), ServerError> {
        let channels = channels.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Couldn't lock channels".to_string(),
            }
        })?;
        for channel in channels.values() {
            let message = channel.channel_to_message();
            sender.send(message).map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Couldn't send message".to_string(),
                }
            })?;
        }
        Ok(())
    }
}

impl ServerRol for MainServer {
    ///
    /// Adds new secondary server. Notifies all secondary servers
    ///
    fn handle_server(
        &mut self,
        message: Message,
        handle: JoinHandle<Result<(), ServerError>>,
        sender: Sender<Message>,
        act_server_name: String,
        users: Arc<Mutex<HashMap<String, User>>>,
        channels: Arc<Mutex<HashMap<String, Channel>>>,
    ) -> Result<(), ServerError> {
        let server_name = message.params[0][0].clone();

        // Already exists a server with that name, it cant connect
        if self.servers.contains_key(server_name.as_str()) {
            println!("Server already exists");
            sender
                .send(Message {
                    prefix: None,
                    command: SERVER.to_string(),
                    params: vec![vec![]],
                })
                .map_err(|_| -> ServerError {
                    ServerError {
                        kind: CRITICAL.to_string(),
                        message: "Couldn't send".to_string(),
                    }
                })?;
        } else {
            // save in hashmap the server with its sender and handler
            self.servers.insert(server_name.clone(), (handle, sender));
            let mut message = message;
            message.prefix = Some(act_server_name);
            self.notify(message)?; // This is to notify the other servers of the new one
                                   // SERVER should be added to the handler of client handler to notify
            let sender = self.servers.get(server_name.as_str()).unwrap().1.clone();
            self.send_users(users, &sender)?;
            self.send_channels(&sender, channels)?;
        }

        Ok(())
    }

    ///
    /// Sends the message received to all the secondary servers
    ///
    fn notify(&self, message: Message) -> Result<(), ServerError> {
        println!("enviando confirmacion");
        println!("self.servers: {:?}", self.servers.values());
        for (_, sender) in self.servers.values() {
            println!("enviando confirmacion ahora si");
            sender.send(message.clone()).map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Couldn't send 1".to_string(),
                }
            })?;
        }

        Ok(())
    }

    ///
    /// Sends amessage to a server, if it exists
    ///
    fn send_message_to_server(
        &self,
        message: Message,
        server_name: String,
    ) -> Result<(), ServerError> {
        let server = match self.servers.get(server_name.as_str()) {
            Some(server) => server,
            None => {
                return Err(ServerError {
                    kind: NONCRITICAL.to_string(),
                    message: "Server doesn't exist".to_string(),
                })
            }
        };
        println!("Sending message to server {}", server_name);
        server.1.send(message).map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Couldn't send".to_string(),
            }
        })?;
        Ok(())
    }

    ///
    /// CHecks if a server exists, if it does it sendes a message to it.
    ///
    fn check_server_existance(&mut self, message: Message) -> Result<(), ServerError> {
        let server_name = &message.params[0][0];
        let server = match self.servers.get(server_name.as_str()) {
            Some(server) => server,
            None => {
                return Err(ServerError {
                    kind: NONCRITICAL.to_string(),
                    message: "Server doesn't exist".to_string(),
                })
            }
        };
        server.1.send(message).map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Couldn't send".to_string(),
            }
        })?;
        Ok(())
    }

    ///
    /// Sends a message to all the servers except the one whose name was received.
    ///
    fn notify_all_but(
        &mut self,
        message: Message,
        server_name_skiping: &str,
    ) -> Result<(), ServerError> {
        for (server_name, (_, sender)) in &self.servers {
            if server_name == server_name_skiping {
                println!("notifying servers but skiping {}", server_name);
                continue;
            }
            sender.send(message.clone()).map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Couldn't send 1".to_string(),
                }
            })?;
        }

        Ok(())
    }
}

// Default implementation, required by the clippy linter
impl Default for MainServer {
    fn default() -> Self {
        Self::new()
    }
}
