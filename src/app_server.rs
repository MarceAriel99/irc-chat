//!
//! This app initiates the server with its info and the users saved
//!

/// These allow(dead_code) are because we need to use the mod and it imports everything.
/// Not all constants are necessarily used in the different files.
///
#[allow(dead_code)]
mod commands;
#[allow(dead_code)]
mod custom_errors;
#[allow(dead_code)]
mod message;
#[allow(dead_code)]
mod numeric_reply;
mod parser;
mod server_utils;
use std::{env, result::Result, string::String};

use crate::server_utils::server::Server;
use crate::server_utils::server_data::ServerData;

fn main() -> Result<(), String> {
    // Collect arguments and check if config file of server is provided
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return Err("No config file provided".to_string());
    }

    let file_name = &args[1];
    println!("initializing server with config file: {}", file_name);

    let mut server_data_file_path = "saved_files/".to_string();
    server_data_file_path.push_str(file_name);

    // Read server data file of server to obtain ServerData
    let server_data = match ServerData::new(server_data_file_path.clone()) {
        Ok(server_data) => server_data,
        Err(error) => return Err(error.to_string()),
    };

    let server_name = server_data.server_name.clone();
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
                println!("Server {} has been shut down with message", server_name);
                Err(error.message)
            } else {
                Err(error.message)
            }
        }
    }
}
