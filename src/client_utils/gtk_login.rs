use crate::commands::{LOGIN, NICK, PASS, USER};
use crate::custom_errors::client_error::ClientError;
use crate::custom_errors::errors::{CRITICAL, SEND_MESSAGE};
use crate::message::Message;
use gtk::{prelude::*, Builder};
use std::env;
use std::sync::mpsc::Sender;

#[path = "gtk_register.rs"]
mod gtk_register;

#[derive(Clone)]
pub struct WindowLogin {
    pub window: gtk::ApplicationWindow,
    pub builder: gtk::Builder,
    pub window_register: gtk_register::WindowRegister,
}

///
/// Initialize the login window
///
impl WindowLogin {
    pub fn new(application: gtk::Application) -> Self {
        let glade_src = include_str!("login.glade");
        let builder = Builder::new();
        builder
            .add_from_string(glade_src)
            .expect("Couldn't add from string");
        let window: gtk::ApplicationWindow = builder.object("window").expect("Couldn't get window");
        window.set_application(Some(&application));

        let window_register = gtk_register::WindowRegister::new(application);
        window.connect_delete_event(move |_, _| {
            println!("Finished application.");
            gtk::main_quit();
            Inhibit(false)
        });
        WindowLogin {
            window,
            builder,
            window_register,
        }
    }

    ///
    /// Calls for the setup of the login window and for the setup of the register button, then shows the login window
    ///
    pub fn show(&self, tx_frontend: Sender<Message>, server_ip: String, server_name: String) {
        self.setup(tx_frontend.clone(), server_ip.clone(), server_name.clone());
        self.setup_register(tx_frontend, server_ip, server_name);
        self.window.show_all();
    }

    ///
    /// Setup login submenu and login button functionality
    /// When the button is clicked, it sends the cooresponding commands to the thread, for the client to send to the server
    ///
    pub fn setup(&self, tx_frontend: Sender<Message>, server_ip: String, server_name: String) {
        let login_button: gtk::Button = self
            .builder
            .object("login_button")
            .expect("Couldn't get login_button");
        let nickname_entry: gtk::Entry = self
            .builder
            .object("entry_nickname")
            .expect("Couldn't get entry_username");
        let password_entry: gtk::Entry = self
            .builder
            .object("entry_password")
            .expect("Couldn't get entry_password");
        let realname_entry: gtk::Entry = self
            .builder
            .object("entry_realname")
            .expect("Couldn't get entry_password");
        let error_label: gtk::Label = self
            .builder
            .object("label_login_error")
            .expect("Couldn't get entry_password");
        let user_env = match env::var("USER") {
            // Search username por environment variable
            Ok(val) => val,
            Err(_e) => "Anonymous".to_string(),
        };

        login_button.connect_clicked(move |_| {
            if password_entry.text().is_empty()
                || nickname_entry.text().is_empty()
                || realname_entry.text().is_empty()
            {
                error_label.set_text("Please fill in all values");
            } else {
                tx_frontend
                    .send(Message {
                        prefix: None,
                        command: LOGIN.to_string(),
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

    ///
    /// Setup register button functionality
    ///
    pub fn setup_register(
        &self,
        tx_frontend: Sender<Message>,
        server_ip: String,
        server_name: String,
    ) {
        let register_button: gtk::Button = self
            .builder
            .object("register_button")
            .expect("Couldn't get register_button");

        let window_register_clone = self.window_register.clone();

        register_button.connect_clicked(move |_| {
            window_register_clone.show(tx_frontend.clone(), server_ip.clone(), server_name.clone());
        });
    }

    ///
    /// Updates the login error label
    ///
    pub fn invalid_login(&self) {
        let login_error_label: gtk::Label = self
            .builder
            .object("label_login_error")
            .expect("Couldn't get login_error_label");
        login_error_label.set_text("Invalid nickname or password");
    }

    // Hides the login window
    pub fn hide(&self) {
        self.window.hide();
    }

    // Hides the register window
    pub fn hide_registration(&self) {
        self.window_register.hide();
    }

    // Sets the invalid registration label
    pub fn invalid_registration(&self, error: String) {
        self.window_register.invalid_registration(error);
    }
}
