//!
//! This module contains functions to handle modifications of multiserver channels.
//!

use crate::custom_errors::server_error::ServerError;
use crate::{
    commands::{
        MODE_GIVE_OP_PRIVILEGES, MODE_REMOVE_BAN, MODE_REMOVE_INVITE, MODE_REMOVE_KEY,
        MODE_REMOVE_LIMIT, MODE_REMOVE_OP_TOPIC, MODE_REMOVE_SECRET, MODE_SET_BAN, MODE_SET_INVITE,
        MODE_SET_KEY, MODE_SET_LIMIT, MODE_SET_OP_TOPIC, MODE_SET_SECRET, MODE_TAKE_OP_PRIVILEGES,
    },
    custom_errors::errors::{CRITICAL, NONCRITICAL},
    message::Message,
    server_utils::{channel::Channel, user::User},
};
use std::{
    collections::HashMap,
    io::Write,
    net::TcpStream,
    sync::{mpsc::Sender, Arc, Mutex},
};
///
/// This function is called when a server receives a join to a channel from
/// another server so that it can add the user to the channel himself to keep
/// track of the users in the channel.
///
pub fn handle_join_server(message: Message, sender: &Sender<Message>) -> Result<(), ServerError> {
    println!("JOIN sending to struct server from hanlder");
    sender.send(message).map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not send".to_string(),
        }
    })?;

    Ok(())
}

///
/// Sends message with away command to server
///
pub fn handle_away_server(message: Message, sender: &Sender<Message>) -> Result<(), ServerError> {
    println!("JOIN sending to struct server from hanlder");
    sender.send(message).map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not send".to_string(),
        }
    })?;

    Ok(())
}

///
/// If channel does not have the kicked user as a member then it notifies other server.
/// If channel does have the kicked user then removes it.
///
pub fn handle_kick_multiserver(
    message: Message,
    mut stream: &TcpStream,
    channels: Arc<Mutex<HashMap<String, Channel>>>,
    sender: &Sender<Message>,
) -> Result<(), ServerError> {
    println!("Handling kick multiserver");

    let channel_name = &message.params[0][0];
    let user_getting_kicked = &message.params[1][0];
    let mut channels = channels.lock().map_err(|_| -> ServerError {
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

    if !channel.is_user_on_channel(user_getting_kicked) {
        // User is not on channel so notify other servers
        println!("MESSAGE SENT from kick server func {:?}", message);
        stream
            .write_all(message.as_string().as_bytes())
            .map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Could not write on stream".to_string(),
                }
            })?;
    } else {
        // User is on channel, remove it
        println!("Removing user {} from channel", user_getting_kicked);
        channel.remove_user(user_getting_kicked);
        sender.send(message).map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Could not send message".to_string(),
            }
        })?;
    }

    Ok(())
}

///
/// This function is called when a server receives a channels info from another server. It will create it and add it to the channels
/// if it doesnt exist. If it does exist it will update the info.
///
pub fn handle_channel_info(
    message: Message,
    channels: Arc<Mutex<HashMap<String, Channel>>>,
    users: Arc<Mutex<HashMap<String, User>>>,
) -> Result<(), ServerError> {
    println!("Received message with channel info: {:?}", message);
    let channel_name = message.params[0][0].clone();
    let mut channels = channels.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not lock channels".to_string(),
        }
    })?;
    let channel = match channels.get(&channel_name) {
        Some(channel) => channel.clone(),
        None => {
            println!("Server does not have channel, adding it");
            Channel::channel_from_message(message, users)?
        }
    };

    println!("Adding channel: {:?}", channel);
    channels.insert(channel.name.clone(), channel);

    Ok(())
}

///
/// This function is called when a server receives a mode from another server. It will update the channel mode.
///
pub fn handle_mode_multiserver(
    message: Message,
    channels: Arc<Mutex<HashMap<String, Channel>>>,
    users: Arc<Mutex<HashMap<String, User>>>,
    sender: &Sender<Message>,
    server_name: String,
) -> Result<(), ServerError> {
    println!("Handling mode multiserver");

    let nickname_user_setting_mode = message.prefix.clone().expect("No prefix in mode message");
    let mut users = users.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not access channels".to_string(),
        }
    })?;

    let user_setting_mode = match users.get_mut(&nickname_user_setting_mode) {
        Some(user) => user,
        None => {
            return Err(ServerError {
                kind: CRITICAL.to_string(),
                message: "user not found".to_string(),
            })
        }
    };

    if user_setting_mode.server_name == server_name {
        // mode already set
        return Ok(());
    }

    let message_clone = message.clone();
    let mode = &message.params[1][0];
    let channel_name = &message.params[0][0];
    let mut channels = channels.lock().map_err(|_| -> ServerError {
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

    let _result = match mode.as_str() {
        MODE_SET_KEY => channel.set_key(message, nickname_user_setting_mode),
        MODE_REMOVE_KEY => channel.remove_key(nickname_user_setting_mode),
        MODE_SET_LIMIT => channel.set_limit(message, nickname_user_setting_mode),
        MODE_REMOVE_LIMIT => channel.remove_limit(nickname_user_setting_mode),
        MODE_SET_INVITE => channel.set_as_invite_only(nickname_user_setting_mode),
        MODE_REMOVE_INVITE => channel.remove_invite_only_status(nickname_user_setting_mode),
        MODE_GIVE_OP_PRIVILEGES => {
            channel.give_operator_privileges(message, nickname_user_setting_mode)
        }
        MODE_TAKE_OP_PRIVILEGES => {
            channel.remove_operator_privileges(message, nickname_user_setting_mode)
        }
        MODE_SET_OP_TOPIC => channel.set_operator_settable_topic(nickname_user_setting_mode),
        MODE_REMOVE_OP_TOPIC => channel.remove_operator_settable_topic(nickname_user_setting_mode),
        MODE_SET_SECRET => channel.set_as_secret(nickname_user_setting_mode),
        MODE_REMOVE_SECRET => channel.remove_secret_status(nickname_user_setting_mode),
        MODE_SET_BAN => channel.set_ban(message, nickname_user_setting_mode),
        MODE_REMOVE_BAN => channel.remove_ban(message, nickname_user_setting_mode),
        &_ => {
            return Err(ServerError {
                kind: CRITICAL.to_string(),
                message: "Unkown mode in multiserver".to_string(),
            })
        }
    };

    sender.send(message_clone).map_err(|_| -> ServerError {
        ServerError {
            kind: CRITICAL.to_string(),
            message: "Could not send message".to_string(),
        }
    })?;

    Ok(())
}

///
/// This function is called when a server receives a part from another server.
///
pub fn handle_part_multiserver(
    message: Message,
    _channels: Arc<Mutex<HashMap<String, Channel>>>,
    _users: Arc<Mutex<HashMap<String, User>>>,
    sender: &Sender<Message>,
) -> Result<(), ServerError> {
    sender.send(message).map_err(|_| -> ServerError {
        ServerError {
            kind: CRITICAL.to_string(),
            message: "Broken channel connection".to_string(),
        }
    })?;

    Ok(())
}

///
/// This function is called when a server receives a topic from another server. It will send the topic to the main thread.
///
pub fn handle_topic(
    message: Message,
    _channels: Arc<Mutex<HashMap<String, Channel>>>,
    sender: &Sender<Message>,
) -> Result<(), ServerError> {
    sender.send(message).map_err(|_| -> ServerError {
        ServerError {
            kind: CRITICAL.to_string(),
            message: "Broken channel connection".to_string(),
        }
    })?;

    Ok(())
}

///
/// This is called when a server receives an invite message from another server, it will check  and send it to the main thread.
///
pub fn handle_invite_multiserver(
    message: Message,
    channels: Arc<Mutex<HashMap<String, Channel>>>,
    _users: Arc<Mutex<HashMap<String, User>>>,
    sender: &Sender<Message>,
) -> Result<(), ServerError> {
    let channel_name = &message.params[1][0];
    let mut channels = channels.lock().map_err(|_| -> ServerError {
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

    let nickname_user_inviting = &message.params[0][0];

    if !channel.is_user_invited(nickname_user_inviting) {
        channel.save_invite(
            nickname_user_inviting,
            message.prefix.clone().expect("No prefix in invite message"),
        );
    }

    sender.send(message).map_err(|_| -> ServerError {
        ServerError {
            kind: CRITICAL.to_string(),
            message: "Broken channel connection".to_string(),
        }
    })?;

    Ok(())
}
