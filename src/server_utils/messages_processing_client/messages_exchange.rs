//! MESSAGES EXCHANGE FUNCTIONS
//!
//! This module contains functions to handle the exchange of messages
//!

use std::{
    collections::HashMap,
    io::Write,
    net::TcpStream,
    string::String,
    sync::{
        mpsc::Sender,
        {Arc, Mutex},
    },
};

use crate::{
    custom_errors::{
        errors::{CRITICAL, NONCRITICAL},
        server_error::ServerError,
    },
    numeric_reply::{
        NumericReply, ERR_NORECIPIENT_MSG, ERR_NORECIPIENT_NUM, ERR_NOSUCHNICK_MSG,
        ERR_NOSUCHNICK_NUM, ERR_NOTEXTTOSEND_MSG, ERR_NOTEXTTOSEND_NUM, ERR_NOTONCHANNEL_MSG,
        ERR_NOTONCHANNEL_NUM, RPL_AWAY_NUM,
    },
    server_utils::{channel::Channel, user::User},
};

use crate::message::Message;

/*******************************PRIVATE MESSAGE***********************************/

///
/// Send message to recipients specified in message. For every recipient, if it
/// is found the server is notified through the sender so it can send the private message. In case
/// of error could return the following numeric replies:
///  
/// ERR_NORECIPIENT: no recipient was given.
/// ERR_NOTEXTTOSEND: no text to send was given.
/// ERR_NOSUCHNICK: no channel or user found with given nick.
///
pub fn private_message(
    message: Message,
    users: Arc<Mutex<HashMap<String, User>>>,
    sender: &Sender<Message>,
    channels: Arc<Mutex<HashMap<String, Channel>>>,
    stream: Option<&TcpStream>,
) -> Result<Option<NumericReply>, ServerError> {
    println!("Send a private message!");

    let data = message.params[1].clone();
    let receivers = message.params[0].clone();

    // Check if there are receiver, if not ERR_NORECIPIENT
    if receivers.is_empty() || receivers[0].is_empty() {
        return Ok(Some(NumericReply::new(
            ERR_NORECIPIENT_NUM,
            ERR_NORECIPIENT_MSG,
            Some(vec![message.command]),
        )));
    }

    // Check if there is text to sent, if not ERR_NOTEXTTOSEND
    if data.is_empty() || data[0].is_empty() {
        return Ok(Some(NumericReply::new(
            ERR_NOTEXTTOSEND_NUM,
            ERR_NOTEXTTOSEND_MSG,
            None,
        )));
    }

    // For each receiver notify server
    for receiver in receivers {
        let reply = if receiver.starts_with('#') || receiver.starts_with('&') {
            send_message_to_channel(&receiver, &message, sender, channels.clone())?
        } else {
            send_message_to_user(&receiver, &message, users.clone(), sender)?
        };

        // Write reply if there is any
        if reply.is_some() && stream.is_some() {
            stream
                .unwrap()
                .write_all(reply.unwrap().as_string().as_bytes())
                .map_err(|_| -> ServerError {
                    ServerError {
                        kind: CRITICAL.to_string(),
                        message: "Could not send to server".to_string(),
                    }
                })?;
        }
    }

    Ok(None)
}

/*******************************NOTICE MESSAGE***********************************/

///
/// Send message to recipient. If the recipient is found the server is notified
/// through the sender so it can send the notice. It will never send a numeric reply
/// to the user.
///
pub fn notice(
    message: Message,
    users: Arc<Mutex<HashMap<String, User>>>,
    sender: &Sender<Message>,
) -> Result<Option<NumericReply>, ServerError> {
    // Check if two params were given (nickname and text)
    if message.params_total_count() != 2 {
        return Ok(None);
    }

    // Check if one nickname was given
    let nickname = &message.params[0];

    if nickname.len() != 1 {
        return Ok(None);
    };

    // Check if there's a user wich such nickname and send message
    send_message_to_user(&nickname[0], &message, users, sender)?;
    Ok(None)
}

/*******************************AUX FUNCTIONS***********************************/

///
/// Notifies server that a private message or notice should be sent to the specified user by sending the
/// proper message. If the recipient is not found the numeric reply ERR_NOSUCHNICK ir returned.
///
fn send_message_to_user(
    receiver: &String,
    message: &Message,
    users: Arc<Mutex<HashMap<String, User>>>,
    sender: &Sender<Message>,
) -> Result<Option<NumericReply>, ServerError> {
    let users = users.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not lock users".to_string(),
        }
    })?;
    return match users.get(receiver) {
        Some(user) => match &user.away_message {
            Some(away_message) => {
                let reply = NumericReply::new(
                    RPL_AWAY_NUM,
                    away_message,
                    Some(vec![user.nickname.clone()]),
                );
                Ok(Some(reply))
            }
            None => {
                notify_server_to_send_message(message, receiver, sender)?;
                Ok(None)
            }
        },
        None => {
            println!("Client not found!");
            Ok(Some(NumericReply::new(
                ERR_NOSUCHNICK_NUM,
                ERR_NOSUCHNICK_MSG,
                Some(vec![receiver.to_string()]),
            )))
        }
    };
}

///
/// Notifies server that a private message should be sent to the specified channel be sending the
/// proper message. If the recipient is not found the numeric reply ERR_NOSUCHNICK ir returned.
///
fn send_message_to_channel(
    channel_name: &String,
    message: &Message,
    sender: &Sender<Message>,
    channels: Arc<Mutex<HashMap<String, Channel>>>,
) -> Result<Option<NumericReply>, ServerError> {
    let channels = channels.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not access channels".to_string(),
        }
    })?;

    match channels.get(channel_name) {
        Some(channel) => {
            if !channel.is_user_on_channel(&message.prefix.clone().expect("No prefix found")) {
                return Ok(Some(NumericReply::new(
                    ERR_NOTONCHANNEL_NUM,
                    ERR_NOTONCHANNEL_MSG,
                    Some(vec![channel_name.to_string()]),
                )));
            }
            notify_server_to_send_message(message, channel_name, sender)?;
            Ok(None)
        }
        None => {
            println!("Channel not found!");
            Ok(Some(NumericReply::new(
                ERR_NOSUCHNICK_NUM,
                ERR_NOSUCHNICK_MSG,
                Some(vec![channel_name.to_string()]),
            )))
        }
    }
}

///
/// Notifies server that a private message or notice should be sent to the specified user by sending the
/// proper message.
///
fn notify_server_to_send_message(
    message: &Message,
    receiver: &String,
    sender: &Sender<Message>,
) -> Result<(), ServerError> {
    println!("Message: {:?}", message);
    println!("Found client! {:?}", receiver);

    sender.send(message.clone()).map_err(|_| -> ServerError {
        ServerError {
            kind: CRITICAL.to_string(),
            message: "Could not send to server".to_string(),
        }
    })?;
    Ok(())
}

/************************************TESTS*******************************************/

#[cfg(test)]
mod tests {
    use crate::commands::PRIVMSG;
    use crate::message::Message;
    use crate::numeric_reply::{
        NumericReply, ERR_NORECIPIENT_MSG, ERR_NORECIPIENT_NUM, ERR_NOTEXTTOSEND_MSG,
        ERR_NOTEXTTOSEND_NUM,
    };
    use crate::server_utils::channel::Channel;
    use crate::server_utils::messages_processing_client::messages_exchange::private_message;
    use crate::server_utils::user::User;
    use std::collections::HashMap;
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::sync::{Arc, Mutex};

    #[test]
    fn send_private_message_with_no_recipient_returns_correct_numeric_reply() {
        let _user = User::new(
            "test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let user_2 = User::new(
            "another_test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let aux = TcpListener::bind("127.0.0.1:3000").unwrap();
        let _other = aux.incoming();
        let stream = TcpStream::connect("127.0.0.1:3000").unwrap();

        let message = Message {
            prefix: None,
            command: PRIVMSG.to_string(),
            params: vec![vec![], vec!["test message\r\n".to_string()]],
        };
        let users = Arc::new(Mutex::new(HashMap::from([(
            user_2.nickname.clone(),
            user_2.clone(),
        )])));
        let channels = Arc::new(Mutex::new(HashMap::new()));
        let (sender, _receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();
        let reply = private_message(message, users, &sender, channels, Some(&stream))
            .unwrap()
            .unwrap();

        assert_eq!(
            reply,
            NumericReply::new(
                ERR_NORECIPIENT_NUM,
                ERR_NORECIPIENT_MSG,
                Some(vec![PRIVMSG.to_string()])
            )
        );
    }

    #[test]

    fn send_private_message_with_no_text_returns_correct_numeric_reply() {
        let _user = User::new(
            "test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let user_2 = User::new(
            "another_test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );
        let aux = TcpListener::bind("127.0.0.1:3001").unwrap();
        let _other = aux.incoming();
        let stream = TcpStream::connect("127.0.0.1:3001").unwrap();

        let message = Message {
            prefix: None,
            command: PRIVMSG.to_string(),
            params: vec![vec![user_2.nickname.clone()], vec![]],
        };
        let users = Arc::new(Mutex::new(HashMap::from([(
            user_2.nickname.clone(),
            user_2.clone(),
        )])));
        let channels = Arc::new(Mutex::new(HashMap::new()));
        let (sender, _receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();
        let reply = private_message(message, users, &sender, channels, Some(&stream))
            .unwrap()
            .unwrap();

        assert_eq!(
            reply,
            NumericReply::new(ERR_NOTEXTTOSEND_NUM, ERR_NOTEXTTOSEND_MSG, None)
        )
    }

    #[test]
    fn send_private_message_with_non_existing_recipient_returns_correct_numeric_reply() {
        let _user = User::new(
            "test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );
        let user_2 = User::new(
            "another_test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let aux = TcpListener::bind("127.0.0.1:3002").unwrap();
        let _other = aux.incoming();
        let stream = TcpStream::connect("127.0.0.1:3002").unwrap();

        let message = Message {
            prefix: None,
            command: PRIVMSG.to_string(),
            params: vec![
                vec!["test_recipient".to_string()],
                vec!["test message\r\n".to_string()],
            ],
        };
        let users = Arc::new(Mutex::new(HashMap::from([(
            user_2.nickname.clone(),
            user_2.clone(),
        )])));
        let channels = Arc::new(Mutex::new(HashMap::new()));
        let (sender, _receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();
        let reply = private_message(message, users, &sender, channels, Some(&stream)).unwrap();
        assert!(reply.is_none());
    }

    #[test]
    fn send_private_message_with_one_existing_and_one_non_existing_recipient_returns_correct_numeric_reply(
    ) {
        let _user = User::new(
            "test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let user_2 = User::new(
            "another_test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let aux = TcpListener::bind("127.0.0.1:3003").unwrap();
        let _other = aux.incoming();
        let stream = TcpStream::connect("127.0.0.1:3003").unwrap();

        let message = Message {
            prefix: None,
            command: PRIVMSG.to_string(),
            params: vec![
                vec![user_2.nickname.clone(), "test_recipient".to_string()],
                vec!["test message\r\n".to_string()],
            ],
        };
        let users = Arc::new(Mutex::new(HashMap::from([(
            user_2.nickname.clone(),
            user_2.clone(),
        )])));
        let channels = Arc::new(Mutex::new(HashMap::new()));
        let (sender, _receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();
        let reply = private_message(message, users, &sender, channels, Some(&stream)).unwrap();

        assert!(reply.is_none());
    }

    #[test]
    fn send_private_message_to_non_existing_channel_returns_correct_numeric_reply() {
        let user = User::new(
            "test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );
        let user_2 = User::new(
            "another_test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let aux = TcpListener::bind("127.0.0.1:3004").unwrap();
        let _other = aux.incoming();
        let stream = TcpStream::connect("127.0.0.1:3004").unwrap();

        let channel = Channel::new("test_name".to_string(), &user);

        let message = Message {
            prefix: None,
            command: PRIVMSG.to_string(),
            params: vec![
                vec!["#test_channel".to_string()],
                vec!["test message\r\n".to_string()],
            ],
        };
        let users = Arc::new(Mutex::new(HashMap::from([(
            user_2.nickname.clone(),
            user_2.clone(),
        )])));
        let channels = Arc::new(Mutex::new(HashMap::from([(
            "#".to_string() + &channel.name,
            channel,
        )])));
        let (sender, _receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();
        let reply = private_message(message, users, &sender, channels, Some(&stream)).unwrap();

        assert!(reply.is_none());
    }

    #[test]
    fn send_private_message_to_user_returns_none() {
        let _user = User::new(
            "test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let user_2 = User::new(
            "another_test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );
        let aux = TcpListener::bind("127.0.0.1:3005").unwrap();
        let _other = aux.incoming();
        let stream = TcpStream::connect("127.0.0.1:3005").unwrap();

        let message = Message {
            prefix: None,
            command: PRIVMSG.to_string(),
            params: vec![
                vec![user_2.nickname.clone()],
                vec!["test message\r\n".to_string()],
            ],
        };
        let users = Arc::new(Mutex::new(HashMap::from([(
            user_2.nickname.clone(),
            user_2.clone(),
        )])));
        let channels = Arc::new(Mutex::new(HashMap::new()));
        let (sender, _receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();
        let reply =
            private_message(message.clone(), users, &sender, channels, Some(&stream)).unwrap();

        assert!(reply.is_none());
    }

    #[test]
    fn send_private_message_to_user_sends_message() {
        let user = User::new(
            "test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let user_2 = User::new(
            "another_test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );
        let aux = TcpListener::bind("127.0.0.1:3006").unwrap();
        let _other = aux.incoming();
        let stream = TcpStream::connect("127.0.0.1:3006").unwrap();

        let mut message = Message {
            prefix: Some("test_user".to_string()),
            command: PRIVMSG.to_string(),
            params: vec![
                vec![user_2.nickname.clone()],
                vec!["test message\r\n".to_string()],
            ],
        };
        let users = Arc::new(Mutex::new(HashMap::from([(
            user_2.nickname.clone(),
            user_2.clone(),
        )])));
        let channels = Arc::new(Mutex::new(HashMap::new()));
        let (sender, receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();
        let _reply = private_message(message.clone(), users, &sender, channels, Some(&stream));

        let received_message = receiver.recv().unwrap();
        message.prefix = Some(user.nickname.clone()); //Should receive message with prefix of sender

        assert_eq!(message, received_message)
    }

    #[test]
    fn send_private_message_to_multiple_users_sends_messages() {
        let user = User::new(
            "test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let user_2 = User::new(
            "another_test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let user_3 = User::new(
            "another_test_user_2".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let aux = TcpListener::bind("127.0.0.1:3008").unwrap();
        let _other = aux.incoming();
        let stream = TcpStream::connect("127.0.0.1:3008").unwrap();

        let mut message = Message {
            prefix: Some("test_user".to_string()),
            command: PRIVMSG.to_string(),
            params: vec![
                vec![user_2.nickname.clone(), user_3.nickname.clone()],
                vec!["test message\r\n".to_string()],
            ],
        };
        let users = Arc::new(Mutex::new(HashMap::from([
            (user_2.nickname.clone(), user_2.clone()),
            (user_3.nickname.clone(), user_3.clone()),
        ])));
        let channels = Arc::new(Mutex::new(HashMap::new()));
        let (sender, receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();
        let _reply = private_message(message.clone(), users, &sender, channels, Some(&stream));

        let first_received_message = receiver.recv().unwrap();
        let second_received_message = receiver.recv().unwrap();
        message.prefix = Some(user.nickname.clone()); //Should receive message with prefix of sender, so we add it

        assert_eq!(message, first_received_message);
        assert_eq!(message, second_received_message);
    }

    #[test]
    fn send_private_message_to_channel_returns_none() {
        let user = User::new(
            "test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let user_2 = User::new(
            "another_test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let aux = TcpListener::bind("127.0.0.1:3009").unwrap();
        let _other = aux.incoming();
        let stream = TcpStream::connect("127.0.0.1:3009").unwrap();

        let channel = Channel::new("test_name".to_string(), &user);

        let message = Message {
            prefix: Some("test_user".to_string()),
            command: PRIVMSG.to_string(),
            params: vec![
                vec!["#".to_string() + &channel.name],
                vec!["test message\r\n".to_string()],
            ],
        };
        let users = Arc::new(Mutex::new(HashMap::from([(
            user_2.nickname.clone(),
            user_2.clone(),
        )])));
        let channels = Arc::new(Mutex::new(HashMap::from([(
            "#".to_string() + &channel.name,
            channel,
        )])));
        let (sender, _receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();
        let reply = private_message(message, users, &sender, channels, Some(&stream)).unwrap();

        assert!(reply.is_none());
    }

    #[test]
    fn send_private_message_to_channel_sends_message() {
        let user = User::new(
            "test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let user_2 = User::new(
            "another_test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let aux = TcpListener::bind("127.0.0.1:3010").unwrap();
        let _other = aux.incoming();
        let stream = TcpStream::connect("127.0.0.1:3010").unwrap();

        let channel = Channel::new("test_name".to_string(), &user);

        let mut message = Message {
            prefix: Some("test_user".to_string()),
            command: PRIVMSG.to_string(),
            params: vec![
                vec!["#".to_string() + &channel.name],
                vec!["test message\r\n".to_string()],
            ],
        };
        let users = Arc::new(Mutex::new(HashMap::from([(
            user_2.nickname.clone(),
            user_2.clone(),
        )])));
        let channels = Arc::new(Mutex::new(HashMap::from([(
            "#".to_string() + &channel.name,
            channel,
        )])));
        let (sender, receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();
        let _reply = private_message(message.clone(), users, &sender, channels, Some(&stream));

        let received_message = receiver.recv().unwrap();
        message.prefix = Some(user.nickname.clone()); //Should receive message with prefix of sender

        assert_eq!(message, received_message)
    }

    #[test]
    fn send_private_message_to_multiple_channels_sends_messages() {
        let user = User::new(
            "test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let user_2 = User::new(
            "another_test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );
        let aux = TcpListener::bind("127.0.0.1:3011").unwrap();

        let _other = aux.incoming();
        let stream = TcpStream::connect("127.0.0.1:3011").unwrap();

        let channel = Channel::new("test_name".to_string(), &user);
        let channel_2 = Channel::new("test_name_2".to_string(), &user);

        let mut message = Message {
            prefix: Some("test_user".to_string()),
            command: PRIVMSG.to_string(),
            params: vec![
                vec![
                    "#".to_string() + &channel.name,
                    "#".to_string() + &channel_2.name,
                ],
                vec!["test message\r\n".to_string()],
            ],
        };
        let users = Arc::new(Mutex::new(HashMap::from([(
            user_2.nickname.clone(),
            user_2.clone(),
        )])));
        let channels = Arc::new(Mutex::new(HashMap::from([
            ("#".to_string() + &channel.name, channel),
            ("#".to_string() + &channel_2.name, channel_2),
        ])));
        let (sender, receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();
        let _reply = private_message(message.clone(), users, &sender, channels, Some(&stream));

        let first_received_message = receiver.recv().unwrap();
        let second_received_message = receiver.recv().unwrap();
        message.prefix = Some(user.nickname.clone()); //Should receive message with prefix of sender, so we add it

        assert_eq!(message, first_received_message);
        assert_eq!(message, second_received_message);
    }
}
