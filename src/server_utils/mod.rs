//!
//! This module contains all the functions necessary for the server to work and it's structs    
//!

pub mod channel;
pub mod client_handler;
pub mod connection_handler;
#[allow(clippy::type_complexity)]
pub mod connection_listener;
#[allow(clippy::type_complexity)]
pub mod main_server;
pub mod messages_processing_client;
pub mod messages_processing_server;
pub mod secondary_server;
#[allow(clippy::type_complexity)]
pub mod server;
pub mod server_data;
pub mod server_rol;
pub mod user;
