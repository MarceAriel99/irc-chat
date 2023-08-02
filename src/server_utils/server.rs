//!
//! Struct server, this represents the main server. It will launch a new thread for each new user that connects
//! to communicate between the threads, it uses the channels. This does not communicate with the channels, that is resposibility
//! of each thread in the client handler
//!

use std::{
    collections::HashMap,
    result::Result,
    string::String,
    sync::{
        mpsc::{self, Receiver, Sender},
        MutexGuard, {Arc, Mutex},
    },
    thread::{self, JoinHandle},
};

use super::user::User;
use super::{
    main_server::MainServer, secondary_server::SecondaryServer, server_data::ServerData,
    server_rol::ServerRol,
};
use crate::message::Message;
use crate::{
    commands::AWAY,
    custom_errors::server_error::ServerError,
    server_utils::{connection_listener::ConnectionListener, server_data::add_user},
};
use crate::{
    commands::{
        INVITE, IS_OPERATOR, JOIN, KICK, MODE, NOTICE, OPERATOR, PART, PRIVMSG, QUIT, REGISTRATION,
        SERVER, SERVER_EXISTS, SQUIT, TOPIC, USERS_INFO, WHO, WHOIS,
    },
    custom_errors::errors::{CRITICAL, NONCRITICAL},
    server_utils::channel::Channel,
};

pub struct Server {
    // initial data from server
    server_data: ServerData,
    // sender for the client handler to communicate with server
    sender_to_server: Sender<Message>,
    receiver_from_handler: Receiver<Message>,
    // server operator
    operator: String,
    //main server or secondary server
    server_rol: Box<dyn ServerRol>,
    // client info, nickname: (thread joinHandle, Sender to client)
    users_clients:
        Arc<Mutex<HashMap<String, (Option<JoinHandle<Result<(), ServerError>>>, Sender<Message>)>>>,
    // server clients
    server_clients:
        Arc<Mutex<HashMap<String, (Option<JoinHandle<Result<(), ServerError>>>, Sender<Message>)>>>,
    // server channels
    channels: Arc<Mutex<HashMap<String, Channel>>>,
    // server users
    users: Arc<Mutex<HashMap<String, User>>>,
}

impl Server {
    ///
    /// Creates a new Server with a received configuration. Initializes the channels and clients connected.
    ///
    pub fn new(server_data: ServerData) -> Result<Self, ServerError> {
        // Communication channel from client handler thread to server thread
        let (sender_to_server, receiver_from_handler): (Sender<Message>, Receiver<Message>) =
            mpsc::channel();
        let users = Arc::new(Mutex::new(server_data.users.clone()));
        let channels = Arc::new(Mutex::new(HashMap::new()));
        let users_clients = Arc::new(Mutex::new(HashMap::new()));
        let server_clients = Arc::new(Mutex::new(HashMap::new()));

        let server_rol: Box<dyn ServerRol> = if server_data.is_main() {
            Box::new(MainServer::new())
        } else {
            println!("{:?}", server_data);
            let secondary_server = SecondaryServer::new(
                users.clone(),
                sender_to_server.clone(),
                channels.clone(),
                server_data.server_name.clone(),
                server_data.main.clone().unwrap(),
            )?;
            Box::new(secondary_server)
        };

        Ok(Server {
            channels,
            users,
            server_data,
            users_clients,
            server_clients,
            sender_to_server,
            receiver_from_handler,
            operator: "".to_string(),
            server_rol,
        })
    }

    ///
    /// Binds server to address specified in ConfigData and awaits and handles new connections.
    /// For each new connection it launches a new thread and saves the user and the channel sender in a vector
    ///
    pub fn run(mut self) -> Result<(), ServerError> {
        println!("Server running");

        let mut connection_listener = ConnectionListener {
            address: self.server_data.server_address.clone(),
            users: self.users.clone(),
            channels: self.channels.clone(),
            user_clients: self.users_clients.clone(),
            server_clients: self.server_clients.clone(),
            server_name: self.server_data.server_name.clone(),
            sender_to_server: self.sender_to_server.clone(),
        };

        let _ = thread::spawn(move || match connection_listener.read_new_connections() {
            Ok(_) => {
                println!("No new connections");
                Ok(())
            }
            Err(err) => Err(err),
        });

        loop {
            if let Err(err) = self.check_messages() {
                if err.kind == CRITICAL {
                    return Err(err);
                }
            }
        }
    }

    /// This function checks if there are messages from the clients and if there are
    /// it will check what to do with them. Right now it is a send so it sends it to
    /// the client
    fn check_messages(&mut self) -> Result<(), ServerError> {
        let message = self.receiver_from_handler.recv().map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Could not receive from handler".to_string(),
            }
        })?;

        println!("Received message in SERVER: {:?}", message);

        match message.command.as_str() {
            REGISTRATION => self.handle_registration(message),
            SERVER => self.handle_server(message),
            PRIVMSG => self.handle_private_message(message),
            NOTICE => self.handle_notice(message),
            JOIN => self.handle_join(message),
            INVITE => self.handle_invite(message),
            OPERATOR => self.handle_operator(message),
            WHO => self.handle_who(message),
            QUIT => self.handle_quit(message),
            WHOIS => self.handle_whois(message),
            SQUIT => self.handle_squit(message),
            KICK => self.handle_kick(message),
            USERS_INFO => self.handle_users_info(message),
            SERVER_EXISTS => self.handle_server_exists(message),
            IS_OPERATOR => self.handle_is_operator(message),
            MODE => self.handle_mode(message),
            PART => self.handle_part(message),
            TOPIC => self.handle_topic(message),
            AWAY => self.handle_away(message),
            &_ => {
                return Err(ServerError {
                    kind: "Message".to_string(),
                    message: "command not existing".to_string(),
                })
            }
        }?;
        Ok(())
    }

    /* FUNCTIONS TO HANDLE MESSAGES */

    ///
    /// Handles away multiserver. If user specified in message is not away is this
    /// server then it is set as away. If user is away and no message was specified
    /// then it is set as no longer away. Notifies all corresponding server so user
    /// away status is updated
    ///
    fn handle_away(&mut self, message: Message) -> Result<(), ServerError> {
        println!("handling away en server");
        let nickname = &message.prefix.clone().unwrap();
        let mut users = self.users.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Could not access channels".to_string(),
            }
        })?;

        let user = match users.get_mut(nickname) {
            Some(user) => user,
            None => {
                return Err(ServerError {
                    kind: CRITICAL.to_string(),
                    message: "user not found".to_string(),
                })
            }
        };

        if message.params_total_count() == 0 {
            if user.is_away() {
                println!("Setting user as no longer away");
                user.no_longer_away();
            } else {
                println!("Notifying all server");
                self.server_rol.notify(message)?;
                return Ok(());
            }
        } else {
            let away_message = &message.params[0][0];
            if !user.has_away_message(away_message) {
                println!("Setting away message");
                user.away_message = Some(away_message.to_string());
            } else {
                println!("Notifying all server");
                self.server_rol.notify(message)?;
                return Ok(());
            }
        }

        if self.server_data.is_main() {
            println!("Notifying server");
            self.server_rol.notify_all_but(message, &user.server_name)?;
        }

        Ok(())
    }

    ///
    /// This function handles a topic message, it will set the topic of the channel if it is possible
    ///
    fn handle_topic(&mut self, message: Message) -> Result<(), ServerError> {
        let nickname = &message.prefix.clone().unwrap();
        let users = self.users.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Could not access channels".to_string(),
            }
        })?;
        let user_setting_topic = match users.get(nickname) {
            Some(user) => user,
            None => {
                return Err(ServerError {
                    kind: CRITICAL.to_string(),
                    message: "user not found".to_string(),
                })
            }
        };

        let channel_name = &message.params[0][0];
        let mut channels = self.channels.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Could not access channels".to_string(),
            }
        })?;
        let channel = match channels.get_mut(channel_name) {
            Some(channel) => channel,
            None => {
                return Err(ServerError {
                    kind: NONCRITICAL.to_string(),
                    message: "channel not found".to_string(),
                })
            }
        };

        let topic = &message.params[1][0];

        if !channel.channel_has_topic(topic) {
            let _result = channel.set_topic(&message.prefix.clone().unwrap(), topic);
            if self.server_data.is_main() {
                self.server_rol
                    .notify_all_but(message, &user_setting_topic.server_name)?;
            }
        } else {
            self.server_rol.notify(message)?;
        }

        Ok(())
    }

    ///
    /// Handles the SQUIT message, it is used either when a user asks to disconnect a server
    /// but if the user isnt an operator it will send an error message to the user
    ///
    fn handle_squit(&mut self, message: Message) -> Result<(), ServerError> {
        println!("Received SQUIT message");
        let user = match message.clone().prefix {
            Some(user) => user,
            None => {
                return Err(ServerError {
                    kind: CRITICAL.to_string(),
                    message: message.params[1][0].clone(),
                })
            }
        };

        let server_name = &message.params[0][0].clone();
        if &message.params[0][0] == "You are not an operator" {
            self.send_message_to_receiver(&message, &message.prefix.clone().unwrap())?;
        } else if *server_name == self.server_data.server_name {
            let mut comment = &"".to_string();
            if message.params_total_count() == 2 {
                comment = &message.params[1][0];
            }
            println!("message: {:?}", message);
            if self.operator == user {
                self.send_message_to_receiver(&message, &message.prefix.clone().unwrap())?; //to notify the thread that the squit was received succesfully
                let quit_message = Message {
                    prefix: None,
                    command: QUIT.to_string(),
                    params: vec![vec![comment.clone()]],
                };
                println!(
                    "Sending quit message to all users: {:?}",
                    self.users_clients
                );
                let users_clients = self.users_clients.lock().map_err(|_| -> ServerError {
                    ServerError {
                        kind: NONCRITICAL.to_string(),
                        message: "Could not access user clients".to_string(),
                    }
                })?;
                for client in users_clients.values() {
                    println!("sending quit message to client");
                    println!("quit message: {:?}", quit_message);
                    client
                        .1
                        .send(quit_message.clone())
                        .map_err(|_| -> ServerError {
                            ServerError {
                                kind: CRITICAL.to_string(),
                                message: "Couldn't set as non blocking".to_string(),
                            }
                        })?;
                    println!("message sent");
                }
                println!("sending quit message to all servers");
                let mut message_notice = message.clone();
                message_notice.prefix = None;
                self.server_rol.notify(message_notice)?;
                // Waiting for all threads to finish before exiting
                for client in self.users_clients.lock().unwrap().iter_mut() {
                    println!("Waiting for client to disconnect");
                    let data = client.1;
                    if let Some(handler) = data.0.take() {
                        handler.join().map_err(|_| -> ServerError {
                            ServerError {
                                kind: CRITICAL.to_string(),
                                message: "Couldn't set as non blocking".to_string(),
                            }
                        })??;
                    }
                }
                // Returns this error to inform that it must stop running, and sends the comment so that it can be shown
                return Err(ServerError {
                    kind: "SQUIT".to_string(),
                    message: comment.to_string(),
                });
            } else {
                println!("User {} is not an operator", user);
                let mut answer_message = message.clone();
                answer_message.params = vec![vec!["You are not an operator".to_string()]];
                self.send_message_to_receiver(&answer_message, &user)?
            }
        } else {
            let _res = match self
                .server_rol
                .send_message_to_server(message.clone(), server_name.to_string())
            {
                Ok(_) => true,
                Err(error) => {
                    if error.kind == CRITICAL {
                        return Err(error);
                    }
                    let mut answer_message = message;
                    answer_message.params = vec![vec!["Server not found".to_string()]];
                    self.send_message_to_receiver(&answer_message, &user)?;
                    false
                }
            };
        }
        Ok(())
    }

    ///
    /// Handles the server message. It is used when a new connection arrives or to notify a server of a new one
    ///
    fn handle_server(&mut self, message: Message) -> Result<(), ServerError> {
        let mut binding = self.server_clients.lock().unwrap();
        let server_client = binding.remove(&message.params[0][0]);

        if let Some((handle, sender)) = server_client {
            self.server_rol.as_mut().handle_server(
                message,
                handle.unwrap(),
                sender,
                self.server_data.server_name.clone(),
                self.users.clone(),
                self.channels.clone(),
            )?;
        }

        Ok(())
    }

    ///
    /// Receives the users infor and saves it in the users hashmap
    ///
    fn handle_users_info(&mut self, message: Message) -> Result<(), ServerError> {
        let users: MutexGuard<HashMap<String, User>> =
            self.users.lock().map_err(|_| -> ServerError {
                ServerError {
                    kind: NONCRITICAL.to_string(),
                    message: "Couldn't get lock".to_string(),
                }
            })?;
        let nickname = message.prefix.clone().unwrap();
        if !users.contains_key(&nickname) {
            println!("new user");
            self.add_new_user(message, users)?;
        }
        Ok(())
    }

    ///
    /// This function handles the registration of a new user in the server.
    /// It checks if the user is already registered in this server
    ///
    fn handle_registration(&mut self, message: Message) -> Result<(), ServerError> {
        println!("handling registration in server");

        let users: MutexGuard<HashMap<String, User>> =
            self.users.lock().map_err(|_| -> ServerError {
                ServerError {
                    kind: NONCRITICAL.to_string(),
                    message: "Couldn't get lock".to_string(),
                }
            })?;

        let nickname = message.prefix.clone().unwrap();
        let user;

        if !users.contains_key(&nickname) {
            println!("Adding new user");
            user = self.add_new_user(message.clone(), users)?;

            if self.server_data.is_main() {
                println!("Notifying servers of message: {:?}", message);
                self.server_rol.notify_all_but(message, &user.server_name)?;
            };
        } else {
            user = users.get(&nickname).unwrap().clone(); //this wont fail

            if user.server_name == self.server_data.server_name {
                println!("Notifying servers of message: {:?}", message);
                self.server_rol.notify_all_but(message, &user.server_name)?;
            }
        };

        if self.server_data.is_main() {
            add_user(&user, self.server_data.users_file_path.clone()).map_err(
                |_| -> ServerError {
                    ServerError {
                        kind: CRITICAL.to_string(),
                        message: "Could not save user in data file".to_string(),
                    }
                },
            )?;
        }

        Ok(())
    }

    ///
    /// This function handles the registration of a new user in the server. It will save it in the users dictionaty
    ///
    fn add_new_user(
        &self,
        message: Message,
        mut users: MutexGuard<HashMap<String, User>>,
    ) -> Result<User, ServerError> {
        let user_data = message.params;
        println!("user_data: {:?}", user_data);
        let user = User::new(
            user_data[0][0].clone(),
            user_data[0][1].clone(),
            user_data[0][2].clone(),
            user_data[1][0].clone(),
            user_data[0][3].clone(),
            user_data[0][4].clone(),
        );

        users.insert(user.nickname.clone(), user.clone());
        println!("new user saved {:?}", user);
        Ok(user)
    }

    ///
    /// This function will take the received invite messsage and ask to resend it to the correct user.
    ///
    fn handle_invite(&mut self, message: Message) -> Result<(), ServerError> {
        let receiver = &message.params.clone()[0][0];

        let users = self.users.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Could not access channels".to_string(),
            }
        })?;
        let user_inviting = match users.get(receiver) {
            Some(user) => user,
            None => {
                return Err(ServerError {
                    kind: CRITICAL.to_string(),
                    message: "user not found".to_string(),
                })
            }
        };

        if user_inviting.server_name == self.server_data.server_name {
            let users_clients = self.users_clients.lock().map_err(|_| -> ServerError {
                ServerError {
                    kind: NONCRITICAL.to_string(),
                    message: "Could not lock user clients".to_string(),
                }
            })?;
            let sender = match users_clients.get(receiver) {
                Some((_, sender)) => sender,
                None => {
                    return Err(ServerError {
                        kind: NONCRITICAL.to_string(),
                        message: "client not found".to_string(),
                    })
                }
            };
            sender.send(message).map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Broken channel connection".to_string(),
                }
            })?;
        } else {
            self.server_rol
                .send_message_to_server(message, user_inviting.server_name.clone())?;
        }

        Ok(())
    }

    ///
    /// Checks ig a server exists, and will send a message to the asking thread with the answer
    ///
    fn handle_server_exists(&mut self, message: Message) -> Result<(), ServerError> {
        let server_name = message.params[0][0].clone();
        if server_name == self.server_data.server_name {
            return Ok(());
        }
        match self.server_rol.check_server_existance(message.clone()) {
            Ok(_) => Ok(()),
            Err(error) => {
                if error.kind != CRITICAL {
                    let mut answer_message = message.clone();
                    answer_message.params = vec![vec!["Server not found".to_string()]];
                    self.send_message_to_receiver(&answer_message, &message.prefix.unwrap())?;
                }
                Err(error)
            }
        }
    }

    ///
    /// This function is called when a thread asks if an user is an operatori of the server, it will send
    /// the thread a message informing if the user is an operator or not<w
    ///
    fn handle_is_operator(&mut self, message: Message) -> Result<(), ServerError> {
        let receiver = message.prefix.clone().unwrap();
        if self.operator == *receiver {
            let mut answer = message;
            answer.params = vec![vec!["You are an operator".to_string()]];
            self.send_message_to_receiver(&answer, &receiver)?;
        } else {
            let mut answer_message = message;
            answer_message.params = vec![vec![]];
            self.send_message_to_receiver(&answer_message, &receiver)?;
        }
        Ok(())
    }

    ///
    /// This is called when a whois command is received. It will send the
    /// information of the user to the receiver
    ///
    fn handle_whois(&mut self, message: Message) -> Result<(), ServerError> {
        if message.params_total_count() == 2 {
            let server_name = message.params[0][0].clone();
            if self.server_data.server_name != server_name {
                let _aux = match self.server_rol.check_server_existance(message.clone()) {
                    Ok(_) => true,
                    Err(error) => {
                        if error.kind != CRITICAL {
                            let mut answer_message = message.clone();
                            answer_message.params = vec![vec!["Server not found".to_string()]];
                            self.send_message_to_receiver(
                                &answer_message,
                                &message.prefix.unwrap(),
                            )?;
                        }
                        return Err(error);
                    }
                };
            } else {
                self.send_message_to_receiver(&message, &message.prefix.clone().unwrap())?;
            }
        } else {
            self.send_message_to_receiver(&message, &message.prefix.clone().unwrap())?;
        }
        Ok(())
    }

    ///
    /// This function receives the operator message
    /// if the password received is correct it will set the operator
    ///
    fn handle_operator(&mut self, message: Message) -> Result<(), ServerError> {
        let operator = message.params[0][0].clone();
        let password = message.params[1][0].clone();
        println!("operator: {}", operator);
        println!("password: {}", password);
        let users = self.users.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Couldn't get lock".to_string(),
            }
        })?;
        for user in users.values() {
            if user.nickname == operator {
                if user.password == password {
                    self.operator = operator.clone();
                    let mut answer = message;
                    answer.params = vec![vec!["You are now an operator".to_string()]];
                    self.send_message_to_receiver(&answer, &operator)?;
                    println!("new operator: {}", self.operator);
                    return Ok(());
                } else {
                    let mut answer = message.clone();
                    answer.params = vec![vec!["Wrong password".to_string()]];
                    self.send_message_to_receiver(&answer, &operator)?;
                }
            }
        }
        Ok(())
    }

    ///
    /// This function is called when a mode message is received, it will notify all servers of the new mode
    ///
    fn handle_mode(&mut self, message: Message) -> Result<(), ServerError> {
        let nickname_setting_mode = message.prefix.clone().unwrap();

        let users = self.users.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Could not access channels".to_string(),
            }
        })?;
        let user_setting_mode = match users.get(&nickname_setting_mode) {
            Some(user) => user,
            None => {
                return Err(ServerError {
                    kind: NONCRITICAL.to_string(),
                    message: "user not found".to_string(),
                })
            }
        };

        if user_setting_mode.server_name == self.server_data.server_name
            || self.server_data.is_main()
        {
            println!("{} is notifying servers", self.server_data.server_name);
            self.server_rol.notify(message)?;
        }

        Ok(())
    }

    ///
    /// Handles part message, and notifys all servers of the part
    ///
    fn handle_part(&mut self, message: Message) -> Result<(), ServerError> {
        let nickname_parting = message.prefix.clone().unwrap();

        let mut users = self.users.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Could not access channels".to_string(),
            }
        })?;
        let user_parting = match users.get_mut(&nickname_parting) {
            Some(user) => user,
            None => {
                return Err(ServerError {
                    kind: NONCRITICAL.to_string(),
                    message: "user not found".to_string(),
                })
            }
        };

        let channel_name = &message.params[0][0];
        let mut channels = self.channels.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Could not access channels".to_string(),
            }
        })?;
        let channel = match channels.get_mut(channel_name) {
            Some(channel) => channel,
            None => {
                return Err(ServerError {
                    kind: CRITICAL.to_string(),
                    message: "channel not found".to_string(),
                })
            }
        };

        if channel.is_user_on_channel(&nickname_parting) {
            channel.part(user_parting.clone());
            user_parting.remove_channel(channel_name);

            if self.server_data.is_main() {
                self.server_rol
                    .notify_all_but(message, &user_parting.server_name)?;
            }
        } else {
            self.server_rol.notify(message)?;
        }

        Ok(())
    }

    ///
    /// This function is called when the thread receives a who message
    /// requesting for operators data, it will either return all operators,
    /// return all the operators that match the name received
    /// or inform that there are no operators to send.
    ///
    fn handle_who(&self, message: Message) -> Result<(), ServerError> {
        let receiver = match message.prefix.clone() {
            Some(prefix) => prefix,
            None => "".to_string(),
        };
        let mut reply_message = message.clone();
        let mut has_sent = false;
        if message.params_total_count() == 1 || message.params[0][0] == "0" {
            println!("sending all operators");
            reply_message.params = vec![vec![self.operator.clone()]];
            self.send_message_to_receiver(&reply_message, &receiver)?;
            has_sent = true;
        } else {
            let users = self.users.lock().map_err(|_| -> ServerError {
                ServerError {
                    kind: NONCRITICAL.to_string(),
                    message: "Couldn't get lock 4".to_string(),
                }
            })?;
            let user = match users.get(&self.operator.clone()) {
                Some(user) => user,
                None => {
                    return Err(ServerError {
                        kind: NONCRITICAL.to_string(),
                        message: "Couldn't get lock".to_string(),
                    })
                }
            };
            if user.has_atribute_name(&message.params[0][0].clone()) {
                println!("sending operator");
                reply_message.params = vec![vec![self.operator.clone()]];
                self.send_message_to_receiver(&message, &receiver)?;
                has_sent = true;
            }
        }
        if !has_sent {
            reply_message.command = "OPERATOR_NOT_FOUND".to_string();
            return self.send_message_to_receiver(&message, &receiver);
        }
        Ok(())
    }

    ///
    /// This function will receive a message to be sent and will send it to the
    /// thread so that it is sent to the correct client
    ///
    fn handle_private_message(&mut self, message: Message) -> Result<(), ServerError> {
        let nick = match message.prefix.clone() {
            Some(nick) => nick,
            None => "".to_string(),
        };
        let receiver = &message.params[0][0];
        println!("Sending private message Receiver: {:?}", receiver);

        if receiver.contains('#') || receiver.contains('&') {
            // Send message to channel
            self.send_message_to_channel(receiver, &nick, &message)?;
        } else {
            // Send message to receiver
            self.send_message_to_receiver(&message, receiver)?;
        }

        Ok(())
    }

    ///
    /// This function will receive a message with NOTICE command and will
    /// notify the user specified
    ///
    fn handle_notice(&mut self, message: Message) -> Result<(), ServerError> {
        let receiver = &message.params[0][0];

        self.send_message_to_receiver(&message, receiver)
    }

    ///
    /// Receives a message with QUIT command. If a text message was provided in message
    /// it is sent to the channels that the user is part of. The thread were the client handler
    /// of the client is running is joined and the client information is removed from server.
    ///
    fn handle_quit(&mut self, message: Message) -> Result<(), ServerError> {
        println!("Handling QUIT in server {}", self.server_data.server_name);

        let nickname = match message.prefix.clone() {
            Some(nickname) => nickname,
            None => "".to_string(),
        };

        if message.params_total_count() == 1 {
            let users = self.users.lock().map_err(|_| -> ServerError {
                ServerError {
                    kind: NONCRITICAL.to_string(),
                    message: "Couldn't get lock".to_string(),
                }
            })?;
            let user = users.get(&nickname).unwrap(); // If it is here then the user exists so it cant fail
            let channels = &user.channels;

            for channel in channels {
                println!("Sending message to channel {}", channel);
                self.send_message_to_channel(channel, &nickname, &message)?;
            }
        }

        if let Some((_, (handler, _))) = self.users_clients.lock().unwrap().remove_entry(&nickname)
        {
            handler.unwrap().join().map_err(|_| -> ServerError {
                ServerError {
                    kind: NONCRITICAL.to_string(),
                    message: "Couldn't get lock".to_string(),
                }
            })??;
        }

        Ok(())
    }

    ///
    /// Handles multiserver kick. If server has already kicked the user from channel
    /// then it notifies other server so they kick it and user getting kicked.
    ///
    fn handle_kick(&mut self, message: Message) -> Result<(), ServerError> {
        let channel_name = &message.params[0][0];
        let nickname_user_getting_kicked = &message.params[1][0];

        let mut channels = self.channels.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Could not access channels".to_string(),
            }
        })?;

        let channel = match channels.get_mut(channel_name) {
            Some(channel) => channel,
            None => {
                return Err(ServerError {
                    kind: CRITICAL.to_string(),
                    message: "channel not found".to_string(),
                })
            }
        };

        let users = self.users.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Could not access channels".to_string(),
            }
        })?;

        let user_kicking = match users.get(&message.prefix.clone().unwrap()) {
            Some(user) => user,
            None => {
                return Err(ServerError {
                    kind: CRITICAL.to_string(),
                    message: "user not found".to_string(),
                })
            }
        };

        if !channel.is_user_on_channel(nickname_user_getting_kicked) {
            if user_kicking.server_name == self.server_data.server_name {
                self.server_rol.notify(message.clone())?;
            }

            let mut users_clients = self.users_clients.lock().map_err(|_| -> ServerError {
                ServerError {
                    kind: NONCRITICAL.to_string(),
                    message: "Could not lock user clients".to_string(),
                }
            })?;
            let sender = users_clients.get_mut(nickname_user_getting_kicked);

            if let Some((_, sender_user)) = sender {
                sender_user.send(message).map_err(|_| -> ServerError {
                    ServerError {
                        kind: CRITICAL.to_string(),
                        message: "Broken channel connection".to_string(),
                    }
                })?;
            };
        }

        Ok(())
    }

    ///
    /// Receives a message with join command. If the server does not have the channel
    /// then gets created. In other case all the correspoding servers get the message
    /// so they can check if they have the channel and all users.
    ///
    fn handle_join(&mut self, message: Message) -> Result<(), ServerError> {
        println!("JOIN handling join in server");

        let channel_name = &message.params[0][0];
        let mut channels = self.channels.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Couldn't get lock".to_string(),
            }
        })?;

        let nickname_user_joining = &message.prefix.clone().unwrap();
        let users = self.users.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Could not access channels".to_string(),
            }
        })?;
        let user_joining = match users.get(nickname_user_joining) {
            Some(user) => user,
            None => {
                return Err(ServerError {
                    kind: CRITICAL.to_string(),
                    message: "user not found".to_string(),
                })
            }
        };

        if !channels.contains_key(channel_name) {
            println!("Channel not found, creating channel");
            // If server does not have channel then create it
            let operator_nick = &message.prefix.clone().unwrap(); // This unwrap is safe because the server will always send the message with a prefix
            let operator = users.get(operator_nick).unwrap();
            let channel = Channel::new(channel_name.clone(), operator);

            channels.insert(channel_name.to_string(), channel);

            if self.server_data.is_main() {
                self.server_rol
                    .notify_all_but(message, &user_joining.server_name)?;
            }
        } else {
            println!("Channel found");
            let channel = match channels.get_mut(channel_name) {
                Some(channel) => channel,
                None => {
                    return Err(ServerError {
                        kind: CRITICAL.to_string(),
                        message: "channel not found".to_string(),
                    })
                }
            };

            if !channel.is_user_on_channel(nickname_user_joining) {
                println!("Adding user to channel");
                channel
                    .users
                    .insert(nickname_user_joining.to_string(), user_joining.clone());
                if self.server_data.is_main() {
                    self.server_rol
                        .notify_all_but(message, &user_joining.server_name)?;
                }
            } else {
                // Notify other channels that a channel was created or a user joined
                self.server_rol
                    .notify_all_but(message, &user_joining.server_name)?;
            }
        }

        Ok(())
    }

    /* FUNCTIONS TO COMMUNICATE WITH CLIENT */

    ///
    /// This function will Send a Message to the receiver
    ///
    fn send_message_to_receiver(
        &self,
        message: &Message,
        receiver: &String,
    ) -> Result<(), ServerError> {
        println!("Sent message: {:?} to {}", message, receiver);
        let users_clients = self.users_clients.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Could not lock user clients".to_string(),
            }
        })?;
        let sender = match users_clients.get(receiver) {
            Some((_, sender)) => sender,
            None => {
                let users = self.users.lock().map_err(|_| -> ServerError {
                    ServerError {
                        kind: NONCRITICAL.to_string(),
                        message: "Couldn't get lock".to_string(),
                    }
                })?;
                let user = users.get(receiver).unwrap();
                let server_name = user.server_name.clone();
                self.server_rol
                    .send_message_to_server(message.clone(), server_name)?;

                return Ok(());
            }
        };

        sender.send(message.clone()).map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Couldn't send".to_string(),
            }
        })?;

        Ok(())
    }

    ///
    /// This function will Send a Message to every member of the channel
    ///
    fn send_message_to_channel(
        &self,
        channel_name: &String,
        nickname_sender: &String,
        message: &Message,
    ) -> Result<(), ServerError> {
        println!("Send message to channel: {}", channel_name);
        let channels = self.channels.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Couldn't get lock".to_string(),
            }
        })?;
        match channels.get(channel_name) {
            Some(channel) => {
                for nickname in channel.users.keys() {
                    let is_in_server = self.users_clients.lock().unwrap().contains_key(nickname);
                    if self.send_priv_msg(nickname_sender, nickname, is_in_server)?
                        && nickname != nickname_sender
                    {
                        self.send_message_to_receiver(message, nickname)?;
                    }
                }
            }
            None => {
                return Err(ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Couldn't get channel".to_string(),
                });
            }
        }
        Ok(())
    }

    fn send_priv_msg(
        &self,
        nickname_sender: &str,
        nickname_receiver: &str,
        is_in_server: bool,
    ) -> Result<bool, ServerError> {
        if is_in_server {
            println!("Sending message client in main");
            return Ok(true);
        }
        let users = self.users.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Couldn't lock users".to_string(),
            }
        })?;
        let user = match users.get(nickname_sender) {
            Some(user) => user,
            None => {
                return Err(ServerError {
                    kind: NONCRITICAL.to_string(),
                    message: "User doesn't exist".to_string(),
                })
            }
        };
        let user_receiver = match users.get(nickname_receiver) {
            Some(user) => user,
            None => {
                return Err(ServerError {
                    kind: NONCRITICAL.to_string(),
                    message: "User doesn't exist".to_string(),
                })
            }
        };

        let sender_server = &user.server_name;
        let receiver_server = &user_receiver.server_name;
        if sender_server == receiver_server {
            return Ok(false);
        }
        Ok(true)
    }
}
