//! USER INFORMATION
//!
//! This module contains functions to get user information
//!

use crate::custom_errors::server_error::ServerError;
use crate::{
    commands::IS_OPERATOR,
    custom_errors::errors::{CRITICAL, NONCRITICAL},
    message::Message,
    numeric_reply::{
        NumericReply, ERR_NONICKNAMEGIVEN_MSG, ERR_NONICKNAMEGIVEN_NUM, ERR_NOSUCHNICK_MSG,
        ERR_NOSUCHNICK_NUM, ERR_NOSUCHSERVER_MSG, ERR_NOSUCHSERVER_NUM, RPL_ENDOFWHOIS_MSG,
        RPL_ENDOFWHOIS_NUM, RPL_ENDOFWHO_MSG, RPL_ENDOFWHO_NUM, RPL_NOWAWAY_MSG, RPL_NOWAWAY_NUM,
        RPL_UNAWAY_MSG, RPL_UNAWAY_NUM, RPL_WHOISCHANNELS_MSG, RPL_WHOISCHANNELS_NUM,
        RPL_WHOISOPERATOR_MSG, RPL_WHOISOPERATOR_NUM, RPL_WHOISSERVER_MSG, RPL_WHOISSERVER_NUM,
        RPL_WHOISUSER_NUM, RPL_WHOREPLY_MSG, RPL_WHOREPLY_NUM,
    },
    server_utils::{channel::Channel, user::User},
};
use std::{
    collections::HashMap,
    io::Write,
    net::TcpStream,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
};
/*******************************WHO MESSAGE***********************************/

///
/// Handles WHO command, will check what it has to answer with
/// if not parameter is received in message then it returns all users
///
pub fn handle_who(
    message: Message,
    mut stream: &TcpStream,
    users: Arc<Mutex<HashMap<String, User>>>,
    receiver: &Receiver<Message>,
    sender: &Sender<Message>,
) -> Result<Option<NumericReply>, ServerError> {
    println!(
        "Handle Who!. Received message with params: {:?}",
        message.params
    );

    if message.params_total_count() == 1 && message.params[0][0] == "0"
        || message.params_total_count() == 0
    {
        let users_list = get_all_users(users)?;
        send_response(users_list, stream)?;
    } else if message.params[0].contains(&"o".to_string()) {
        sender.send(message).map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Could not send to server".to_string(),
            }
        })?;
        let msg = receiver.recv().map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Could not receive from server".to_string(),
            }
        })?;

        if msg.command != *"OPERATOR_NOT_FOUND" {
            let names = &msg.params[0];
            for user in names {
                let answer = NumericReply::new(
                    RPL_WHOREPLY_NUM,
                    RPL_WHOREPLY_MSG,
                    Some(vec![user.to_string()]),
                );
                stream
                    .write_all(answer.as_string().as_bytes())
                    .map_err(|_| -> ServerError {
                        ServerError {
                            kind: CRITICAL.to_string(),
                            message: "Could not send to server".to_string(),
                        }
                    })?;
            }
        }
    } else {
        let users_list = get_users_with(message.params[0][0].clone(), users)?;
        send_response(users_list, stream)?;
    }

    let end = NumericReply::new(RPL_ENDOFWHO_NUM, RPL_ENDOFWHO_MSG, None);
    stream
        .write_all(end.as_string().as_bytes())
        .map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Could not send to server".to_string(),
            }
        })?;

    Ok(None)
}

/*******************************WHOIS MESSAGE***********************************/

///
/// Possible replys
/// RPL_WHOISUSER       //datos del usuario
/// RPL_WHOISCHANNELS   // channels a los que esta conectado
/// RPL_WHOISSERVER     // datos del server al q esta conectado
/// RPL_WHOISOPERATOR   // si el user es operador           
/// RPL_ENDOFWHOIS    // fin de la respuesta
/// RPL_AWAY // returns the away message if the user is away
///
/// ERRORES:
/// ERR_NOSUCHNICK
/// ERR_NONICKNAMEGIVEN
///
pub fn whois(
    message: Message,
    mut stream: &TcpStream,
    users: Arc<Mutex<HashMap<String, User>>>,
    sender: &Sender<Message>,
    receiver: &Receiver<Message>,
    channels: Arc<Mutex<HashMap<String, Channel>>>,
) -> Result<Option<NumericReply>, ServerError> {
    if message.params_total_count() == 0 {
        return Ok(Some(NumericReply::new(
            ERR_NONICKNAMEGIVEN_NUM,
            ERR_NONICKNAMEGIVEN_MSG,
            None,
        )));
    }
    println!(
        "Handle Whois!. Received message with params: {:?}",
        message.params
    );
    let mut nick = message.params[0][0].clone();
    if message.params_total_count() == 2 {
        nick = message.params[1][0].clone();
    }
    let users = users.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not lock users".to_string(),
        }
    })?;

    let user = match users.get(&nick) {
        Some(user) => user,
        None => {
            return Ok(Some(NumericReply::new(
                ERR_NOSUCHNICK_NUM,
                ERR_NOSUCHNICK_MSG,
                Some(vec![nick]),
            )));
        }
    };
    sender.send(message.clone()).map_err(|_| -> ServerError {
        ServerError {
            kind: CRITICAL.to_string(),
            message: "Could not send to server".to_string(),
        }
    })?;
    let msg = receiver.recv().map_err(|_| -> ServerError {
        ServerError {
            kind: CRITICAL.to_string(),
            message: "Could not send to server".to_string(),
        }
    })?;
    if msg.params_total_count() > 0 && msg.params[0][0] == "Server not found" {
        return Ok(Some(NumericReply::new(
            ERR_NOSUCHSERVER_NUM,
            ERR_NOSUCHSERVER_MSG,
            Some(vec![message.params[0][0].clone()]),
        )));
    }

    let args = vec![
        user.nickname.clone(),
        user.username.clone(),
        user.address.clone(),
        "*".to_string(),
    ];
    let reply = NumericReply::new(RPL_WHOISUSER_NUM, &user.real_name.clone(), Some(args));
    stream
        .write_all(reply.as_string().as_bytes())
        .map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Could not send to server".to_string(),
            }
        })?;

    let args = vec![user.nickname.clone(), user.server_name.clone()];
    let reply = NumericReply::new(RPL_WHOISSERVER_NUM, RPL_WHOISSERVER_MSG, Some(args));
    stream
        .write_all(reply.as_string().as_bytes())
        .map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Could not send to server".to_string(),
            }
        })?;

    let mut request = message;
    request.command = IS_OPERATOR.to_string();
    sender.send(request).map_err(|_| -> ServerError {
        ServerError {
            kind: CRITICAL.to_string(),
            message: "Could not send to server".to_string(),
        }
    })?;
    let msg = receiver.recv().map_err(|_| -> ServerError {
        ServerError {
            kind: CRITICAL.to_string(),
            message: "Could not send to server".to_string(),
        }
    })?;

    if msg.params_total_count() > 0 && msg.params[0][0] == "You are an operator" {
        let args = vec![user.nickname.clone()];
        let reply = NumericReply::new(RPL_WHOISOPERATOR_NUM, RPL_WHOISOPERATOR_MSG, Some(args));
        stream
            .write_all(reply.as_string().as_bytes())
            .map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Could not send to server".to_string(),
                }
            })?;
    }
    let channels = channels.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not lock channels".to_string(),
        }
    })?;
    for channel in channels.values() {
        if channel.is_user_on_channel(&nick) {
            let args = vec![user.nickname.clone(), channel.name.clone()];
            let reply = NumericReply::new(RPL_WHOISCHANNELS_NUM, RPL_WHOISCHANNELS_MSG, Some(args));
            stream
                .write_all(reply.as_string().as_bytes())
                .map_err(|_| -> ServerError {
                    ServerError {
                        kind: CRITICAL.to_string(),
                        message: "Could not send to server".to_string(),
                    }
                })?;
        }
    }
    let args = vec![user.nickname.clone()];
    let reply = NumericReply::new(RPL_ENDOFWHOIS_NUM, RPL_ENDOFWHOIS_MSG, Some(args));
    stream
        .write_all(reply.as_string().as_bytes())
        .map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Could not send to server".to_string(),
            }
        })?;
    Ok(None)
}

/*******************************AUX FUNCTIONS***********************************/

///
/// Get a vector of all users
///
pub fn get_all_users(users: Arc<Mutex<HashMap<String, User>>>) -> Result<Vec<User>, ServerError> {
    let users = users.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not lock users".to_string(),
        }
    })?;
    let mut list_of_users = Vec::new();

    for user in users.values() {
        list_of_users.push(user.clone());
    }

    println!("In get_all_users, list of users: {:?}", list_of_users);
    Ok(list_of_users)
}

///
/// Get all users who has any parameter that mathces with name
///
pub fn get_users_with(
    name: String,
    users: Arc<Mutex<HashMap<String, User>>>,
) -> Result<Vec<User>, ServerError> {
    let users = users.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not lock users".to_string(),
        }
    })?;
    let mut list_of_users = Vec::new();

    for user in users.values() {
        if user.has_atribute_name(&name) {
            list_of_users.push(user.clone());
        }
    }

    Ok(list_of_users)
}

///
/// Sends the response to the client, it will send NumericReply::RPL_WHOREPLY first and then the users nick
///
pub fn send_response(users: Vec<User>, mut stream: &TcpStream) -> Result<(), ServerError> {
    for user in users {
        let answer = NumericReply::new(
            RPL_WHOREPLY_NUM,
            RPL_WHOREPLY_MSG,
            Some(vec![user.nickname.clone()]),
        );
        stream
            .write_all(answer.as_string().as_bytes())
            .map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Could not send to server".to_string(),
                }
            })?;
    }

    Ok(())
}

pub fn handle_away(
    message: Message,
    user: &mut User,
    users: Arc<Mutex<HashMap<String, User>>>,
    sender: Option<&Sender<Message>>,
) -> Result<Option<NumericReply>, ServerError> {
    if message.params_total_count() == 0 {
        let mut users = users.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Could not lock users".to_string(),
            }
        })?;
        let user_data = users.get_mut(&user.nickname).unwrap();
        user_data.away_message = None;
        user.away_message = None;
        let answer = NumericReply::new(RPL_UNAWAY_NUM, RPL_UNAWAY_MSG, None);
        Ok(Some(answer))
    } else {
        let mut users = users.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Could not lock users".to_string(),
            }
        })?;
        let user_data = users.get_mut(&user.nickname).unwrap();
        user_data.away_message = Some(message.params[0][0].clone());
        user.away_message = Some(message.params[0][0].clone());

        if let Some(sender_) = sender {
            sender_.send(message).map_err(|_| -> ServerError {
                ServerError {
                    kind: NONCRITICAL.to_string(),
                    message: "Could not send".to_string(),
                }
            })?;
        }

        let answer = NumericReply::new(RPL_NOWAWAY_NUM, RPL_NOWAWAY_MSG, None);
        Ok(Some(answer))
    }
}

/************************************TESTS*******************************************/

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        net::{TcpListener, TcpStream},
        sync::{
            mpsc::{self},
            Arc, Mutex,
        },
    };

    use crate::{
        commands::NICK,
        message::Message,
        numeric_reply::{
            NumericReply, ERR_NONICKNAMEGIVEN_MSG, ERR_NONICKNAMEGIVEN_NUM, ERR_NOSUCHNICK_MSG,
            ERR_NOSUCHNICK_NUM, ERR_NOSUCHSERVER_MSG, ERR_NOSUCHSERVER_NUM, RPL_NOWAWAY_MSG,
            RPL_NOWAWAY_NUM, RPL_UNAWAY_MSG, RPL_UNAWAY_NUM,
        },
        server_utils::user::User,
    };

    use super::{handle_away, whois};

    #[test]
    fn test_handle_away_no_message() {
        let nickname_expected = "test".to_string();
        let mut user = User::new(
            nickname_expected.clone(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let message = Message {
            prefix: None,
            command: NICK.to_string(),
            params: vec![vec![]],
        };
        let users = Arc::new(Mutex::new(HashMap::from([(
            nickname_expected.clone(),
            user.clone(),
        )])));

        let reply = handle_away(message, &mut user, users, None).unwrap();
        assert!(reply.is_some());
        assert_eq!(
            reply.unwrap(),
            NumericReply::new(RPL_UNAWAY_NUM, RPL_UNAWAY_MSG, None)
        );
        assert!(user.away_message.is_none());
    }

    #[test]
    fn test_handle_away_message() {
        let nickname_expected = "test".to_string();
        let mut user = User::new(
            nickname_expected.clone(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let message = Message {
            prefix: None,
            command: NICK.to_string(),
            params: vec![vec!["I went to sleep".to_string()]],
        };
        let users = Arc::new(Mutex::new(HashMap::from([(
            nickname_expected.clone(),
            user.clone(),
        )])));

        let reply = handle_away(message, &mut user, users, None).unwrap();
        assert!(reply.is_some());
        assert_eq!(
            reply.unwrap(),
            NumericReply::new(RPL_NOWAWAY_NUM, RPL_NOWAWAY_MSG, None)
        );
        assert!(user.away_message.is_some());
        assert_eq!(user.away_message.unwrap(), "I went to sleep".to_string());
    }

    #[test]
    fn test_wohis_need_more_params() {
        let nickname_expected = "test".to_string();
        let user = User::new(
            nickname_expected.clone(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let message = Message {
            prefix: None,
            command: NICK.to_string(),
            params: vec![vec![]],
        };
        let users = Arc::new(Mutex::new(HashMap::from([(
            nickname_expected.clone(),
            user.clone(),
        )])));
        let (sender, receiver) = mpsc::channel();

        let channels = Arc::new(Mutex::new(HashMap::new()));
        let aux = TcpListener::bind("127.0.0.1:4001").unwrap();
        let _other = aux.incoming();
        let stream = TcpStream::connect("127.0.0.1:4001").unwrap();

        let reply = whois(message, &stream, users, &sender, &receiver, channels).unwrap();
        assert!(reply.is_some());
        assert_eq!(
            reply.unwrap(),
            NumericReply::new(ERR_NONICKNAMEGIVEN_NUM, ERR_NONICKNAMEGIVEN_MSG, None)
        );
    }

    #[test]
    fn test_wohis_no_such_nick() {
        let nickname_expected = "test".to_string();
        let user = User::new(
            nickname_expected.clone(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let message = Message {
            prefix: None,
            command: NICK.to_string(),
            params: vec![vec!["hola".to_string()]],
        };
        let users = Arc::new(Mutex::new(HashMap::from([(
            nickname_expected.clone(),
            user.clone(),
        )])));
        let (sender, receiver) = mpsc::channel();

        let channels = Arc::new(Mutex::new(HashMap::new()));
        let aux = TcpListener::bind("127.0.0.1:4000").unwrap();
        let _other = aux.incoming();
        let stream = TcpStream::connect("127.0.0.1:4000").unwrap();

        let reply = whois(message, &stream, users, &sender, &receiver, channels).unwrap();
        assert!(reply.is_some());
        assert_eq!(
            reply.unwrap(),
            NumericReply::new(
                ERR_NOSUCHNICK_NUM,
                ERR_NOSUCHNICK_MSG,
                Some(vec!["hola".to_string()])
            )
        );
    }

    #[test]
    fn test_wohis_no_such_server() {
        let nickname_expected = "test".to_string();
        let user = User::new(
            nickname_expected.clone(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let message = Message {
            prefix: None,
            command: NICK.to_string(),
            params: vec![vec!["server".to_string()], vec![nickname_expected.clone()]],
        };
        let users = Arc::new(Mutex::new(HashMap::from([(
            nickname_expected.clone(),
            user.clone(),
        )])));
        let (sender, receiver) = mpsc::channel();

        let channels = Arc::new(Mutex::new(HashMap::new()));
        let aux = TcpListener::bind("127.0.0.1:4003").unwrap();
        let _other = aux.incoming();
        let stream = TcpStream::connect("127.0.0.1:4003").unwrap();
        let mut answer = message.clone();
        answer.params = vec![vec!["Server not found".to_string()]];
        sender.send(answer).unwrap();
        let reply = whois(message, &stream, users, &sender, &receiver, channels).unwrap();
        assert!(reply.is_some());
        assert_eq!(
            reply.unwrap(),
            NumericReply::new(
                ERR_NOSUCHSERVER_NUM,
                ERR_NOSUCHSERVER_MSG,
                Some(vec!["server".to_string()])
            )
        );
    }

    #[test]
    fn test_wohis() {
        let nickname_expected = "test".to_string();
        let user = User::new(
            nickname_expected.clone(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let message = Message {
            prefix: None,
            command: NICK.to_string(),
            params: vec![vec![nickname_expected.clone()]],
        };
        let users = Arc::new(Mutex::new(HashMap::from([(
            nickname_expected.clone(),
            user.clone(),
        )])));
        let (sender, receiver) = mpsc::channel();

        let channels = Arc::new(Mutex::new(HashMap::new()));
        let aux = TcpListener::bind("127.0.0.1:4002").unwrap();
        let _other = aux.incoming();
        let stream = TcpStream::connect("127.0.0.1:4002").unwrap();
        let answer = message.clone();
        sender.send(answer.clone()).unwrap();
        sender.send(answer).unwrap();
        let reply = whois(message, &stream, users, &sender, &receiver, channels).unwrap();
        assert!(reply.is_none());
    }
}
