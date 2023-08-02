//! CONNECTION AND REGISTRATION
//!
//! This module contains functions to handle the connection, registration
//! and user information of clients
//!

use std::{
    collections::HashMap,
    net::TcpStream,
    result::Result,
    string::String,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
};

use crate::{
    commands::{LOGIN, NICK, PASS, REGISTRATION, SERVER, USER},
    custom_errors::{
        errors::{CRITICAL, NONCRITICAL},
        server_error::ServerError,
    },
    message::Message,
    numeric_reply::{
        NumericReply, ERR_ERRONEUSNICKNAME_MSG, ERR_ERRONEUSNICKNAME_NUM, ERR_INVALIDLOGIN_MSG,
        ERR_INVALIDLOGIN_NUM, ERR_NEEDMOREPARAMS_MSG, ERR_NEEDMOREPARAMS_NUM,
        ERR_NICKCOLLISION_MSG, ERR_NICKCOLLISION_NUM, ERR_NICKNAMEINUSE_MSG, ERR_NICKNAMEINUSE_NUM,
        ERR_NONICKNAMEGIVEN_MSG, ERR_NONICKNAMEGIVEN_NUM, ERR_PASSWDMISMATCH_MSG,
        ERR_PASSWDMISMATCH_NUM, RPL_CORRECTLOGIN_MSG, RPL_CORRECTLOGIN_NUM,
        RPL_CORRECTREGISTRATION_MSG, RPL_CORRECTREGISTRATION_NUM, RPL_YOUREOPER_MSG,
        RPL_YOUREOPER_NUM,
    },
    server_utils::user::User,
};

/********************************PASS MESSAGE*************************************/

///
/// Receives message with pass command and returns password. Could return the following
/// numeric replies:
///
/// ERR_NEEDMOREPARAMS: password was not suplied.
///
pub fn get_password(message: &Message) -> Result<String, NumericReply> {
    if message.command != *PASS {
        panic!("password needed");
    }

    // if no password was given then ERR_NEEDMOREPARAMS
    if message.params_total_count() == 0 {
        return Err(NumericReply::new(
            ERR_NEEDMOREPARAMS_NUM,
            ERR_NEEDMOREPARAMS_MSG,
            None,
        ));
    }

    Ok(message.params[0][0].to_string())
}

/********************************NICK MESSAGE*************************************/

///
/// Receives message with nick command and returns nickname. Could return
/// the following numeric replies:
///
/// ERR_NONICKNAMEGIVEN: nickname was not suplied.
/// ERR_ERRONEUSNICKNAME: nickname contains invalid characters.
/// ERR_NICKCOLLISION: registered of a NICK that already exists by another server.
///
pub fn get_nickname(
    message: &Message,
    action: &String,
    users: &Arc<Mutex<HashMap<String, User>>>,
) -> Result<Result<String, NumericReply>, ServerError> {
    if message.command != *NICK {
        panic!("nickname needed");
    }

    // If there are no params then ERR_NONICKNAMEGIVEN
    if message.params_total_count() == 0 {
        return Ok(Err(NumericReply::new(
            ERR_NONICKNAMEGIVEN_NUM,
            ERR_NONICKNAMEGIVEN_MSG,
            None,
        )));
    }

    // Get nickname
    let nickname = message.params[0][0].to_string();

    if action == REGISTRATION {
        let result = check_registration_nick(&nickname, users)?;

        if let Some(reply) = result {
            return Ok(Err(reply));
        }
    }

    Ok(Ok(nickname))
}

///
/// Receives message with nick command to register a user. Could return the following
/// numeric reply:
///
/// ERR_NICKCOLLISION: registered of a NICK that already exists by another server.
/// ERR_ERRONEUSNICKNAME: nickname contains invalid characters.
///  
pub fn check_registration_nick(
    nickname: &String,
    users: &Arc<Mutex<HashMap<String, User>>>,
) -> Result<Option<NumericReply>, ServerError> {
    // Check if nickname is valid
    if !nickname_is_valid(nickname) {
        return Ok(Some(NumericReply::new(
            ERR_ERRONEUSNICKNAME_NUM,
            ERR_ERRONEUSNICKNAME_MSG,
            Some(vec![nickname.to_string()]),
        )));
    }

    // Check if nickname is in use, if it is then ERR_NICKCOLLISION
    if check_nickname_collision(nickname, users)? {
        return Ok(Some(NumericReply::new(
            ERR_NICKCOLLISION_NUM,
            ERR_NICKCOLLISION_MSG,
            None,
        )));
    }

    Ok(None)
}

///
/// Receives message with nick command to change nickname of user.Could return the
/// following numeric reply:
///
/// ERR_NICKNAMEINUSE: attempt to change to a currently existing nickname
///
pub fn change_nick(
    message: Message,
    users: &Arc<Mutex<HashMap<String, User>>>,
    user: &mut User,
) -> Result<Option<NumericReply>, ServerError> {
    if message.params_total_count() == 0 {
        return Ok(Some(NumericReply::new(
            ERR_NONICKNAMEGIVEN_NUM,
            ERR_NONICKNAMEGIVEN_MSG,
            None,
        )));
    }

    let new_nickname = message.params[0][0].clone();

    if check_nickname_collision(&new_nickname, users)? {
        return Ok(Some(NumericReply::new(
            ERR_NICKNAMEINUSE_NUM,
            ERR_NICKNAMEINUSE_MSG,
            None,
        )));
    }

    let mut users = users.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Couldn't get lock".to_string(),
        }
    })?;
    let user_to_modify = match users.get_mut(&new_nickname) {
        Some(user) => user,
        None => return Ok(None),
    };
    user_to_modify.nickname = new_nickname.clone();
    user.nickname = new_nickname;

    Ok(None)
}

/********************************USER MESSAGE*************************************/

///
/// Receives message with user command to register a user. Could return the
/// following numeric reply:
///
/// ERR_NEEDMOREPARAMS: not enough parameter where suplied.
///
/// message parameters: <username> <hostname> <servername> <realname>
///
pub fn get_user_info(message: &Message) -> Result<(String, String, String, String), NumericReply> {
    if message.command != *USER {
        panic!("user information needed");
    }

    // if less than 4 params are given then ERR_NEEDMOREPARAMS
    if message.params_total_count() < 4 {
        return Err(NumericReply::new(
            ERR_NEEDMOREPARAMS_NUM,
            ERR_NEEDMOREPARAMS_MSG,
            None,
        ));
    }

    Ok((
        message.params[0][0].to_string(),
        message.params[0][1].to_string(),
        message.params[0][2].to_string(),
        message.params[1][0].to_string(),
    ))
}

/***************************LOGIN AND REGISTRATION*********************************/

///
/// Returns LOGIN or REGISTRATION command from message
///
pub fn get_action(message: &Message) -> Result<String, ServerError> {
    println!("{:?}", message);
    if message.command != LOGIN && message.command != REGISTRATION && message.command != SERVER {
        return Err(ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Invalid command".to_string(),
        });
    }

    Ok(message.command.clone())
}

///
/// Verifies that user is already registrates with nickname and password of user provided.
/// Could return the following numeri reply:
///
/// ERR_INVALIDLOGIN: user information was incorrect
/// RPL_CORRECTLOGIN
///
pub fn login_user(
    users: &Arc<Mutex<HashMap<String, User>>>,
    user: User,
    server_name: &String,
) -> Result<Result<NumericReply, NumericReply>, ServerError> {
    let nickname = user.nickname.clone();
    let password = user.password;

    if !check_login(&nickname, &password, users, server_name)? {
        println!("Login incorrect");
        return Ok(Err(NumericReply::new(
            ERR_INVALIDLOGIN_NUM,
            ERR_INVALIDLOGIN_MSG,
            Some(vec![nickname, password]),
        )));
    };

    println!("Login correct");
    Ok(Ok(NumericReply::new(
        RPL_CORRECTLOGIN_NUM,
        RPL_CORRECTLOGIN_MSG,
        Some(vec![nickname]),
    )))
}

///
/// Registrates user. Saves the user information in the server data file.
/// Returns the following numeric reply:
///
/// RPL_CORRECTREGISTRATION: user was registrated succesfully.
///
pub fn registrate_user(
    users: &Arc<Mutex<HashMap<String, User>>>,
    new_user: User,
) -> Result<NumericReply, ServerError> {
    let nickname = new_user.nickname.clone();

    // Save the new user
    let mut users = users.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not lock users".to_string(),
        }
    })?;

    users.insert(nickname.clone(), new_user);

    println!("Registration correct");
    Ok(NumericReply::new(
        RPL_CORRECTREGISTRATION_NUM,
        RPL_CORRECTREGISTRATION_MSG,
        Some(vec![nickname]),
    ))
}

///
/// Receives message with QUIT command. The stream provided get shutdown and the server is notified
/// to disconect client.
///
pub fn quit(
    message: Message,
    stream: &TcpStream,
    sender: &Sender<Message>,
    user: &mut User,
) -> Result<Option<NumericReply>, ServerError> {
    print!("In QUIT");
    stream
        .shutdown(std::net::Shutdown::Both)
        .expect("shutdown call failed");

    sender
        .send(message.set_prefix(user.nickname.clone()))
        .map_err(|_| -> ServerError {
            ServerError {
                kind: CRITICAL.to_string(),
                message: "Could not send to server".to_string(),
            }
        })?;
    Ok(None)
}

/********************************AUX FUNCTIONS*************************************/

///
/// Checks if there's nickname collision (another user with the nickname provided)
///
fn check_nickname_collision(
    nickname: &String,
    users: &Arc<Mutex<HashMap<String, User>>>,
) -> Result<bool, ServerError> {
    let users = users.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not lock users".to_string(),
        }
    })?;

    Ok(users.get(nickname).is_some())
}

///
/// Checks if there's a user in users with the password and nickname of the user suplied
///
fn check_login(
    nickname: &String,
    password: &String,
    users: &Arc<Mutex<HashMap<String, User>>>,
    server_name: &String,
) -> Result<bool, ServerError> {
    let users = users.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Could not lock users".to_string(),
        }
    })?;
    println!("In check login, with servername: {}", server_name);

    // check if user and password are correct
    match users.get(nickname) {
        Some(user) => {
            if user.server_name != *server_name {
                println!("Server names do not match");
                return Ok(false);
            };
            if user.password == *password {
                return Ok(true);
            };
            Ok(false)
        }
        None => Ok(false),
    }
}

///
/// Checks that nickname doesnt contains invalid characters and
/// has less than 9 characters
///
fn nickname_is_valid(nickname: &String) -> bool {
    if nickname.len() > 9 {
        return false;
    }

    !(nickname.starts_with('#') || nickname.starts_with('&') || nickname.starts_with(':'))
}

///
/// Sets user as operator
///
pub fn set_operator(
    message: Message,
    sender: &Sender<Message>,
    receiver: &Receiver<Message>,
) -> Result<Option<NumericReply>, ServerError> {
    if message.params_total_count() < 2 {
        return Ok(Some(NumericReply::new(
            ERR_NEEDMOREPARAMS_NUM,
            ERR_NEEDMOREPARAMS_MSG,
            None,
        )));
    }
    println!("Set operator");
    sender.send(message).map_err(|_| -> ServerError {
        ServerError {
            kind: CRITICAL.to_string(),
            message: "Could not send to server".to_string(),
        }
    })?;
    let answer = receiver.recv().map_err(|_| -> ServerError {
        ServerError {
            kind: CRITICAL.to_string(),
            message: "Could not receive from server".to_string(),
        }
    })?;
    if answer.params[0][0] == "Wrong password" {
        return Ok(Some(NumericReply::new(
            ERR_PASSWDMISMATCH_NUM,
            ERR_PASSWDMISMATCH_MSG,
            None,
        )));
    }

    Ok(Some(NumericReply::new(
        RPL_YOUREOPER_NUM,
        RPL_YOUREOPER_MSG,
        None,
    )))
}

/************************************TESTS*******************************************/

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{mpsc, Arc, Mutex};

    use crate::commands::{LOGIN, NICK, OPER, PASS, REGISTRATION, USER};
    use crate::message::Message;
    use crate::numeric_reply::{
        NumericReply, ERR_ERRONEUSNICKNAME_MSG, ERR_ERRONEUSNICKNAME_NUM, ERR_INVALIDLOGIN_MSG,
        ERR_INVALIDLOGIN_NUM, ERR_NEEDMOREPARAMS_MSG, ERR_NEEDMOREPARAMS_NUM,
        ERR_NICKCOLLISION_MSG, ERR_NICKCOLLISION_NUM, ERR_NONICKNAMEGIVEN_MSG,
        ERR_NONICKNAMEGIVEN_NUM, ERR_PASSWDMISMATCH_MSG, ERR_PASSWDMISMATCH_NUM,
        RPL_CORRECTLOGIN_MSG, RPL_CORRECTLOGIN_NUM, RPL_YOUREOPER_MSG, RPL_YOUREOPER_NUM,
    };
    use crate::server_utils::messages_processing_client::connection_and_registration::{
        check_registration_nick, get_nickname, get_password, get_user_info, set_operator,
    };
    use crate::server_utils::user::User;

    use super::login_user;

    // Tests pass message

    #[test]
    fn get_password_returns_correct_password() {
        let password_expected = "password".to_string();
        let message = Message {
            prefix: None,
            command: PASS.to_string(),
            params: vec![vec![password_expected.clone()]],
        };
        let password = get_password(&message);

        assert!(password.is_ok());

        let password = password.unwrap();

        assert_eq!(password, password_expected)
    }

    #[test]
    fn get_password_returns_whitout_password_return_corerct_numeric_reply() {
        let message = Message {
            prefix: None,
            command: PASS.to_string(),
            params: vec![],
        };
        let reply = get_password(&message);

        assert!(reply.is_err());

        let reply = reply.err().unwrap();

        assert_eq!(
            reply,
            NumericReply::new(ERR_NEEDMOREPARAMS_NUM, ERR_NEEDMOREPARAMS_MSG, None)
        )
    }

    // Tests nickname message

    #[test]
    fn get_nickname_while_registrating_returns_correct_nickname() {
        let nickname_expected = "nickname".to_string();
        let message = Message {
            prefix: None,
            command: NICK.to_string(),
            params: vec![vec![nickname_expected.clone()]],
        };
        let users = Arc::new(Mutex::new(HashMap::new()));
        let nickname = get_nickname(&message, &REGISTRATION.to_string(), &users);

        assert!(nickname.is_ok());

        let nickname = nickname.unwrap().unwrap();

        assert_eq!(nickname, nickname_expected)
    }

    #[test]
    fn get_nickname_while_logingin_returns_correct_nickname() {
        let nickname_expected = "nickname".to_string();

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
            user,
        )])));
        let nickname = get_nickname(&message, &LOGIN.to_string(), &users);

        assert!(nickname.is_ok());

        let nickname = nickname.unwrap().unwrap();

        assert_eq!(nickname, nickname_expected)
    }

    #[test]
    fn get_nickname_without_nickname_correct_numeric_reply() {
        let message = Message {
            prefix: None,
            command: NICK.to_string(),
            params: vec![],
        };
        let users = Arc::new(Mutex::new(HashMap::new()));
        let reply = get_nickname(&message, &LOGIN.to_string(), &users);

        assert!(!reply.is_err());
        let reply = reply.unwrap().err().unwrap();

        assert_eq!(
            reply,
            NumericReply::new(ERR_NONICKNAMEGIVEN_NUM, ERR_NONICKNAMEGIVEN_MSG, None)
        )
    }

    #[test]
    fn get_nickname_with_invalid_character_returns_correct_numeric_reply() {
        let nickname_expected = "#nickname".to_string();

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
            user,
        )])));
        let reply = get_nickname(&message, &REGISTRATION.to_string(), &users);

        assert!(!reply.is_err());

        let reply = reply.unwrap().err().unwrap();

        assert_eq!(
            reply,
            NumericReply::new(
                ERR_ERRONEUSNICKNAME_NUM,
                ERR_ERRONEUSNICKNAME_MSG,
                Some(vec![nickname_expected.clone()])
            )
        )
    }

    #[test]
    fn get_nickname_with_collision_returns_correct_numeric_reply() {
        let nickname_expected = "nickname".to_string();

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
            user,
        )])));
        let reply = get_nickname(&message, &REGISTRATION.to_string(), &users);
        assert!(!reply.is_err());

        let reply = reply.unwrap().err().unwrap();

        assert_eq!(
            reply,
            NumericReply::new(ERR_NICKCOLLISION_NUM, ERR_NICKCOLLISION_MSG, None)
        )
    }

    // Tests user message

    #[test]
    fn get_user_info_returns_correct_user_info() {
        //message parameters: <username> <hostname> <servername> <realname>
        let params = vec![
            vec![
                "address".to_string(),
                "username".to_string(),
                "server_name".to_string(),
            ],
            vec!["real_name".to_string()],
        ];
        let message = Message {
            prefix: None,
            command: USER.to_string(),
            params,
        };
        let user_info = get_user_info(&message);

        assert!(user_info.is_ok());

        let user_info = user_info.unwrap();

        assert_eq!(user_info.0, "address".to_string());
        assert_eq!(user_info.1, "username".to_string());
        assert_eq!(user_info.2, "server_name".to_string());
        assert_eq!(user_info.3, "real_name".to_string());
    }

    #[test]
    fn get_user_info_without_every_param_returns_correct_numeric_reply() {
        //message parameters: <username> <hostname> <servername> <realname>
        let params = vec![
            vec!["username".to_string(), "server_name".to_string()],
            vec!["real_name".to_string()],
        ];
        let message = Message {
            prefix: None,
            command: USER.to_string(),
            params,
        };
        let reply = get_user_info(&message);

        assert!(reply.is_err());

        let reply = reply.err().unwrap();

        assert_eq!(
            reply,
            NumericReply::new(ERR_NEEDMOREPARAMS_NUM, ERR_NEEDMOREPARAMS_MSG, None)
        );
    }

    // Test login user

    #[test]
    fn login_user_with_correct_information_return_correct_numeric_reply() {
        let nickname_expected = "nickname".to_string();

        let user = User::new(
            nickname_expected.clone(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let users = Arc::new(Mutex::new(HashMap::from([(
            nickname_expected.clone(),
            user.clone(),
        )])));

        let reply = login_user(&users, user, &"test".to_string())
            .unwrap()
            .unwrap();

        assert_eq!(
            reply,
            NumericReply::new(
                RPL_CORRECTLOGIN_NUM,
                RPL_CORRECTLOGIN_MSG,
                Some(vec![nickname_expected])
            )
        )
    }

    #[test]
    fn login_user_with_incorrect_information_return_correct_numeric_reply() {
        let nickname_expected = "nickname".to_string();

        let user_expected = User::new(
            nickname_expected.clone(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let user = User::new(
            nickname_expected.clone(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "incorrectpassword".to_string(),
        );

        let users = Arc::new(Mutex::new(HashMap::from([(
            nickname_expected.clone(),
            user_expected,
        )])));

        let reply = login_user(&users, user, &"test".to_string());
        let reply = reply.unwrap().err().unwrap();
        assert_eq!(
            reply,
            NumericReply::new(
                ERR_INVALIDLOGIN_NUM,
                ERR_INVALIDLOGIN_MSG,
                Some(vec![nickname_expected, "incorrectpassword".to_string()])
            )
        )
    }

    // test
    /// set_operator
    ///
    #[test]
    fn test_check_registration_nick_already_in_use() {
        let nickname_expected = "nickname".to_string();

        let user_expected = User::new(
            nickname_expected.clone(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let users = Arc::new(Mutex::new(HashMap::from([(
            nickname_expected.clone(),
            user_expected,
        )])));

        let reply = check_registration_nick(&nickname_expected, &users).unwrap();
        assert_eq!(
            reply.unwrap(),
            NumericReply::new(ERR_NICKCOLLISION_NUM, ERR_NICKCOLLISION_MSG, None)
        );
    }
    #[test]
    fn test_check_registration_nick_invalid() {
        let nickname_expected = "nickname".to_string();

        let user_expected = User::new(
            nickname_expected.clone(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        let users = Arc::new(Mutex::new(HashMap::from([(
            nickname_expected.clone(),
            user_expected,
        )])));

        let reply = check_registration_nick(&"#juani".to_string(), &users).unwrap();
        assert_eq!(
            reply.unwrap(),
            NumericReply::new(
                ERR_ERRONEUSNICKNAME_NUM,
                ERR_ERRONEUSNICKNAME_MSG,
                Some(vec!["#juani".to_string()])
            )
        );
    }
    #[test]
    fn test_set_operator_need_more_params() {
        let message = Message {
            prefix: None,
            command: OPER.to_string(),
            params: vec![vec![]],
        };
        let (sender1, _receiver1) = mpsc::channel();
        let (_sender2, receiver2) = mpsc::channel();
        let reply = set_operator(message, &sender1, &receiver2).unwrap();
        assert_eq!(
            reply.unwrap(),
            NumericReply::new(ERR_NEEDMOREPARAMS_NUM, ERR_NEEDMOREPARAMS_MSG, None)
        );
    }

    #[test]
    fn test_set_operator_wrong_password() {
        let nickname_expected = "nickname".to_string();

        let message = Message {
            prefix: None,
            command: OPER.to_string(),
            params: vec![
                vec![nickname_expected.clone()],
                vec!["wrong_password".to_string()],
            ],
        };
        let (sender1, _receiver1) = mpsc::channel();
        let (sender2, receiver2) = mpsc::channel();
        sender2.send(message.clone()).unwrap();
        let reply = set_operator(message, &sender1, &receiver2).unwrap();
        assert_eq!(
            reply.unwrap(),
            NumericReply::new(RPL_YOUREOPER_NUM, RPL_YOUREOPER_MSG, None)
        );
    }
    #[test]
    fn test_set_operator_correctly() {
        let nickname_expected = "nickname".to_string();

        let message = Message {
            prefix: None,
            command: OPER.to_string(),
            params: vec![
                vec![nickname_expected.clone()],
                vec!["password".to_string()],
            ],
        };
        let (sender1, _receiver1) = mpsc::channel();
        let (sender2, receiver2) = mpsc::channel();
        let mut answer = message.clone();
        answer.params = vec![vec!["Wrong password".to_string()]];
        sender2.send(answer).unwrap();
        let reply = set_operator(message, &sender1, &receiver2).unwrap();
        assert_eq!(
            reply.unwrap(),
            NumericReply::new(ERR_PASSWDMISMATCH_NUM, ERR_PASSWDMISMATCH_MSG, None)
        );
    }
}
