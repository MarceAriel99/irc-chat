//!
//! This module contains the common functions used in the integration tests
//!

use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
};

use irc::{
    numeric_reply::{NumericReply, RPL_CORRECTLOGIN_MSG, RPL_CORRECTLOGIN_NUM},
    server_utils::{server::Server, server_data::ServerData},
};

///
/// Runs a test server
///
pub fn run_server() -> Result<(), String> {
    // Read server data file of server to obtain ServerData
    let server_data = match ServerData::new("tests/common/server_data_test.txt".to_string()) {
        Ok(server_data) => server_data,
        Err(error) => {
            println!("aca toy");
            return Err(error.to_string());
        }
    };

    let server = match Server::new(server_data) {
        Ok(server) => server,
        Err(error) => {
            println!("Error: {}", error);
            return Err(error.message);
        }
    };

    // Run server
    match server.run() {
        Ok(_) => Ok(()),
        Err(error) => {
            if error.kind == "SQUIT" {
                Err(error.message)
            } else {
                Err(error.message)
            }
        }
    }
}

///
/// Logs in user with nickname ari in file users_test.txt
///
pub fn login_ari() -> TcpStream {
    let address = "127.0.0.1:3000";
    let mut socket = TcpStream::connect(address).unwrap();

    let action = "LOGIN\r\n";
    let result = socket.write_all(action.as_bytes());
    assert!(result.is_ok());

    let pass_message = "PASS password123\r\n";
    let result = socket.write_all(pass_message.as_bytes());
    assert!(result.is_ok());

    let nick_message = "NICK ari\r\n";
    let result = socket.write_all(nick_message.as_bytes());
    assert!(result.is_ok());

    let user_message = "USER arisalese,127.0.0.1,main_server :Ariana Salese\r\n";
    let result = socket.write_all(user_message.as_bytes());
    assert!(result.is_ok());

    let mut reader = BufReader::new(socket.try_clone().unwrap());

    let mut data = String::new();
    let result = reader.read_line(&mut data);
    assert!(result.is_ok());

    let reply = NumericReply::new(
        RPL_CORRECTLOGIN_NUM,
        RPL_CORRECTLOGIN_MSG,
        Some(vec!["ari".to_string()]),
    )
    .as_string();
    assert_eq!(reply, data);

    socket
}

///
/// Logs in user with nickname juanireil in file users_test.txt
///
pub fn login_juani() -> TcpStream {
    let address = "127.0.0.1:3000";
    let mut socket = TcpStream::connect(address).unwrap();

    let action = "LOGIN\r\n";
    let result = socket.write_all(action.as_bytes());
    assert!(result.is_ok());

    let pass_message = "PASS password123\r\n";
    let result = socket.write_all(pass_message.as_bytes());
    assert!(result.is_ok());

    let nick_message = "NICK juanireil\r\n";
    let result = socket.write_all(nick_message.as_bytes());
    assert!(result.is_ok());

    let user_message = "USER juanireil,127.0.0.1,main_server :Juani Reil\r\n";
    let result = socket.write_all(user_message.as_bytes());
    assert!(result.is_ok());

    let mut reader = BufReader::new(socket.try_clone().unwrap());

    let mut data = String::new();
    let result = reader.read_line(&mut data);
    assert!(result.is_ok());

    let reply = NumericReply::new(
        RPL_CORRECTLOGIN_NUM,
        RPL_CORRECTLOGIN_MSG,
        Some(vec!["juanireil".to_string()]),
    )
    .as_string();
    assert_eq!(reply, data);

    socket
}
