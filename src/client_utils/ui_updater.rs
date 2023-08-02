use crate::client_utils::client::message_types::{ERROR, INFO, PRIVATE_MESSAGE};
use crate::commands::{
    ADD_LIST_CHATS, CORRECT_LOGIN, CORRECT_REGISTRATION, DCC_ACCEPT, DCC_CHAT, DCC_CLOSE,
    DCC_RESUME, DCC_SEND, ERROR_CHANNEL, INVALID_LOGIN, INVALID_REGISTRATION, INVITE, KICK,
    KICK_CHANNEL, LIST_CHANNELS, NAMES, PRIVMSG, QUIT, RECEIVED_MESSAGE, SEARCH_USERS,
};
use crate::custom_errors::client_error::ClientError;
use crate::custom_errors::errors::{
    CRITICAL, LOCK_USERS, NONCRITICAL, RECEIVE_MESSAGE, SEND_MESSAGE,
};
use crate::message::Message;
use crate::numeric_reply::{
    ERR_BADCHANNELKEY_NUM, ERR_BANNEDFROMCHAN_NUM, ERR_CHANNELHASKEY_MSG, ERR_CHANNELHASKEY_NUM,
    ERR_CHANNELISFULL_NUM, ERR_CHANOPRIVSNEEDED_MSG, ERR_CHANOPRIVSNEEDED_NUM,
    ERR_ERRONEUSNICKNAME_NUM, ERR_INVALIDLOGIN_NUM, ERR_INVITEONLYCHAN_NUM, ERR_KEYSET_MSG,
    ERR_KEYSET_NUM, ERR_NEEDMOREPARAMS_MSG, ERR_NEEDMOREPARAMS_NUM, ERR_NICKCOLLISION_NUM,
    ERR_NONICKNAMEGIVEN_MSG, ERR_NONICKNAMEGIVEN_NUM, ERR_NOPRIVILEGES_MSG, ERR_NOPRIVILEGES_NUM,
    ERR_NOSUCHCHANNEL_MSG, ERR_NOSUCHCHANNEL_NUM, ERR_NOSUCHNICK_MSG, ERR_NOSUCHNICK_NUM,
    ERR_NOSUCHSERVER_MSG, ERR_NOSUCHSERVER_NUM, ERR_NOTONCHANNEL_MSG, ERR_NOTONCHANNEL_NUM,
    ERR_PASSWDMISMATCH_MSG, ERR_PASSWDMISMATCH_NUM, ERR_TOOMANYCHANNELS_MSG,
    ERR_TOOMANYCHANNELS_NUM, ERR_UNKNOWNMODE_NUM, ERR_USERONCHANNEL_MSG, ERR_USERONCHANNEL_NUM,
    RPL_AWAY_NUM, RPL_CORRECTLOGIN_NUM, RPL_CORRECTREGISTRATION_NUM, RPL_ENDOFNAMES_NUM,
    RPL_ENDOFWHOIS_NUM, RPL_ENDOFWHO_NUM, RPL_INVITING_NUM, RPL_LISTEND_NUM, RPL_LISTSTART_NUM,
    RPL_LIST_NUM, RPL_MODESET_MSG, RPL_MODESET_NUM, RPL_NAMEREPLY_NUM, RPL_NOTOPIC_NUM,
    RPL_NOWAWAY_MSG, RPL_NOWAWAY_NUM, RPL_TOPIC_NUM, RPL_UNAWAY_MSG, RPL_UNAWAY_NUM,
    RPL_WHOISCHANNELS_NUM, RPL_WHOISOPERATOR_NUM, RPL_WHOISSERVER_NUM, RPL_WHOISUSER_NUM,
    RPL_WHOREPLY_NUM, RPL_YOUREOPER_MSG, RPL_YOUREOPER_NUM,
};
use crate::parser;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

///
/// This function is responsible for receiving messages from the server and updating the UI accordingly.
///
pub fn ui_updater(
    tx_backend: gtk::glib::Sender<Message>,
    rx_stream: Receiver<TcpStream>,
    users_online: &Arc<Mutex<Vec<String>>>,
) -> Result<(), ClientError> {
    let stream = rx_stream.recv().map_err(|_| -> ClientError {
        ClientError {
            kind: CRITICAL.to_string(),
            message: RECEIVE_MESSAGE.to_string(),
        }
    })?;

    let mut data = String::new();
    let mut reader = BufReader::new(stream);
    let mut channels: Vec<String> = Vec::new();
    let mut search_users: Vec<String> = Vec::new();
    while match reader.read_line(&mut data) {
        Ok(bytes_read) => {
            if bytes_read > 0 && !data.is_empty() {
                let users_online = users_online.lock().map_err(|_| -> ClientError {
                    ClientError {
                        kind: CRITICAL.to_string(),
                        message: LOCK_USERS.to_string(),
                    }
                })?;
                let message = parser::parse(data.clone()).expect("Couldn't parse message");
                println!("Received from server: {:?}", message);

                match message.command.clone().as_str() {
                    // Commands
                    PRIVMSG => {
                        parse_message(message, &tx_backend, &users_online)?;
                    }
                    NAMES => {
                        names(message, &tx_backend, &users_online);
                    }
                    KICK => {
                        kick(message, &tx_backend);
                    }
                    INVITE => {
                        let text_to_print = format!(
                            "{} invited you to the channel: {}",
                            message.prefix.expect("No prefix in message"),
                            message.params[1][0].clone()
                        );
                        tx_backend
                            .send(Message {
                                prefix: Some("You".to_string()),
                                command: RECEIVED_MESSAGE.to_string(),
                                params: vec![vec![text_to_print, INFO.to_string()]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    QUIT => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: QUIT.to_string(),
                                params: vec![vec![]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    // Replies
                    RPL_LISTSTART_NUM => {
                        channels.clear();
                    }
                    RPL_LIST_NUM => {
                        channels.push(message.params[0][0].clone());
                    }
                    RPL_LISTEND_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: LIST_CHANNELS.to_string(),
                                params: vec![channels.clone()],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    RPL_TOPIC_NUM | RPL_NOTOPIC_NUM => {
                        topic_message(message, &tx_backend);
                    }
                    RPL_INVITING_NUM => {
                        invite_success(message, &tx_backend);
                    }
                    RPL_WHOREPLY_NUM => {
                        search_users.push(message.params[0][0].clone());
                    }
                    RPL_ENDOFWHO_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: SEARCH_USERS.to_string(),
                                params: vec![search_users.clone()],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                        search_users.clear();
                    }
                    RPL_CORRECTLOGIN_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: CORRECT_LOGIN.to_string(),
                                params: vec![message.params[0].clone()],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    RPL_CORRECTREGISTRATION_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: CORRECT_REGISTRATION.to_string(),
                                params: vec![message.params[0].clone()],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    RPL_NAMEREPLY_NUM => {
                        show_participants(message, &tx_backend);
                    }
                    RPL_NOWAWAY_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: RECEIVED_MESSAGE.to_string(),
                                params: vec![vec![RPL_NOWAWAY_MSG.to_string(), INFO.to_string()]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    RPL_UNAWAY_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: RECEIVED_MESSAGE.to_string(),
                                params: vec![vec![RPL_UNAWAY_MSG.to_string(), INFO.to_string()]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    RPL_AWAY_NUM => {
                        user_away(message, &tx_backend);
                    }
                    RPL_YOUREOPER_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: RECEIVED_MESSAGE.to_string(),
                                params: vec![vec![RPL_YOUREOPER_MSG.to_string(), INFO.to_string()]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    RPL_WHOISUSER_NUM => {
                        whoisuser(message, &tx_backend);
                    }
                    RPL_WHOISCHANNELS_NUM => {
                        let text_to_print = format!(
                            "{} in on the channel: {}",
                            message.params[0][0].clone(),
                            message.params[1][0].clone()
                        );
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: RECEIVED_MESSAGE.to_string(),
                                params: vec![vec![text_to_print, INFO.to_string()]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    RPL_WHOISSERVER_NUM => {
                        let text_to_print = format!(
                            "{} is connected to the server '{}'",
                            message.params[0][0].clone(),
                            message.params[1][0].clone()
                        );
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: RECEIVED_MESSAGE.to_string(),
                                params: vec![vec![text_to_print, INFO.to_string()]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    RPL_WHOISOPERATOR_NUM => {
                        let text_to_print = format!(
                            "{} {}",
                            message.params[0][0].clone(),
                            message.params[1][0].clone()
                        );
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: RECEIVED_MESSAGE.to_string(),
                                params: vec![vec![text_to_print, INFO.to_string()]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    RPL_MODESET_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: RECEIVED_MESSAGE.to_string(),
                                params: vec![vec![RPL_MODESET_MSG.to_string(), INFO.to_string()]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    RPL_ENDOFWHOIS_NUM => {}
                    RPL_ENDOFNAMES_NUM => {}

                    // Errors
                    ERR_INVALIDLOGIN_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: INVALID_LOGIN.to_string(),
                                params: vec![],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    ERR_NICKCOLLISION_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: INVALID_REGISTRATION.to_string(),
                                params: vec![vec!["Nickname is already in use".to_string()]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    ERR_ERRONEUSNICKNAME_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: INVALID_REGISTRATION.to_string(),
                                params: vec![vec![
                                    "Nickname cannot start with #, & or :".to_string()
                                ]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    ERR_NEEDMOREPARAMS_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: RECEIVED_MESSAGE.to_string(),
                                params: vec![vec![
                                    ERR_NEEDMOREPARAMS_MSG.to_string(),
                                    ERROR.to_string(),
                                ]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    ERR_PASSWDMISMATCH_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: RECEIVED_MESSAGE.to_string(),
                                params: vec![vec![
                                    ERR_PASSWDMISMATCH_MSG.to_string(),
                                    ERROR.to_string(),
                                ]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    ERR_NOPRIVILEGES_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: RECEIVED_MESSAGE.to_string(),
                                params: vec![vec![
                                    ERR_NOPRIVILEGES_MSG.to_string(),
                                    ERROR.to_string(),
                                ]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    ERR_NOSUCHSERVER_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: RECEIVED_MESSAGE.to_string(),
                                params: vec![vec![
                                    ERR_NOSUCHSERVER_MSG.to_string(),
                                    ERROR.to_string(),
                                ]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    ERR_NONICKNAMEGIVEN_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: RECEIVED_MESSAGE.to_string(),
                                params: vec![vec![
                                    ERR_NONICKNAMEGIVEN_MSG.to_string(),
                                    ERROR.to_string(),
                                ]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    ERR_NOSUCHNICK_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: RECEIVED_MESSAGE.to_string(),
                                params: vec![vec![
                                    ERR_NOSUCHNICK_MSG.to_string(),
                                    ERROR.to_string(),
                                ]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }

                    // Channel errors
                    ERR_INVITEONLYCHAN_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: ERROR_CHANNEL.to_string(),
                                params: vec![vec!["You aren't invited to the channel".to_string()]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    ERR_BADCHANNELKEY_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: ERROR_CHANNEL.to_string(),
                                params: vec![vec!["The key entered is not valid".to_string()]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    ERR_CHANNELHASKEY_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: ERROR_CHANNEL.to_string(),
                                params: vec![vec![ERR_CHANNELHASKEY_MSG.to_string()]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    ERR_BANNEDFROMCHAN_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: ERROR_CHANNEL.to_string(),
                                params: vec![vec!["You are banned from channel".to_string()]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    ERR_CHANNELISFULL_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: ERROR_CHANNEL.to_string(),
                                params: vec![vec!["Channel is full".to_string()]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    ERR_TOOMANYCHANNELS_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: ERROR_CHANNEL.to_string(),
                                params: vec![vec![ERR_TOOMANYCHANNELS_MSG.to_string()]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    ERR_NOSUCHCHANNEL_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: ERROR_CHANNEL.to_string(),
                                params: vec![vec![ERR_NOSUCHCHANNEL_MSG.to_string()]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    ERR_USERONCHANNEL_NUM => {
                        send_custom_error_to_channel(
                            message,
                            &tx_backend,
                            ERR_USERONCHANNEL_MSG.to_string(),
                        );
                    }
                    ERR_CHANOPRIVSNEEDED_NUM => {
                        send_custom_error_to_channel(
                            message,
                            &tx_backend,
                            ERR_CHANOPRIVSNEEDED_MSG.to_string(),
                        );
                    }
                    ERR_UNKNOWNMODE_NUM => {
                        let text_to_print = format!(
                            "{} is not a valid mode, allowed: +/-[k, l, i, o, t, s, b].",
                            message.params[0][0].clone()
                        );
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: RECEIVED_MESSAGE.to_string(),
                                params: vec![vec![text_to_print, ERROR.to_string()]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    ERR_KEYSET_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: RECEIVED_MESSAGE.to_string(),
                                params: vec![vec![ERR_KEYSET_MSG.to_string(), ERROR.to_string()]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }
                    ERR_NOTONCHANNEL_NUM => {
                        tx_backend
                            .send(Message {
                                prefix: None,
                                command: RECEIVED_MESSAGE.to_string(),
                                params: vec![vec![
                                    ERR_NOTONCHANNEL_MSG.to_string(),
                                    ERROR.to_string(),
                                ]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })?;
                    }

                    &_ => println!("Undefined message received by the server"),
                }
                data.clear();
            }
            true
        }
        Err(e) => {
            println!("Error reading {}", e);
            false
        }
    } {}

    Ok(())
}

fn parse_message(
    message: Message,
    tx_backend: &gtk::glib::Sender<Message>,
    users: &[String],
) -> Result<(), ClientError> {
    println!("Parseo mensaje -> {:?}", message);
    let prefix = message.prefix.clone();

    let message_to_parse = message.params[1][0].clone() + "\r\n";
    println!("Mensaje a parsear: {}", message_to_parse);

    match parser::parse(message_to_parse) {
        Ok(mut message) => {
            println!("message parsed: {:?} ", message);
            if message.command == *DCC_CHAT
                || message.command == *DCC_SEND
                || message.command == *DCC_CLOSE
                || message.command == *DCC_RESUME
                || message.command == *DCC_ACCEPT
            {
                let user_nick = prefix.clone().expect("No prefix in message");
                message.prefix = prefix.clone();
                if !users.contains(&user_nick) {
                    // If the user is not in the list of users, we add it
                    tx_backend
                        .send(Message {
                            prefix: Some(user_nick),
                            command: ADD_LIST_CHATS.to_string(),
                            params: vec![vec![]],
                        })
                        .map_err(|_| -> ClientError {
                            ClientError {
                                kind: NONCRITICAL.to_string(),
                                message: SEND_MESSAGE.to_string(),
                            }
                        })
                        .ok();
                }
                if message.command == *DCC_CHAT {
                    let aux = format!("{} Sent you an invitation to start a DCC chat.\r\n Do you accept? \r\n (Time to accept: 10 seconds)", prefix.expect("No prefix in message"));
                    message.params.push(vec![aux]);
                } else if message.command == *DCC_SEND {
                    let aux = format!("{} wants to send you the file {} with a total weight of {} bytes.\r\n Do you accept?", prefix.expect("No prefix in message"), message.params[0][0], message.params[3][0]);
                    message.params.push(vec![aux]);
                } else if message.command == *DCC_RESUME {
                    let aux = format!(
                        "{} wants to continue the transfer of the file {}.\r\n Do you accept?",
                        prefix.expect("No prefix in message"),
                        message.params[0][0]
                    );
                    message.params.push(vec![aux]);
                }
                tx_backend.send(message).map_err(|_| -> ClientError {
                    ClientError {
                        kind: NONCRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })?;
                return Ok(());
            }
        }
        Err(_) => {
            println!("Error al parsear")
        }
    };

    private_message(message, tx_backend, users);
    Ok(())
}

///
/// The client receives a PRIVMSG from the server
/// If a user is not in the list of users online, it sends a ADD_LIST_CHATS command to the main thread to add it to the list
/// If the user is in the list of users online or is a message from channel, it sends a RECEIVED_MESSAGE command to the main thread to update the UI
///
fn private_message(message: Message, tx_backend: &gtk::glib::Sender<Message>, users: &[String]) {
    let user_nick = message.prefix.expect("No prefix in message");
    let message_receiver = message.params[0][0].clone();

    let message_is_from_channel =
        message_receiver.starts_with('#') || message_receiver.starts_with('&');

    // If the message is from a channel, the prefix is the channel name
    let prefix = if message_is_from_channel {
        // Ignore the message if the channel is not in the list of channels
        if !users.contains(&message_receiver) {
            return;
        }
        message_receiver
    } else {
        // If the message is from a user, the prefix is the user nick
        // If the user is not in the list of users online, it sends a ADD_LIST_CHATS command to the main thread to add it to the list
        if !users.contains(&user_nick) {
            tx_backend
                .send(Message {
                    prefix: Some(user_nick.clone()),
                    command: ADD_LIST_CHATS.to_string(),
                    params: vec![vec![]],
                })
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: NONCRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })
                .ok();
        }
        user_nick.clone()
    };

    let message_to_print = format!("{}: {}", user_nick, message.params[1][0].clone());
    tx_backend
        .send(Message {
            prefix: Some(prefix),
            command: RECEIVED_MESSAGE.to_string(),
            params: vec![vec![message_to_print, PRIVATE_MESSAGE.to_string()]],
        })
        .map_err(|_| -> ClientError {
            ClientError {
                kind: NONCRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            }
        })
        .ok();
}

///
/// The client receives a NAMES from the server
/// It sends a ADD_LIST_CHATS command to the main thread to add the users to the list
///
fn names(message: Message, tx_backend: &gtk::glib::Sender<Message>, users: &[String]) {
    if !message.params.is_empty() {
        for user in message.params[0].clone() {
            if users.contains(&user) {
                continue;
            }
            tx_backend
                .send(Message {
                    prefix: Some(user.clone()),
                    command: ADD_LIST_CHATS.to_string(),
                    params: vec![vec![]],
                })
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: NONCRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })
                .ok();
        }
    }
}

///
/// The client receives a RPL_INVITING from the server
/// Sends a RECEIVED_MESSAGE command to the main thread to let the user know that it has been invited to a channel
///
fn invite_success(message: Message, tx_backend: &gtk::glib::Sender<Message>) {
    let channel = message.params[0][0].clone();
    let user = message.params[1][0].clone();
    let text_to_print = format!(
        "Your invitation for {} to the channel {} was sent.",
        user, channel
    );
    tx_backend
        .send(Message {
            prefix: Some(channel),
            command: RECEIVED_MESSAGE.to_string(),
            params: vec![vec![text_to_print, INFO.to_string()]],
        })
        .map_err(|_| -> ClientError {
            ClientError {
                kind: NONCRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            }
        })
        .ok();
}

///
/// The client receives a RPL_TOPIC from the server
/// Try to add the channel to the list of chats, if it already exists, it will not be added
/// Sends a RECEIVED_MESSAGE command to the main thread to update the topic of the channel
///
fn topic_message(message: Message, tx_backend: &gtk::glib::Sender<Message>) {
    let channel = message.params[0][0].clone();
    let topic = message.params[1][0].clone();

    // We don't know if the channel is in the list of channels, so we try to add it
    tx_backend
        .send(Message {
            prefix: Some(channel.clone()),
            command: ADD_LIST_CHATS.to_string(),
            params: vec![vec![]],
        })
        .map_err(|_| -> ClientError {
            ClientError {
                kind: NONCRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            }
        })
        .ok();

    let text_to_print = format!("Channel topic: {}", topic);
    tx_backend
        .send(Message {
            prefix: Some(channel),
            command: RECEIVED_MESSAGE.to_string(),
            params: vec![vec![text_to_print, INFO.to_string()]],
        })
        .map_err(|_| -> ClientError {
            ClientError {
                kind: NONCRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            }
        })
        .ok();
}

///
/// The client receives a RPL_NAMEREPLY from the server
/// It filters the current user and sends a RECEIVED_MESSAGE command to the main thread to print the list of users online
///
fn show_participants(message: Message, tx_backend: &gtk::glib::Sender<Message>) {
    let this_user = message.params[0][0].clone();
    let channel = message.params[1][0].clone();
    let users = message.params[2][0].split(' ').collect::<Vec<&str>>();

    // Filter this user
    let users_without_this_user = users
        .iter()
        .filter(|user| **user != this_user)
        .copied()
        .collect::<Vec<&str>>();

    let text_to_print = match users_without_this_user.is_empty() {
        true => "No one is here yet!".to_string(),
        false => format!("Users in channel: {}", users_without_this_user.join(", ")),
    };

    tx_backend
        .send(Message {
            prefix: Some(channel),
            command: RECEIVED_MESSAGE.to_string(),
            params: vec![vec![text_to_print, INFO.to_string()]],
        })
        .map_err(|_| -> ClientError {
            ClientError {
                kind: NONCRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            }
        })
        .ok();
}

///
/// The client received a message from user in away mode
///
fn user_away(message: Message, tx_backend: &gtk::glib::Sender<Message>) {
    let user_away = message.params[0][0].clone();
    let message_away = message.params[1][0].clone();
    let message_to_print = format!("{} is away '{}'", user_away, message_away);
    tx_backend
        .send(Message {
            prefix: Some(user_away),
            command: RECEIVED_MESSAGE.to_string(),
            params: vec![vec![message_to_print, INFO.to_string()]],
        })
        .map_err(|_| -> ClientError {
            ClientError {
                kind: NONCRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            }
        })
        .ok();
}

///
/// Sends a RECEIVED_MESSAGE command to the main thread to print the custom error on the cannel
///
fn send_custom_error_to_channel(
    message: Message,
    tx_backend: &gtk::glib::Sender<Message>,
    message_to_print: String,
) {
    let channel = message.params[0][0].clone();
    let mut text_to_print = message_to_print.to_string();
    if message.params_total_count() >= 3 {
        let user = message.params[1][0].clone();
        text_to_print = format!("{} {}", user, message_to_print);
    }
    tx_backend
        .send(Message {
            prefix: Some(channel),
            command: RECEIVED_MESSAGE.to_string(),
            params: vec![vec![text_to_print, ERROR.to_string()]],
        })
        .map_err(|_| -> ClientError {
            ClientError {
                kind: NONCRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            }
        })
        .ok();
}

///
/// Sends a RECEIVED_MESSAGE command to the main thread to let the user know taht it has been kicked
/// Sends a KICK_CHANNEL command to the main thread to remove the channel from the list of channels
///
fn kick(message: Message, tx_backend: &gtk::glib::Sender<Message>) {
    let channel = message.params[0][0].clone();
    let text_to_print = format!("You have been kicked from the channel {}", channel);
    tx_backend
        .send(Message {
            prefix: Some("You".to_string()),
            command: RECEIVED_MESSAGE.to_string(),
            params: vec![vec![text_to_print, INFO.to_string()]],
        })
        .map_err(|_| -> ClientError {
            ClientError {
                kind: NONCRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            }
        })
        .ok();
    tx_backend
        .send(Message {
            prefix: None,
            command: KICK_CHANNEL.to_string(),
            params: vec![vec![channel]],
        })
        .map_err(|_| -> ClientError {
            ClientError {
                kind: NONCRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            }
        })
        .ok();
}

///
/// The client receives a RPL_WHOISUSER from the server
/// Creates the text and sends a RECEIVED_MESSAGE command to the main thread to print it
///
fn whoisuser(message: Message, tx_backend: &gtk::glib::Sender<Message>) {
    let nickname = message.params[0][0].clone();
    let username = message.params[1][0].clone();
    let server_ip = message.params[2][0].clone();
    let realname = message.params[4][0].clone();
    let text_to_print = format!(
        "{}, has username '{}' and realname '{}'.",
        nickname, username, realname
    );
    tx_backend
        .send(Message {
            prefix: None,
            command: RECEIVED_MESSAGE.to_string(),
            params: vec![vec![text_to_print, INFO.to_string()]],
        })
        .map_err(|_| -> ClientError {
            ClientError {
                kind: NONCRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            }
        })
        .ok();
    let text_to_print = format!("The host is: {}.", server_ip);
    tx_backend
        .send(Message {
            prefix: None,
            command: RECEIVED_MESSAGE.to_string(),
            params: vec![vec![text_to_print, INFO.to_string()]],
        })
        .map_err(|_| -> ClientError {
            ClientError {
                kind: NONCRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            }
        })
        .ok();
}
