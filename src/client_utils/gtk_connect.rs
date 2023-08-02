use super::gtk_login;
use crate::{
    custom_errors::{
        client_error::ClientError,
        errors::{CRITICAL, SEND_MESSAGE},
    },
    message::Message,
};
use gtk::{prelude::*, Builder};

pub struct WindowConnect {
    pub window: gtk::ApplicationWindow,
    pub builder: gtk::Builder,
    pub window_login: gtk_login::WindowLogin,
}

///
/// Initialize the connection window
///
impl WindowConnect {
    pub fn new(application: gtk::Application) -> Self {
        let glade_src = include_str!("connect.glade");
        let builder = Builder::new();
        builder
            .add_from_string(glade_src)
            .expect("Couldn't add from string");
        let window: gtk::ApplicationWindow = builder.object("window").expect("Couldn't get window");
        window.set_application(Some(&application));

        let window_login = gtk_login::WindowLogin::new(application);
        window.connect_delete_event(move |_, _| {
            println!("Finished application.");
            gtk::main_quit();
            Inhibit(false)
        });
        WindowConnect {
            window,
            builder,
            window_login,
        }
    }

    ///
    /// Calls for the setup of the connection window and shows it
    ///
    pub fn show(&self, tx_backend: gtk::glib::Sender<Message>) {
        self.setup(tx_backend);
        self.window.show_all();
    }

    ///
    /// Setup the connection window, define the action for the connect button
    ///
    pub fn setup(&self, tx_backend: gtk::glib::Sender<Message>) {
        let login_button: gtk::Button = self
            .builder
            .object("connect_button")
            .expect("Couldn't get login_button");
        let server_name_entry: gtk::Entry = self
            .builder
            .object("entry_server_name")
            .expect("Couldn't get entry_username");
        let server_ip_entry: gtk::Entry = self
            .builder
            .object("entry_server_ip")
            .expect("Couldn't get entry_password");
        let port_entry: gtk::Entry = self
            .builder
            .object("entry_port")
            .expect("Couldn't get entry_password");

        // Sends a connection attempt to the Client
        login_button.connect_clicked(move |_| {
            tx_backend
                .send(Message {
                    prefix: None,
                    command: "CONNECTION_ATTEMPT".to_string(),
                    params: vec![vec![
                        server_name_entry.text().to_string(),
                        server_ip_entry.text().to_string(),
                        port_entry.text().to_string(),
                    ]],
                })
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: CRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })
                .ok();
        });
    }

    ///
    /// Updates the error label
    ///
    pub fn connection_error(&self) {
        let error_label: gtk::Label = self
            .builder
            .object("label_connection_error")
            .expect("Couldn't get entry_password");
        error_label.set_text("Couldn't connect to server");
    }

    // Hides the connection window
    pub fn hide(&self) {
        self.window.hide();
    }
}
