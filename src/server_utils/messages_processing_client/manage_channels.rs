//! CHANNELS
//!
//! This module contains functions to handle channel commands
//!

use std::{
    collections::HashMap,
    io::Write,
    net::TcpStream,
    sync::{mpsc::Sender, Arc, Mutex},
};

use crate::custom_errors::server_error::ServerError;
use crate::message::Message;
use crate::numeric_reply::*;
use crate::server_utils::{channel::Channel, user::User};
use crate::{
    commands::{
        JOIN, MODE_GIVE_OP_PRIVILEGES, MODE_REMOVE_BAN, MODE_REMOVE_INVITE, MODE_REMOVE_KEY,
        MODE_REMOVE_LIMIT, MODE_REMOVE_OP_TOPIC, MODE_REMOVE_SECRET, MODE_SET_BAN, MODE_SET_INVITE,
        MODE_SET_KEY, MODE_SET_LIMIT, MODE_SET_OP_TOPIC, MODE_SET_SECRET, MODE_TAKE_OP_PRIVILEGES,
        PART,
    },
    custom_errors::errors::{CRITICAL, NONCRITICAL},
    numeric_reply::{
        NumericReply, ERR_NEEDMOREPARAMS_MSG, ERR_NEEDMOREPARAMS_NUM, ERR_NOSUCHCHANNEL_MSG,
        ERR_NOSUCHCHANNEL_NUM, ERR_NOSUCHNICK_MSG, ERR_NOSUCHNICK_NUM, ERR_NOTONCHANNEL_MSG,
        ERR_NOTONCHANNEL_NUM, ERR_UNKNOWNMODE_MSG, ERR_UNKNOWNMODE_NUM, RPL_ENDOFNAMES_MSG,
        RPL_ENDOFNAMES_NUM, RPL_INVITING_NUM, RPL_LISTEND_MSG, RPL_LISTEND_NUM, RPL_LISTSTART_MSG,
        RPL_LISTSTART_NUM, RPL_LIST_NUM, RPL_NAMEREPLY_NUM, RPL_NOTOPIC_MSG, RPL_NOTOPIC_NUM,
        RPL_TOPIC_NUM,
    },
};

/********************************JOIN MESSAGE*************************************/

///
/// Receives a Message to join user to one channel or several. In case of error could return the following
/// numeric replies:
///
/// ERR_NEEDMOREPARAMS: channel name not suplied.
/// ERR_INVITEONLYCHAN: user trying to join wasnt invited.
/// ERR_BADCHANNELKEY: user trying to join with incorrect key.
/// ERR_CHANNELISFULL: channel has a limit of participants and reached it.               
/// ERR_TOOMANYCHANNELS: user already joined 10 channels, cant join another one.
///
pub fn join_channel(
    mut stream: &TcpStream,
    message: Message,
    channels: &Arc<Mutex<HashMap<String, Channel>>>,
    users: &Arc<Mutex<HashMap<String, User>>>,
    user: &User,
    sender: &Sender<Message>,
) -> Result<Option<NumericReply>, ServerError> {
    println!("In Join a channel! Message: {:?}", message);

    let mut passwords: Vec<String> = vec![];
    let channels_names = message.params[0].clone();
    let mut binding = users.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not lock users".to_string(),
        }
    })?;
    let user = match binding.get_mut(&user.nickname) {
        Some(u) => u,
        None => {
            return Err(ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Could not found user".to_string(),
            })
        }
    };
    if message.params_total_count() == 0 {
        return Ok(Some(NumericReply::new(
            ERR_NEEDMOREPARAMS_NUM,
            ERR_NEEDMOREPARAMS_MSG,
            None,
        )));
    }
    if message.params_total_count() > 1 {
        passwords = message.params[1].clone();
    }

    let mut channels = channels.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not lock channels".to_string(),
        }
    })?;

    for (i, channel_name) in channels_names.into_iter().enumerate() {
        match channels.get_mut(&channel_name) {
            //Check if the channel exists
            Some(channel) => {
                // If exists joins
                println!("Channel found!");
                let mut password: Option<String> = None;

                if passwords.len() > i {
                    password = Some(passwords[i].clone())
                }

                let reply = channel.join(user.clone(), password)?;

                if channel.is_multiserver()
                    && reply.has_number(vec![RPL_TOPIC_NUM, RPL_NOTOPIC_NUM])
                {
                    let new_user_message = Message {
                        prefix: Some(user.nickname.clone()),
                        command: JOIN.to_string(),
                        params: vec![vec![channel_name.clone()]],
                    };

                    sender.send(new_user_message).map_err(|_| -> ServerError {
                        ServerError {
                            kind: CRITICAL.to_string(),
                            message: "Could not send to server".to_string(),
                        }
                    })?;
                }

                stream
                    .write_all(reply.as_string().as_bytes())
                    .map_err(|_| -> ServerError {
                        ServerError {
                            kind: CRITICAL.to_string(),
                            message: "Could not send to server".to_string(),
                        }
                    })?;
            }
            None => {
                // If it doesn't exist create one
                println!("Channel not found! Creating channel");

                let channel = Channel::new(channel_name.clone(), &user.clone());

                // If channel is multiserver notify server soit notifies
                // all server a new channel was created
                if channel.is_multiserver() {
                    println!("Channel is multiserver notify server");
                    let message_new_channel = Message {
                        prefix: Some(user.nickname.clone()),
                        command: JOIN.to_string(),
                        params: vec![vec![channel_name.clone()]],
                    };

                    sender
                        .send(message_new_channel)
                        .map_err(|_| -> ServerError {
                            ServerError {
                                kind: CRITICAL.to_string(),
                                message: "Could not send to server".to_string(),
                            }
                        })?;
                }

                channels.insert(channel_name.to_string(), channel);

                //channels.insert(channel_name.clone(), channel);
                let reply = NumericReply::new(
                    RPL_NOTOPIC_NUM,
                    RPL_NOTOPIC_MSG,
                    Some(vec![channel_name.clone()]),
                );

                stream
                    .write_all(reply.as_string().as_bytes())
                    .map_err(|_| -> ServerError {
                        ServerError {
                            kind: CRITICAL.to_string(),
                            message: "Could not write in stream".to_string(),
                        }
                    })?;
            }
        }
        user.add_channel(&channel_name);
    }
    Ok(None)
}

/********************************LIST MESSAGE*************************************/

///
/// This function is called when a user sends a LIST message to the server
/// with a specific name or list of names. It will return a list of the servers
/// that match with a name received
///
pub fn list_channels(
    message: Message,
    channels: &Arc<Mutex<HashMap<String, Channel>>>,
    mut stream: &TcpStream,
) -> Result<Option<NumericReply>, ServerError> {
    println!("List channels!");

    //RPL_LISTSTART
    let reply = NumericReply::new(
        RPL_LISTSTART_NUM,
        RPL_LISTSTART_MSG,
        Some(vec!["Channel".to_string()]),
    );
    stream
        .write_all(reply.as_string().as_bytes())
        .map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Could not write on stream".to_string(),
            }
        })?;

    if message.params_total_count() == 0 {
        // Return all channels
        list_all_channels(channels, stream)?;
    } else {
        // Return channels received in message
        list_some_channels(message, channels, stream)?;
    }

    //RPL_LISTEND
    let reply = NumericReply::new(RPL_LISTEND_NUM, RPL_LISTEND_MSG, None);
    stream
        .write_all(reply.as_string().as_bytes())
        .map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Could not write on stream".to_string(),
            }
        })?;

    Ok(None)
}

///
/// This function is called to list all channels known by the server if it is
/// possible
///
pub fn list_all_channels(
    channels: &Arc<Mutex<HashMap<String, Channel>>>,
    mut stream: &TcpStream,
) -> Result<(), ServerError> {
    println!("Return all channels");

    let channels = channels.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not access channels".to_string(),
        }
    })?;

    for channel in channels.values() {
        if channel.is_secret() {
            continue;
        }

        let topic = if let Some(t) = channel.topic.clone() {
            t.to_owned()
        } else {
            "No topic".to_string()
        };

        let args = Some(vec![channel.name.clone(), "visibility".to_string()]);
        let reply = NumericReply::new(RPL_LIST_NUM, topic.as_str(), args); //channels shouldnt appear unless the client is part of the channel
        stream
            .write_all(reply.as_string().as_bytes())
            .map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Could not write on stream".to_string(),
                }
            })?;
    }

    Ok(())
}

///
/// LISTS channels received in message
///
pub fn list_some_channels(
    message: Message,
    channels: &Arc<Mutex<HashMap<String, Channel>>>,
    mut stream: &TcpStream,
) -> Result<(), ServerError> {
    let channels_to_show = message.params[0].clone();
    let channels_existing = channels.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not access channels".to_string(),
        }
    })?;

    for channel in channels_to_show.into_iter() {
        if let Some(channel_act) = channels_existing.get(&channel) {
            if channel_act.is_secret() {
                continue;
            }

            let topic = if let Some(t) = channel_act.topic.clone() {
                t.to_owned()
            } else {
                "No topic".to_string()
            };

            let args = Some(vec![channel_act.name.clone(), "visibility".to_string()]);
            let reply = NumericReply::new(RPL_LIST_NUM, topic.as_str(), args);
            stream
                .write_all(reply.as_string().as_bytes())
                .map_err(|_| -> ServerError {
                    ServerError {
                        kind: CRITICAL.to_string(),
                        message: "Could not write on stream".to_string(),
                    }
                })?;
        };
    }

    Ok(())
}

/********************************PART MESSAGE*************************************/

///
/// Handles the PART message received from the client, it will remove the user from the channel
///
pub fn part_channel(
    message: Message,
    channels: &Arc<Mutex<HashMap<String, Channel>>>,
    user: &User,
    mut stream: &TcpStream,
    sender: &Sender<Message>,
) -> Result<Option<NumericReply>, ServerError> {
    if message.params_total_count() == 0 {
        println!("No channel name provided");
        return Ok(Some(NumericReply::new(
            ERR_NEEDMOREPARAMS_NUM,
            ERR_NEEDMOREPARAMS_MSG,
            None,
        )));
    }

    let channels_to_leave = message.params[0].clone();
    let mut channels = channels.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not access channels".to_string(),
        }
    })?;
    let mut empty_channels = vec![];

    for channel in channels_to_leave.into_iter() {
        match channels.get_mut(&channel) {
            Some(channel_act) => {
                let reply = channel_act.part(user.clone());

                if channel_act.is_empty() {
                    empty_channels.push(channel.clone());
                }

                if reply.is_some() {
                    stream
                        .write_all(reply.unwrap().as_string().as_bytes())
                        .map_err(|_| -> ServerError {
                            ServerError {
                                kind: CRITICAL.to_string(),
                                message: "Broken stream connection".to_string(),
                            }
                        })?;
                } else if channel_act.is_multiserver() {
                    let part_message = Message {
                        prefix: Some(user.nickname.clone()),
                        command: PART.to_string(),
                        params: vec![vec![channel_act.name.clone()]],
                    };

                    sender.send(part_message).map_err(|_| -> ServerError {
                        ServerError {
                            kind: CRITICAL.to_string(),
                            message: "Broken channel connection".to_string(),
                        }
                    })?;
                }
            }
            None => {
                let reply = NumericReply::new(ERR_NOSUCHCHANNEL_NUM, ERR_NOSUCHCHANNEL_MSG, None);
                stream
                    .write_all(reply.as_string().as_bytes())
                    .map_err(|_| -> ServerError {
                        ServerError {
                            kind: CRITICAL.to_string(),
                            message: "Broken stream connection".to_string(),
                        }
                    })?;
            }
        };
    }

    for channel in empty_channels {
        channels.remove(&channel);
    }

    Ok(None)
}

/********************************MODE MESSAGE*************************************/

///
/// Changes channel mode according to Message received. The available modes are:
///
/// +k: set a channel key (password). ej MODE #ChannelName +k pass.
/// +l: set the user limit to channel. ej MODE #ChannelName +l 10.
/// +i: set channel as invite only. ej MODE #ChannelName +i.
/// +o: give/take channel operator privilege. ej MODE #ChannelName +o ari.
/// +t: topic settable by channel operator only flag. ej MODE #ChannelName +t.
/// +s: secret channel flag.
/// +b: bans nicknames given.
///
/// In case of error could return the following numeric replies:
///
/// ERR_NEEDMOREPARAMS: not enough params suplied.
/// ERR_CHANOPRIVSNEEDED: non operator trying to use operator privileges.
/// ERR_NOSUCHNICK: no such nickname on channel?
/// ERR_NOTONCHANNEL: user is not member of channel. DONE
/// ERR_KEYSET: key already set. DONE
/// ERR_UNKNOWNMODE: non existing mode. DONE
/// ERR_NOSUCHCHANNEL: channel name invalid. DONE
/// ERR_USERSDONTMATCH:              
/// ERR_USERNOTINCHANNEL?? lo agregamos este?
///
/// RPL_UMODEIS:
/// RPL_BANLIST:
/// RPL_CHANNELMODEIS:
/// RPL_ENDOFBANLIST:
///
pub fn set_channel_mode(
    message: Message,
    channels: &Arc<Mutex<HashMap<String, Channel>>>,
    user: &mut User,
    sender: &Sender<Message>,
    stream: &TcpStream,
) -> Result<Option<NumericReply>, ServerError> {
    println!("Set channel mode function");
    // Check if channel and mode were given
    if message.params_total_count() < 2 {
        return Ok(Some(NumericReply::new(
            ERR_NEEDMOREPARAMS_NUM,
            ERR_NEEDMOREPARAMS_MSG,
            None,
        )));
    };

    let channel_name = &message.params[0][0];

    println!("Setting channel {} mode", channel_name);

    let mut channels = channels.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not access channels".to_string(),
        }
    })?;

    return match channels.get_mut(channel_name) {
        Some(channel) => Ok(handle_mode(
            channel,
            message,
            user.nickname.clone(),
            sender,
            stream,
        )?),
        None => Ok(Some(NumericReply::new(
            ERR_NOSUCHCHANNEL_NUM,
            ERR_NOSUCHCHANNEL_MSG,
            Some(vec![channel_name.to_string()]),
        ))),
    };
}

///
/// This function is used to handle the mode command.
///
fn handle_mode(
    channel: &mut Channel,
    message: Message,
    nickname_user_setting_mode: String,
    sender: &Sender<Message>,
    mut stream: &TcpStream,
) -> Result<Option<NumericReply>, ServerError> {
    println!("Handling mode");
    if !channel.is_user_on_channel(&nickname_user_setting_mode) {
        return Ok(Some(NumericReply::new(
            ERR_NOTONCHANNEL_NUM,
            ERR_NOTONCHANNEL_MSG,
            Some(vec![channel.name.clone()]),
        )));
    }

    let mode = &message.params[1][0].clone();
    println!("Process mode {}", mode);
    let message_clone = message.clone();

    let result = match mode.to_string().as_str() {
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
        &_ => Err(NumericReply::new(
            ERR_UNKNOWNMODE_NUM,
            ERR_UNKNOWNMODE_MSG,
            Some(vec![mode.to_string()]),
        )),
    };

    match result {
        Ok(_) => {
            if channel.is_multiserver() {
                sender.send(message_clone).map_err(|_| -> ServerError {
                    ServerError {
                        kind: CRITICAL.to_string(),
                        message: "Could not send to server".to_string(),
                    }
                })?;
            }
            stream
                .write_all(
                    NumericReply::new(
                        RPL_MODESET_NUM,
                        RPL_MODESET_MSG,
                        Some(vec![channel.name.clone(), mode.to_string()]),
                    )
                    .as_string()
                    .as_bytes(),
                )
                .map_err(|_| -> ServerError {
                    ServerError {
                        kind: CRITICAL.to_string(),
                        message: "Could not write on stream".to_string(),
                    }
                })?;

            Ok(None)
        }
        Err(reply) => Ok(Some(reply)),
    }
}

/********************************INVITE MESSAGE*************************************/

///
/// Invites user to channel according to Message received. If the invitation
/// was successful and is being passed onto the end client returns RPL_INVITING.
/// In cases of error could return the followig numeric replies:
///
/// ERR_NEEDMOREPARAMS: not enough parametes were given.
/// ERR_NOSUCHNICK: nickname or channel name do not exist.
/// ERR_NOTONCHANNEL: user trying to invite is not on channel.
/// ERR_USERONCHANNEL: user already on channel.
/// ERR_CHANOPRIVSNEEDED: non operator trying to invite.
///
///  
pub fn invite_to_channel(
    sender: &Sender<Message>,
    message: Message,
    channels: Arc<Mutex<HashMap<String, Channel>>>,
    users: Arc<Mutex<HashMap<String, User>>>,
    user: &User,
) -> Result<Option<NumericReply>, ServerError> {
    // Check if enough parameters were given
    if message.params_total_count() < 2 {
        return Ok(Some(NumericReply::new(
            ERR_NEEDMOREPARAMS_NUM,
            ERR_NEEDMOREPARAMS_MSG,
            None,
        )));
    }

    let nick_user_to_invite = message.params[0][0].clone();
    let channel_name = &message.params[1][0];
    let mut channels = channels.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not access channels".to_string(),
        }
    })?;
    let users = users.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not lock users".to_string(),
        }
    })?;

    println!("Invite user {}", nick_user_to_invite);

    if users.get(&nick_user_to_invite).is_none() {
        return Ok(Some(NumericReply::new(
            ERR_NOSUCHNICK_NUM,
            ERR_NOSUCHNICK_MSG,
            Some(vec![nick_user_to_invite]),
        )));
    }

    return match channels.get_mut(channel_name) {
        Some(channel) => {
            // Channel found, try to invite user

            // Try to save invite in channel, if return a numeric reply then an error was found
            let save_reply = channel.save_invite(&nick_user_to_invite, user.nickname.clone());

            // If error was found return numeric reply that specifies it
            if save_reply.is_some() {
                println!("invite to channel failed");
                return Ok(save_reply);
            }
            println!("invited to channel {}", nick_user_to_invite);

            // Notify server to send invitation
            sender
                .send(message.set_prefix(user.nickname.clone()))
                .map_err(|_| -> ServerError {
                    ServerError {
                        kind: CRITICAL.to_string(),
                        message: "Could not send to server".to_string(),
                    }
                })?;

            Ok(Some(NumericReply::new(
                RPL_INVITING_NUM,
                "",
                Some(vec![channel.name.clone(), nick_user_to_invite]),
            )))
        }

        None => Ok(Some(NumericReply::new(
            ERR_NOSUCHNICK_NUM,
            ERR_NOSUCHNICK_MSG,
            Some(vec![channel_name.to_string()]),
        ))),
    };
}

/********************************NAMES MESSAGE*************************************/

///
/// This function will receive a message with NAMES command and will
/// notify with RPL_NAMREPLY every user in the channel specified in message.
/// When all users where listed it will send RPL_ENDOFNAMES.
/// If no channel is specified it will list all users in all channels.
///
pub fn names(
    message: Message,
    mut stream: &TcpStream,
    channels: Arc<Mutex<HashMap<String, Channel>>>,
) -> Result<Option<NumericReply>, ServerError> {
    println!("Names!, with message: {:?}", message);
    let user_nickname = message.clone().prefix.expect("No prefix in message");
    let channels = channels.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not access channels".to_string(),
        }
    })?;
    if message.params_total_count() < 1 {
        for channel in channels.values() {
            let users = &channel.users;
            let users = users.keys().cloned().collect::<Vec<String>>();
            let message_to_send = users.join(" ");
            let answer = NumericReply::new(
                RPL_NAMEREPLY_NUM,
                &message_to_send,
                Some(vec![user_nickname.clone(), channel.name.clone()]),
            );
            stream
                .write_all(answer.as_string().as_bytes())
                .map_err(|_| -> ServerError {
                    ServerError {
                        kind: CRITICAL.to_string(),
                        message: "Could not write to stream".to_string(),
                    }
                })?;
        }
    } else {
        let channels_to_send = message.params[0].clone();
        for channel in channels_to_send {
            let channel = match channels.get(&channel) {
                Some(channel) => channel,
                None => continue,
            };
            let users = &channel.users;
            let users = users.keys().cloned().collect::<Vec<String>>();
            let message = users.join(" ");
            let answer = NumericReply::new(
                RPL_NAMEREPLY_NUM,
                &message,
                Some(vec![user_nickname.clone(), channel.name.clone()]),
            );
            stream
                .write_all(answer.as_string().as_bytes())
                .map_err(|_| -> ServerError {
                    ServerError {
                        kind: CRITICAL.to_string(),
                        message: "Could not write to stream".to_string(),
                    }
                })?;
        }
    }

    let answer = NumericReply::new(RPL_ENDOFNAMES_NUM, RPL_ENDOFNAMES_MSG, None);
    stream
        .write_all(answer.as_string().as_bytes())
        .map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Could not write to stream".to_string(),
            }
        })?;
    Ok(None)
}

/**********************************KICK MESSAGE***********************************/

///
/// Kics a user from the channel received in message.
///
/// Command: KICK
/// Parameters: <channel> <user> [<comment>]
///
/// ERR_NEEDMOREPARAMS: not enough params were given.
/// ERR_NOSUCHCHANNEL: non existing channels name provided.
/// ERR_CHANOPRIVSNEEDED: user trying to kick is not operator.
/// ERR_NOTONCHANNEL: user being kicked is not on channel.
/// ERR_NOSUCHNICK: user being kicked is not on channel.
///
pub fn kick(
    message: Message,
    user: &mut User,
    channels: Arc<Mutex<HashMap<String, Channel>>>,
    sender: &Sender<Message>,
) -> Result<Option<NumericReply>, ServerError> {
    if message.params_total_count() < 2 {
        return Ok(Some(NumericReply::new(
            ERR_NEEDMOREPARAMS_NUM,
            ERR_NEEDMOREPARAMS_MSG,
            None,
        )));
    }

    let mut channels = channels.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not access channels".to_string(),
        }
    })?;
    let channel_name = &message.params[0][0];
    let nickname_user_getting_kicked = &message.params[1][0];
    let nickname_user_kicking = &user.nickname;

    let channel = match channels.get_mut(channel_name) {
        Some(channel) => channel,
        None => {
            return Ok(Some(NumericReply::new(
                ERR_NOSUCHCHANNEL_NUM,
                ERR_NOSUCHCHANNEL_MSG,
                None,
            )))
        }
    };

    match channel.kick(nickname_user_getting_kicked, nickname_user_kicking) {
        Some(reply) => Ok(Some(reply)),
        None => {
            sender.send(message).map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Channel broken".to_string(),
                }
            })?;
            Ok(None)
        }
    }
}

/**********************************TOPIC MESSAGE***********************************/

///
///
/// The TOPIC message is used to change or view the topic of a channel.
/// The topic for channel <channel> is returned if there is no <topic>
/// given.  If the <topic> parameter is present, the topic for that
/// channel will be changed, if the channel modes permit this action.
///
/// Command: TOPIC
/// Parameters: <channel> [<topic>]
///
/// ERR_NEEDMOREPARAMS
/// ERR_NOTONCHANNEL
/// RPL_NOTOPIC
/// RPL_TOPIC
/// ERR_CHANOPRIVSNEEDED
/// ERR_NOSUCHCHANNEL.
///
pub fn topic(
    message: Message,
    channels: Arc<Mutex<HashMap<String, Channel>>>,
    user: &mut User,
    sender: &Sender<Message>,
) -> Result<Option<NumericReply>, ServerError> {
    if message.params_total_count() == 0 {
        return Ok(Some(NumericReply::new(
            ERR_NEEDMOREPARAMS_NUM,
            ERR_NEEDMOREPARAMS_MSG,
            None,
        )));
    }

    let nickname = user.nickname.clone();
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
            return Ok(Some(NumericReply::new(
                ERR_NOSUCHCHANNEL_NUM,
                ERR_NOSUCHCHANNEL_MSG,
                None,
            )))
        }
    };

    if message.params_total_count() == 1 {
        Ok(Some(channel.get_topic_reply()))
    } else {
        let topic = &message.params[1][0];
        match channel.set_topic(&nickname, topic) {
            Ok(reply) => {
                if channel.is_multiserver() {
                    sender.send(message).map_err(|_| -> ServerError {
                        ServerError {
                            kind: CRITICAL.to_string(),
                            message: "Broken channel connection".to_string(),
                        }
                    })?;
                };
                Ok(Some(reply))
            }
            Err(reply) => Ok(Some(reply)),
        }
    }
}

/**************************************TESTS**************************************/

#[cfg(test)]
mod tests {
    use crate::commands::{INVITE, JOIN};
    use crate::message::Message;
    use crate::numeric_reply::{
        NumericReply, ERR_NEEDMOREPARAMS_MSG, ERR_NEEDMOREPARAMS_NUM, ERR_NOSUCHNICK_MSG,
        ERR_NOSUCHNICK_NUM, RPL_INVITING_NUM,
    };
    use crate::server_utils::channel::Channel;
    use crate::server_utils::user::User;
    use std::collections::HashMap;
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::sync::{Arc, Mutex};

    use super::{invite_to_channel, join_channel};

    #[test]
    fn test_join_channel_creates_new_channel_correctly() {
        let (sender, _receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();
        let channels: Arc<Mutex<HashMap<String, Channel>>> = Arc::new(Mutex::new(HashMap::new()));
        let user = User::new(
            "test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );
        let mut users = HashMap::new();
        users.insert(user.nickname.clone(), user.clone());
        let users: Arc<Mutex<HashMap<String, User>>> = Arc::new(Mutex::new(users));
        let aux = TcpListener::bind("127.0.0.1:5000").unwrap();
        let _other = aux.incoming();
        let stream = TcpStream::connect("127.0.0.1:5000").unwrap();
        let message = Message {
            prefix: Some("test_user".to_string()),
            command: JOIN.to_string(),
            params: vec![vec!["#test_channel".to_string()]],
        };

        let reply = join_channel(&stream, message, &channels, &users, &user, &sender);
        assert!(reply.is_ok());
        assert!(reply.unwrap().is_none());
        assert!(channels.lock().unwrap().contains_key("#test_channel"));
    }

    #[test]
    fn test_join_channel_need_more_params() {
        let (sender, _receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();
        let channels: Arc<Mutex<HashMap<String, Channel>>> = Arc::new(Mutex::new(HashMap::new()));
        let user = User::new(
            "test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );
        let mut users = HashMap::new();
        users.insert(user.nickname.clone(), user.clone());
        let users: Arc<Mutex<HashMap<String, User>>> = Arc::new(Mutex::new(users));
        let aux = TcpListener::bind("127.0.0.1:5001").unwrap();
        let _other = aux.incoming();
        let stream = TcpStream::connect("127.0.0.1:5001").unwrap();
        let message = Message {
            prefix: Some("test_user".to_string()),
            command: JOIN.to_string(),
            params: vec![vec![]],
        };

        let reply = join_channel(&stream, message, &channels, &users, &user, &sender);
        assert!(reply.is_ok());
        assert_eq!(
            reply.unwrap().unwrap(),
            NumericReply::new(ERR_NEEDMOREPARAMS_NUM, ERR_NEEDMOREPARAMS_MSG, None)
        );
    }
    #[test]
    fn test_invite_new_user_need_params() {
        let (sender, _receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();

        let user = User::new(
            "test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );
        let other_user = User::new(
            "other_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );
        let channel = Channel::new("#test_channel".to_string(), &user);
        let mut channels = HashMap::new();
        channels.insert(channel.name.clone(), channel.clone());
        let channels: Arc<Mutex<HashMap<String, Channel>>> = Arc::new(Mutex::new(channels));
        let mut users = HashMap::new();
        users.insert(user.nickname.clone(), user.clone());
        users.insert(other_user.nickname.clone(), other_user.clone());
        let users: Arc<Mutex<HashMap<String, User>>> = Arc::new(Mutex::new(users));

        let message = Message {
            prefix: Some("test_user".to_string()),
            command: INVITE.to_string(),
            params: vec![vec![]],
        };
        let reply = invite_to_channel(&sender, message, channels, users, &user);
        assert!(reply.is_ok());
        assert_eq!(
            reply.unwrap().unwrap(),
            NumericReply::new(ERR_NEEDMOREPARAMS_NUM, ERR_NEEDMOREPARAMS_MSG, None)
        );
    }

    #[test]
    fn test_invite_new_user() {
        let (sender, _receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();

        let user = User::new(
            "test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );
        let other_user = User::new(
            "other_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );
        let channel = Channel::new("#test_channel".to_string(), &user);
        let mut channels = HashMap::new();
        channels.insert(channel.name.clone(), channel.clone());
        let channels: Arc<Mutex<HashMap<String, Channel>>> = Arc::new(Mutex::new(channels));
        let mut users = HashMap::new();
        users.insert(user.nickname.clone(), user.clone());
        users.insert(other_user.nickname.clone(), other_user.clone());
        let users: Arc<Mutex<HashMap<String, User>>> = Arc::new(Mutex::new(users));

        let message = Message {
            prefix: Some("test_user".to_string()),
            command: INVITE.to_string(),
            params: vec![
                vec!["other_user".to_string()],
                vec!["#test_channel".to_string()],
            ],
        };
        let reply = invite_to_channel(&sender, message, channels.clone(), users, &user);

        assert!(reply.is_ok());
        assert_eq!(
            reply.unwrap().unwrap(),
            NumericReply::new(
                RPL_INVITING_NUM,
                "",
                Some(vec!["#test_channel".to_string(), "other_user".to_string()])
            )
        );
        assert!(channels
            .lock()
            .unwrap()
            .get("#test_channel")
            .unwrap()
            .invites
            .contains(&"other_user".to_string()));
    }

    #[test]
    fn test_invite_non_existing_userl() {
        let (sender, _receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();

        let user = User::new(
            "test_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );
        let other_user = User::new(
            "other_user".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );
        let channel = Channel::new("#test_channel".to_string(), &user);
        let mut channels = HashMap::new();
        channels.insert(channel.name.clone(), channel.clone());
        let channels: Arc<Mutex<HashMap<String, Channel>>> = Arc::new(Mutex::new(channels));
        let mut users = HashMap::new();
        users.insert(user.nickname.clone(), user.clone());
        users.insert(other_user.nickname.clone(), other_user.clone());
        let users: Arc<Mutex<HashMap<String, User>>> = Arc::new(Mutex::new(users));

        let message = Message {
            prefix: Some("test_user".to_string()),
            command: INVITE.to_string(),
            params: vec![
                vec!["new_user".to_string()],
                vec!["#test_channel".to_string()],
            ],
        };
        let reply = invite_to_channel(&sender, message, channels, users, &user);

        assert!(reply.is_ok());
        assert_eq!(
            reply.unwrap().unwrap(),
            NumericReply::new(
                ERR_NOSUCHNICK_NUM,
                ERR_NOSUCHNICK_MSG,
                Some(vec!["new_user".to_string()])
            )
        );
    }

    #[test]
    //topic
    //kick
    //mode
    fn test() {}
}
