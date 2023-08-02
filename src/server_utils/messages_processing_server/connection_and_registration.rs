//!
//! This module contains functions to handle messages beetwen servers regardingo connection and registration
//!

use crate::custom_errors::server_error::ServerError;
use crate::{
    custom_errors::errors::{CRITICAL, NONCRITICAL},
    message::Message,
    server_utils::user::User,
};
use std::{
    collections::HashMap,
    sync::{mpsc::Sender, Arc, Mutex, MutexGuard},
};

///
/// This function is called when a server receives  information of a new client that registered
///
pub fn handle_registration_server(
    message: Message,
    sender: &Sender<Message>,
) -> Result<(), ServerError> {
    println!("REG in handle registration server {:?}", message);

    sender.send(message).map_err(|_| -> ServerError {
        ServerError {
            kind: CRITICAL.to_string(),
            message: "Couldn't send".to_string(),
        }
    })?;

    Ok(())
}

///
/// This function is called when a server receives information about a new registered user
///
pub fn handle_users_info(
    message: Message,
    users: Arc<Mutex<HashMap<String, User>>>,
) -> Result<(), ServerError> {
    let users = users.lock().map_err(|_| -> ServerError {
        ServerError {
            kind: NONCRITICAL.to_string(),
            message: "Couldn't get lock".to_string(),
        }
    })?;
    let nickname = message
        .prefix
        .clone()
        .expect("No prefix in registration message");
    if !users.contains_key(&nickname) {
        add_new_user(message, users)?;
    }
    Ok(())
}

///
/// This function adds a new user that was received from another server
///
fn add_new_user(
    message: Message,
    mut users: MutexGuard<HashMap<String, User>>,
) -> Result<(), ServerError> {
    let user_data = message.params;
    //vec![vec![user.nickname.clone(), user.address.clone(), user.username.clone(), user.server_name.clone(), user.password.clone()], vec![user.real_name.clone()]];
    let user = User::new(
        user_data[0][0].clone(),
        user_data[0][1].clone(),
        user_data[0][2].clone(),
        user_data[1][0].clone(),
        user_data[0][3].clone(),
        user_data[0][4].clone(),
    );
    println!("New user saved {:?}", user);
    users.insert(user.nickname.clone(), user);
    Ok(())
}

/************************************TESTS*******************************************/

#[cfg(test)]
mod tests {
    use crate::{
        commands::USERS_INFO,
        message::Message,
        server_utils::messages_processing_server::connection_and_registration::{
            add_new_user, handle_users_info,
        },
    };
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    #[test]
    fn test_add_new_user() {
        let users = Arc::new(Mutex::new(HashMap::new()));

        let message = Message {
            prefix: Some("nickname".to_string()),
            command: USERS_INFO.to_string(),
            params: vec![
                vec![
                    "nickname".to_string(),
                    "hostname".to_string(),
                    "username".to_string(),
                    "servername".to_string(),
                    "contrasenia".to_string(),
                ],
                vec!["realname".to_string()],
            ],
        };
        let users = users.lock().unwrap();
        let reply = add_new_user(message, users);
        assert!(reply.is_ok());
    }
    #[test]
    fn test_handle_users_info() {
        let users = Arc::new(Mutex::new(HashMap::new()));

        let message = Message {
            prefix: Some("nickname".to_string()),
            command: USERS_INFO.to_string(),
            params: vec![
                vec![
                    "nickname".to_string(),
                    "hostname".to_string(),
                    "username".to_string(),
                    "servername".to_string(),
                    "contrasenia".to_string(),
                ],
                vec!["realname".to_string()],
            ],
        };
        let reply = handle_users_info(message, users.clone());
        assert!(reply.is_ok());
        let users = users.lock().unwrap();
        assert!(users.contains_key("nickname"));
        let user = users.get("nickname").unwrap();
        assert_eq!(user.nickname, "nickname");
        assert_eq!(user.address, "hostname");
        assert_eq!(user.username, "username");
        assert_eq!(user.real_name, "realname");
    }
}
