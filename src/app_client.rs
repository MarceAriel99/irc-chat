//!
//! This app initiates the client and its interface
//!

// These allow(dead_code) are because we need to use the mod and it imports everything.
// Not all constants are necessarily used in the different files.
mod client_utils;
#[allow(dead_code)]
mod commands;
#[allow(dead_code)]
mod custom_errors;
#[allow(dead_code)]
mod message;
#[allow(dead_code)]
mod numeric_reply;
mod parser;
use gtk::{
    prelude::{ApplicationExt, ApplicationExtManual},
    Application,
};

use crate::client_utils::client;

fn main() {
    let application = Application::new(None, Default::default());

    application.connect_activate(client::init_chat);
    application.run();
}
