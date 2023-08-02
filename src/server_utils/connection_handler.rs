//!
//! Connection Handler receives new connections and will instantiate the correct
//! handler for each connection.
//!

use std::{
    collections::HashMap,
    io::Write,
    io::{BufRead, BufReader},
    net::TcpStream,
    sync::{mpsc::Receiver, mpsc::Sender, Arc, Mutex},
};

use crate::{
    commands::{LOGIN, SERVER},
    custom_errors::errors::{CRITICAL, NONCRITICAL},
    message::Message,
    numeric_reply::{NumericReply, ERR_ALREADYREGISTRED_MSG, ERR_ALREADYREGISTRED_NUM},
    parser::parse,
    server_utils::{
        channel::Channel,
        client_handler::ClientHandler,
        messages_processing_client::connection_and_registration::{
            get_action, get_nickname, get_password, get_user_info, login_user, registrate_user,
        },
        user::User,
    },
};

use crate::custom_errors::server_error::ServerError;
pub struct ConnectionHandler {
    pub stream: TcpStream,
    pub sender_to_server: Sender<Message>,
    pub sender_to_read_new_connections: Option<Sender<Message>>,
    pub receiver: Receiver<Message>,
    pub users: Arc<Mutex<HashMap<String, User>>>,
    pub channels: Arc<Mutex<HashMap<String, Channel>>>,
    pub server_name: String,
}

impl ConnectionHandler {
    ///
    /// Reads messages from the client and carries out the request.
    /// This function is used by the receiver of a connection
    ///
    pub fn handle_client(&mut self) -> Result<(), ServerError> {
        // Wait until there's data to read
        let socket = self.stream.try_clone().map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Could not clone stream".to_string(),
            }
        })?;

        let mut reader = BufReader::new(socket);
        let client_name;
        let mut user = None;

        // Create handler from data received
        match self.handle_new_connection(&mut reader) {
            Ok(new_user) => match new_user {
                Some(new_user) => {
                    client_name = new_user.nickname.clone();
                    user = Some(new_user);
                }
                None => {
                    client_name = self.server_name.clone();
                }
            },
            Err(e) => return Err(e),
        }

        let mut handler = ClientHandler {
            stream: &self.stream,
            users: self.users.clone(),
            sender: self.sender_to_server.clone(),
            receiver: &self.receiver,
            channels: self.channels.clone(),
            client_name,
            user,
            reader,
        };

        handler.handle_client()?;

        Ok(())
    }

    /* FUNCTION TO HANDLE NEW CONNECTION (login and registration) */

    ///
    /// Handles the new connection to the server. Keeps on handling login and registration
    /// until the user logins or registration correctly. Returns a user or a Server Error
    /// if an error was found.
    ///
    fn handle_new_connection(
        &mut self,
        reader: &mut BufReader<TcpStream>,
    ) -> Result<Option<User>, ServerError> {
        let action_message = self.process_data(reader)?;
        println!("Action message: {:?}", action_message);
        let action = get_action(&action_message).map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Couldn't connect to main server".to_string(),
            }
        })?;

        if action == SERVER {
            println!("Connecting to server");
            self.connect_server(action_message)?;
            return Ok(None); // Return None because it's a server
        }

        // Create user from data received
        // Handle new client and proceed with login or registration
        let mut user = None;
        let mut action_clone = Some(action.clone());

        // Until a user is correctly resistered or loged in
        while user.is_none() {
            user = match self.handle_login_registration(reader, action_clone.clone()) {
                Ok(result) => {
                    match result {
                        Some(user) => Some(user),
                        // If no user was returned the the user information
                        // was not given correctly and the client can try again
                        None => {
                            // If login/registration was incorrect then action must be read again
                            action_clone = None;
                            continue;
                        }
                    }
                }
                // An error was found then the server is notified and the client is disconnected
                Err(err) => return Err(err),
            };
        }

        println!("User: {:?}", user);
        let new_user = user.clone().unwrap();
        let params = vec![
            vec![
                new_user.nickname.clone(),
                new_user.address.clone(),
                new_user.username.clone(),
                new_user.server_name.clone(),
                new_user.password.clone(),
            ],
            vec![new_user.real_name.clone()],
        ];

        let message = Message {
            prefix: Some(new_user.nickname),
            command: action,
            params,
        };

        self.sender_to_read_new_connections
            .clone()
            .unwrap()
            .send(message)
            .map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Could not send message".to_string(),
                }
            })?;

        Ok(user)
    }

    ///
    /// This function handles the connection of a new server. This is the main server handling the connection of a secondary
    /// Server. It will receive the SERVER command and the server name and will check if the server is already connected.
    ///
    pub fn connect_server(&mut self, message: Message) -> Result<(), ServerError> {
        //send to main thread so that it checks if it is possible and save it
        self.sender_to_read_new_connections
            .as_ref()
            .unwrap()
            .send(message)
            .map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Could not send to server".to_string(),
                }
            })?;

        let answer = self.receiver.recv().map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Could not receive from server".to_string(),
            }
        })?;

        // if the answer has no params means that it already exists so send ERR_ALREADYREGISTERED
        if answer.params_total_count() == 0 {
            println!("server already registered");
            let reply = NumericReply::new(ERR_ALREADYREGISTRED_NUM, ERR_ALREADYREGISTRED_MSG, None);
            self.stream
                .write_all(reply.as_string().as_bytes())
                .map_err(|_| -> ServerError {
                    ServerError {
                        kind: CRITICAL.to_string(),
                        message: "Could not send to server".to_string(),
                    }
                })?;
        };

        // Else it is already connectede so it should continue with the normal handler flow
        Ok(())
    }

    ///
    /// This is used by the secondary server after connecting the socket.
    /// This is called when a secondary server connects to the primary server
    /// it will send the SERVER message and start listening for messages with the main server
    ///
    pub fn connect_to_main_server(&mut self) -> Result<(), ServerError> {
        let message = Message {
            prefix: None,
            command: SERVER.to_string(),
            params: vec![vec![self.server_name.clone()]],
        };

        //this is to test the connection between servers
        self.stream
            .write(message.as_string().as_bytes())
            .map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Could not write to stream".to_string(),
                }
            })?;
        println!("Sent message to main server");

        let mut server_handler = ClientHandler {
            stream: &self.stream,
            users: self.users.clone(),
            sender: self.sender_to_server.clone(),
            receiver: &self.receiver,
            channels: self.channels.clone(),
            client_name: self.server_name.clone(),
            user: None,
            reader: BufReader::new(self.stream.try_clone().map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Could not clone stream".to_string(),
                }
            })?),
        };

        server_handler.handle_client()?;

        Ok(())
    }

    ///
    /// Receives action(registration or login), password, nickname and user messages.
    ///
    /// If the action received is login, user is loged in if the user data is correct and a user
    /// is returned.
    ///
    /// If the action received is registration, user is registered if the user data is
    /// provided correctly and a user is returned.
    ///
    /// If the user could not registrate or log in the a numeric reply is sent to the client
    /// specifing the problem and None is returned
    ///
    /// If an error is found a Server error is returned.
    ///
    pub fn handle_login_registration(
        &self,
        reader: &mut BufReader<TcpStream>,
        action: Option<String>,
    ) -> Result<Option<User>, ServerError> {
        println!("Handling login and registration in connection handler");
        println!("action is {:?}", action);
        let mut action = action;
        let mut correct_registration = true;
        // Process message action if action was not given
        if action.is_none() {
            let action_message = self.process_data(reader)?;
            println!("Action message: {:?}", action_message);
            action = Some(get_action(&action_message).map_err(|_| -> ServerError {
                ServerError {
                    kind: NONCRITICAL.to_string(),
                    message: "Couldn't connect to main server".to_string(),
                }
            })?);
        }

        // Process message pass
        let password_message = self.process_data(reader)?;
        println!("Pass message: {:?}", password_message);

        let password = match get_password(&password_message) {
            Ok(pass) => pass,
            Err(reply) => {
                // If a numeric reply was given then invalid password
                self.send_reply(reply, &self.stream)?;
                correct_registration = false;
                "".to_string()
            }
        };

        // Process message nick
        let nick_message = self.process_data(reader)?;
        println!("Nick message: {:?}", nick_message);

        let nickname;
        match get_nickname(&nick_message, &action.clone().unwrap(), &self.users) {
            Ok(result) => {
                match result {
                    Ok(nick) => nickname = nick,
                    Err(reply) => {
                        // If a numeric reply was given then invalid nickname
                        self.send_reply(reply, &self.stream)?;
                        correct_registration = false;
                        nickname = "".to_string();
                    }
                };
            }
            Err(err) => {
                return Err(err);
            }
        };

        // Process message user
        let user_message = self.process_data(reader)?;
        println!("User message: {:?}", user_message);

        let user_info = match get_user_info(&user_message) {
            Ok(user_info) => user_info,
            Err(reply) => {
                // If a numeric reply was given then invalid user information
                self.send_reply(reply, &self.stream)?;
                return Ok(None);
            }
        };
        if !correct_registration {
            return Ok(None);
        }

        let user = User::new(
            nickname,
            user_info.1,
            user_info.0,
            user_info.3,
            user_info.2,
            password,
        );

        if action.unwrap() == LOGIN {
            println!("In login");
            match login_user(&self.users, user.clone(), &self.server_name) {
                Ok(result) => match result {
                    Ok(reply) => {
                        self.send_reply(reply, &self.stream)?;
                    }
                    Err(reply) => {
                        self.send_reply(reply, &self.stream)?;
                        return Ok(None);
                    }
                },
                Err(err) => return Err(err),
            }
        } else {
            println!("In registration");
            match registrate_user(&self.users, user.clone()) {
                Ok(reply) => {
                    self.send_reply(reply, &self.stream)?;
                }
                Err(err) => return Err(err),
            }
        }
        Ok(Some(user))
    }

    ///
    /// Read line from reader received, parses data and returns message.
    ///
    pub fn process_data(&self, reader: &mut BufReader<TcpStream>) -> Result<Message, ServerError> {
        let mut data = String::new();

        let received = match reader.read_line(&mut data) {
            Ok(_) => data.as_mut(),
            // Failed to read data received
            Err(_) => {
                return Err(ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Could not read from stream".to_string(),
                })
            }
        };

        println!("DATA RECEIVED {}", received);

        let message = match parse(received.to_string()) {
            Ok(message) => message,
            // Failed to parsed data received
            Err(_) => {
                return Err(ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Could not parse message".to_string(),
                })
            }
        };

        Ok(message)
    }

    ///
    /// This will send a numeric reply to a client via the stream.
    ///
    pub fn send_reply(
        &self,
        reply: NumericReply,
        mut stream: &TcpStream,
    ) -> Result<(), ServerError> {
        println!("Reply sent: {:?}", reply);
        stream
            .write_all(reply.as_string().as_bytes())
            .map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Could not write".to_string(),
                }
            })?;
        Ok(())
    }
}
