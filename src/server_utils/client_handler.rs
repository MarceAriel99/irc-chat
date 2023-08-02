//!
//! Client Handler receives messages from the client (server or user) and processes them
//!

use std::{
    collections::HashMap,
    io::Write,
    io::{BufRead, BufReader},
    net::TcpStream,
    sync::{mpsc::Receiver, mpsc::Sender, Arc, Mutex},
    time::Duration,
};

use crate::{
    commands::{
        AWAY, CHANNEL_INFO, INVITE, JOIN, KICK, LIST, MODE, NAMES, NICK, NOTICE, OPERATOR, PART,
        PRIVMSG, QUIT, REGISTRATION, SQUIT, TOPIC, USERS_INFO, WHO, WHOIS,
    },
    custom_errors::errors::{CRITICAL, SEND_MESSAGE},
    message::Message,
    numeric_reply::NumericReply,
    parser::parse,
    server_utils::{
        channel::Channel,
        messages_processing_client::{
            admin_server::handle_quit_server,
            connection_and_registration::{change_nick, quit, set_operator},
            manage_channels::{
                invite_to_channel, join_channel, kick, list_channels, names, part_channel,
                set_channel_mode, topic,
            },
            messages_exchange::{notice, private_message},
            user_information::{handle_away, handle_who, whois},
        },
        messages_processing_server::{
            connection_and_registration::{handle_registration_server, handle_users_info},
            manage_channels::{
                handle_away_server, handle_channel_info, handle_invite_multiserver,
                handle_join_server, handle_kick_multiserver, handle_mode_multiserver,
                handle_part_multiserver, handle_topic,
            },
            manage_server::handle_squit,
            message_exchange::handle_privmsg_server,
        },
        user::User,
    },
};

use crate::custom_errors::server_error::ServerError;

pub struct ClientHandler<'a> {
    pub stream: &'a TcpStream,
    pub sender: Sender<Message>,
    pub receiver: &'a Receiver<Message>,
    pub users: Arc<Mutex<HashMap<String, User>>>,
    pub channels: Arc<Mutex<HashMap<String, Channel>>>,
    pub client_name: String, // nickname from user or server name
    pub user: Option<User>,  // If client is a server then user = None
    pub reader: BufReader<TcpStream>,
}

impl ClientHandler<'_> {
    ///
    /// Reads messages from the client and carries out the request.
    ///
    pub fn handle_client(&mut self) -> Result<(), ServerError> {
        println!("handling client");

        // Wait until there's data to read
        self.stream.try_clone().map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Could not clone stream".to_string(),
            }
        })?;

        // Keep reading every message received
        self.read_and_handle_messages()?;

        Ok(())
    }

    /* FUNCTIONS TO KEEP ON READING MESSAGES (server socket or thread) */

    ///
    /// Reads data from the client and handles it.
    ///
    fn read_and_handle_messages(&mut self) -> Result<(), ServerError> {
        println!("begining to read and handle messages from client");

        let mut data = String::new();

        self.stream
            .set_read_timeout(Some(Duration::from_millis(100))) // this is needed so that it doesnt block
            .map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Could not set stream time out".to_string(),
                }
            })?;

        while match self.reader.read_line(&mut data) {
            Ok(_bytes_read) => {
                // If data was read then handle it
                if !data.is_empty() {
                    println!("Message from client read");
                    self.handle_data(&data)?;
                    data.clear();
                    true
                } else {
                    println!("No data read");
                    false
                }
            }
            Err(_error) => {
                true // Keep looping
            }
        } {
            // In every execution execute this block
            // Receive from the server channel to write to client
            self.read_from_server()?
        }
        Ok(())
    }

    ///
    /// Handles data read from the client. Parses message and carries out the request.
    ///
    fn handle_data(&mut self, data: &str) -> Result<(), ServerError> {
        println!("handling data read");

        // Parse messsage
        let mut message = match parse(data.to_owned()) {
            Ok(msg) => msg,
            Err(_) => {
                return Err(ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Error".to_string(),
                })
            }
        };

        println!("message read in client handler {:?}", message);

        // Add prefix of sender user to the message

        if message.prefix.is_none() && self.user.is_some() {
            message.prefix = Some(self.client_name.clone());
        }
        println!("message read in client handler with prefix {:?}", message);

        // Handle message
        match self.handle_message(message, self.sender.clone()) {
            Ok(_) => {}
            Err(err) => {
                // If a critical error was found return error
                if err.kind == *CRITICAL {
                    return Err(err);
                }
            }
        };

        Ok(())
    }

    ///
    /// Reads message from server and sends it to the client.
    ///
    fn read_from_server(&mut self) -> Result<(), ServerError> {
        let message = match self.receiver.try_recv() {
            Ok(msg) => msg,
            Err(_) => return Ok(()),
        };

        println!("MESSAGE SENT, read from server: {}", message.as_string());

        self.stream
            .write_all(message.as_string().as_bytes())
            .map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Could not send to server".to_string(),
                }
            })?;
        Ok(())
    }

    /* FUNCTIONS TO HANDLE MESSAGES */

    ///
    /// Calls corresponding function to carry out the request in Message. Could return a Numeric Reply
    ///
    fn handle_message(
        &mut self,
        message: Message,
        sender: Sender<Message>,
    ) -> Result<(), ServerError> {
        //println!("Handling message");

        if self.user.is_some() {
            println!("handling message from user ");
            self.handle_user_message(message, sender)?;
        } else {
            println!("handling message from server");
            self.handle_server_message(message, sender)?;
        }

        Ok(())
    }

    ///
    /// Calls corresponding function to carry out the request in Message. Could return a Numeric Reply
    ///
    fn handle_user_message(
        &mut self,
        message: Message,
        sender: Sender<Message>,
    ) -> Result<(), ServerError> {
        let user = self.user.as_mut().expect("Couldn't get user");

        println!("user that send the message: {:?}", user);
        println!("message received: {:?}", message);

        let command = &message.command;
        let result = match command.as_str() {
            NICK => change_nick(message, &self.users, user),
            PRIVMSG => private_message(
                message,
                self.users.clone(),
                &sender,
                self.channels.clone(),
                Some(self.stream),
            ),
            NOTICE => notice(message, self.users.clone(), &sender),
            JOIN => join_channel(
                self.stream,
                message,
                &self.channels.clone(),
                &self.users.clone(),
                user,
                &sender,
            ),
            NAMES => names(message, self.stream, self.channels.clone()),
            LIST => list_channels(message, &self.channels.clone(), self.stream),
            PART => part_channel(
                message,
                &self.channels.clone(),
                user,
                self.stream,
                &self.sender,
            ),
            INVITE => invite_to_channel(
                &sender,
                message,
                self.channels.clone(),
                self.users.clone(),
                user,
            ),
            MODE => set_channel_mode(
                message,
                &self.channels.clone(),
                user,
                &self.sender,
                self.stream,
            ),
            OPERATOR => set_operator(message, &sender, self.receiver),
            WHO => handle_who(
                message,
                self.stream,
                self.users.clone(),
                self.receiver,
                &sender,
            ),
            WHOIS => whois(
                message,
                self.stream,
                self.users.clone(),
                &sender,
                self.receiver,
                self.channels.clone(),
            ),
            QUIT => quit(message, self.stream, &sender, user),
            AWAY => handle_away(message, user, self.users.clone(), Some(&sender)),
            SQUIT => handle_quit_server(message, &sender, self.receiver),
            KICK => kick(message, user, self.channels.clone(), &sender),
            TOPIC => topic(message, self.channels.clone(), user, &sender),
            _ => return Ok(()),
        };

        let result = result?;

        println!("REPLY {:?}", result);
        if let Some(reply) = result {
            self.send_reply(reply, self.stream)?;
        }

        Ok(())
    }

    ///
    /// Calls corresponding function to carry out the request in Message. Could return a Numeric Reply
    ///
    fn handle_server_message(
        &mut self,
        message: Message,
        sender: Sender<Message>,
    ) -> Result<(), ServerError> {
        println!("Handling message {:?}", message);

        let command = &message.command;

        match command.as_str() {
            JOIN => handle_join_server(message, &sender),
            REGISTRATION => handle_registration_server(message, &self.sender),
            SQUIT => handle_squit(message, &sender, self.receiver, self.stream),
            PRIVMSG => handle_privmsg_server(message, &sender),
            USERS_INFO => handle_users_info(message, self.users.clone()),
            CHANNEL_INFO => handle_channel_info(message, self.channels.clone(), self.users.clone()),
            KICK => {
                handle_kick_multiserver(message, self.stream, self.channels.clone(), &self.sender)
            }
            MODE => handle_mode_multiserver(
                message,
                self.channels.clone(),
                self.users.clone(),
                &self.sender,
                self.client_name.clone(),
            ),
            PART => handle_part_multiserver(
                message,
                self.channels.clone(),
                self.users.clone(),
                &self.sender,
            ),
            TOPIC => handle_topic(message, self.channels.clone(), &self.sender),
            INVITE => handle_invite_multiserver(
                message,
                self.channels.clone(),
                self.users.clone(),
                &self.sender,
            ),
            AWAY => handle_away_server(message, &self.sender),
            _ => return Ok(()),
        }?;

        Ok(())
    }

    ///
    /// This function sends a nnumeric reply to the client who is connected via the stream
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
                    message: SEND_MESSAGE.to_string(),
                }
            })?;
        Ok(())
    }
}
