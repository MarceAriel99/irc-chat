use crate::commands::{NICK, PASS, REGISTRATION, USER};
use crate::custom_errors::client_error::ClientError;
use crate::custom_errors::errors::{CRITICAL, SEND_MESSAGE};
use crate::message::Message;
use gtk::{prelude::*, Builder};
use std::env;
use std::sync::mpsc::Sender;

#[derive(Clone)]
pub struct WindowRegister {
    pub window: gtk::ApplicationWindow,
    pub builder: gtk::Builder,
}

///
/// Initialize the register window
///
impl WindowRegister {
    pub fn new(application: gtk::Application) -> Self {
        let glade_src = include_str!("register.glade");
        let builder = Builder::new();
        builder
            .add_from_string(glade_src)
            .expect("Couldn't add from string");

        let window: gtk::ApplicationWindow = builder.object("window").expect("Couldn't get window");
        window.set_application(Some(&application));
        window.connect_delete_event(move |_, _| {
            println!("Finished application.");
            gtk::main_quit();
            Inhibit(false)
        });
        WindowRegister { window, builder }
    }

    // Calls for the setup of the registration window and shows it
    pub fn show(&self, tx_frontend: Sender<Message>, server_ip: String, server_name: String) {
        self.setup(tx_frontend, server_ip, server_name);
        self.window.show_all();
    }

    ///
    /// Setup login submenu and login button functionality
    /// When the button is clicked, it sends the cooresponding commands to the thread, for the client to send to the server
    ///
    pub fn setup(&self, tx_frontend: Sender<Message>, server_ip: String, server_name: String) {
        let register_button: gtk::Button = self
            .builder
            .object("register_button")
            .expect("Couldn't get register_button");
        let password_entry: gtk::Entry = self
            .builder
            .object("entry_password")
            .expect("Couldn't get entry_password");
        let nickname_entry: gtk::Entry = self
            .builder
            .object("entry_nickname")
            .expect("Couldn't get entry_nickname");
        let realname_entry: gtk::Entry = self
            .builder
            .object("entry_realname")
            .expect("Couldn't get entry_realname");
        let error_label: gtk::Label = self
            .builder
            .object("label_register_error")
            .expect("Couldn't get error_label");

        // Get the username from the environment
        let user_env = match env::var("USER") {
            Ok(val) => val,
            Err(_e) => "Anonymous".to_string(),
        };

        register_button.connect_clicked(move |_| {
            if password_entry.text().is_empty()
                || nickname_entry.text().is_empty()
                || realname_entry.text().is_empty()
            {
                error_label.set_text("Please fill in all values");
            } else {
                tx_frontend
                    .send(Message {
                        prefix: None,
                        command: REGISTRATION.to_string(),
                        params: vec![],
                    })
                    .map_err(|_| -> ClientError {
                        ClientError {
                            kind: CRITICAL.to_string(),
                            message: SEND_MESSAGE.to_string(),
                        }
                    })
                    .ok();
                tx_frontend
                    .send(Message {
                        prefix: None,
                        command: PASS.to_string(),
                        params: vec![vec![password_entry.text().to_string()]],
                    })
                    .map_err(|_| -> ClientError {
                        ClientError {
                            kind: CRITICAL.to_string(),
                            message: SEND_MESSAGE.to_string(),
                        }
                    })
                    .ok();
                tx_frontend
                    .send(Message {
                        prefix: None,
                        command: NICK.to_string(),
                        params: vec![vec![nickname_entry.text().to_string()]],
                    })
                    .map_err(|_| -> ClientError {
                        ClientError {
                            kind: CRITICAL.to_string(),
                            message: SEND_MESSAGE.to_string(),
                        }
                    })
                    .ok();
                tx_frontend
                    .send(Message {
                        prefix: None,
                        command: USER.to_string(),
                        params: vec![
                            vec![user_env.clone(), server_ip.clone(), server_name.clone()],
                            vec![realname_entry.text().to_string()],
                        ],
                    })
                    .map_err(|_| -> ClientError {
                        ClientError {
                            kind: CRITICAL.to_string(),
                            message: SEND_MESSAGE.to_string(),
                        }
                    })
                    .ok();
            }
        });
    }

    // Updates the register error label
    pub fn invalid_registration(&self, error: String) {
        let error_label: gtk::Label = self
            .builder
            .object("label_register_error")
            .expect("Couldn't get error_label");
        error_label.set_text(error.as_str());
    }

    // Hides the register window
    pub fn hide(&self) {
        self.window.hide();
    }
}
