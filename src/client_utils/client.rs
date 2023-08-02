use crate::client_utils::client::gtk_methods::find_user_by_current_buffer;
use crate::client_utils::client::gtk_methods::send_privmsg;
use crate::client_utils::client::gtk_methods::wait_connection_dcc_file;
use crate::commands::DCC_ACCEPT;
use crate::commands::DCC_CLOSE;
use crate::commands::PAUSE;
use crate::parser;
use gtk::glib;
use gtk::prelude::*;
use gtk::Builder;
use gtk::TextBuffer;
use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::net::Shutdown;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[path = "gtk_methods.rs"]
mod gtk_methods;

#[path = "gtk_login.rs"]
mod gtk_login;

#[path = "gtk_connect.rs"]
mod gtk_connect;

#[path = "ui_updater.rs"]
mod ui_updater;

#[path = "message_types.rs"]
mod message_types;

use crate::commands::{
    ADD_LIST_CHATS, CONNECTION_ATTEMPT, CORRECT_LOGIN, CORRECT_REGISTRATION, DCC_CHAT, DCC_RESUME,
    DCC_SEND, ERROR_CHANNEL, INVALID_LOGIN, INVALID_REGISTRATION, KICK_CHANNEL, LIST_CHANNELS,
    PART_CHANNEL, QUIT, RECEIVED_MESSAGE, SEARCH_USERS,
};
use crate::custom_errors::client_error::ClientError;
use crate::custom_errors::errors::LOCK_DCC;
use crate::custom_errors::errors::{CRITICAL, NONCRITICAL, RECEIVE_MESSAGE, SEND_MESSAGE};
use crate::message::Message;

use self::gtk_connect::WindowConnect;
use self::gtk_login::WindowLogin;
use self::message_types::{ERROR, INFO, PRIVATE_MESSAGE};

// This is the main struct of the client
pub struct Client {
    pub application: gtk::Application,
    // A map of all chats and their corresponding buffers
    pub online_chats_buffers: Arc<Mutex<HashMap<String, TextBuffer>>>,
    //A map of all chats names
    pub online_chats_names: Arc<Mutex<Vec<String>>>,
    // A vector of all current channels
    pub channels: Arc<Mutex<Vec<String>>>,
    // Hashmap of senders, if there us a private connection it will have it here, if not it wont appear
    pub dcc_chats: Arc<Mutex<HashMap<String, Sender<Message>>>>,
    // Hashmap of files being sent, this is to keep track of the file path if a file transfer is not completed
    pub dcc_file_paths: Arc<Mutex<HashMap<String, PathBuf>>>,
    // Gtk builder.
    pub builder: Builder,
    // Gtk login window
    pub window_login: WindowLogin,
    // Gtk connection window
    pub window_connect: WindowConnect,

    pub window: gtk::ApplicationWindow,
}

impl Client {
    ///
    /// Creates a new Client, initializing all the necessary fields
    ///
    fn new(application: &gtk::Application) -> Self {
        // Setup builder
        let client_glade_src = include_str!("client.glade");
        let builder = Builder::new();
        builder
            .add_from_string(client_glade_src)
            .expect("Couldn't add from string");

        let window_connect = WindowConnect::new(application.clone());
        let window_login = WindowLogin::new(application.clone());
        let window = builder.object("window").expect("Couldn't get window");
        Client {
            application: application.clone(),
            online_chats_buffers: Arc::new(Mutex::new(HashMap::new())),
            online_chats_names: Arc::new(Mutex::new(Vec::new())),
            channels: Arc::new(Mutex::new(Vec::new())),
            dcc_chats: Arc::new(Mutex::new(HashMap::new())),
            dcc_file_paths: Arc::new(Mutex::new(HashMap::new())),
            builder,
            window_login,
            window_connect,
            window,
        }
    }

    ///
    /// Starts the client, creates the threads for communication with the server
    /// and finally waits for updates to the UI
    ///
    fn run(self) {
        // Communication channel from backend to frontend (UI Updater to Client)
        let (tx_backend, rx_frontend): (gtk::glib::Sender<Message>, gtk::glib::Receiver<Message>) =
            glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        // Communication channel from frontend to backend (Client to UI Listener)
        let (tx_frontend, rx_backend): (Sender<Message>, Receiver<Message>) = mpsc::channel();

        // Show connect window
        self.window_connect.show(tx_backend.clone());

        // Communication channel for sending the stream to the UI Updater after a successful connection
        let (tx_stream_1, rx_stream_1): (Sender<TcpStream>, Receiver<TcpStream>) = mpsc::channel();
        // Communication channel for sending the stream to the UI Listener after a successful connection
        let (tx_stream_2, rx_stream_2): (Sender<TcpStream>, Receiver<TcpStream>) = mpsc::channel();

        let tx_backend_clone = tx_backend.clone();

        // Create thread that listens to TCP messages and updates the UI through the channel
        let online_users_names_clone = self.online_chats_names.clone();
        let _ = thread::spawn(move || {
            match ui_updater::ui_updater(tx_backend, rx_stream_1, &online_users_names_clone) {
                Ok(_) => {}
                Err(err_message) => {
                    println!("Error in ui_updater {}", err_message)
                }
            }
        });

        // Create thread that listens to UI changes and writes to the socket through the channel
        let _ = thread::spawn(move || match ui_listener(rx_backend, rx_stream_2) {
            Ok(_) => {}
            Err(err_message) => {
                println!("Error in ui_listener {}", err_message)
            }
        });

        // Update the UI
        self.update_ui(
            rx_frontend,
            tx_backend_clone,
            tx_frontend,
            tx_stream_1,
            tx_stream_2,
        );
    }

    ///
    /// Wait for messages from the ui_updater to update the frontend according to the received message
    ///
    fn update_ui(
        self,
        rx_frontend: gtk::glib::Receiver<Message>,
        tx_backend: gtk::glib::Sender<Message>,
        tx_frontend: Sender<Message>,
        tx_stream_1: Sender<TcpStream>,
        tx_stream_2: Sender<TcpStream>,
    ) {
        //Listen to update frontend
        let message_actuator = move |message: Message| -> glib::Continue {
            let mut buffers = self.online_chats_buffers.lock().unwrap();
            let mut users = self.online_chats_names.lock().unwrap();
            let mut channels = self.channels.lock().unwrap();
            let text_view: gtk::TextView = self
                .builder
                .object("chat_text")
                .expect("Couldn't get chat_text");

            let current_name_chat = find_user_by_current_buffer(buffers.clone(), &text_view);
            println!("Message received from ui_updater: {:?}", message);
            match &*message.command {
                CONNECTION_ATTEMPT => self.create_stream(
                    &message,
                    tx_stream_1.clone(),
                    tx_stream_2.clone(),
                    tx_frontend.clone(),
                ),
                CORRECT_LOGIN => self.correct_login(
                    &message,
                    tx_frontend.clone(),
                    tx_backend.clone(),
                    &mut users,
                    &mut buffers,
                ),
                INVALID_LOGIN => self.window_login.invalid_login(),
                CORRECT_REGISTRATION => self.correct_registration(
                    &message,
                    tx_frontend.clone(),
                    tx_backend.clone(),
                    &mut users,
                    &mut buffers,
                ),
                INVALID_REGISTRATION => self
                    .window_login
                    .invalid_registration(message.params[0][0].clone()),
                ADD_LIST_CHATS => {
                    self.add_list_chats(&message, &mut users, &mut buffers, &mut channels)
                }
                RECEIVED_MESSAGE => self.received_message(&message, &mut buffers, &mut channels),
                LIST_CHANNELS => self.list_channels(message),
                SEARCH_USERS => self.search_users(message, &tx_backend),
                PART_CHANNEL => self.delete_chat(&message, &mut users, &mut buffers, &mut channels),
                KICK_CHANNEL => self.delete_chat(&message, &mut users, &mut buffers, &mut channels),
                ERROR_CHANNEL => self.error_channel(&message),
                QUIT => {
                    gtk::main_quit();
                    return glib::Continue(false);
                }
                DCC_CHAT => self.accept_or_reject_dcc(
                    message.clone(),
                    &tx_backend,
                    tx_frontend.clone(),
                    &message.params[3][0],
                    current_name_chat,
                ),
                DCC_SEND => self.accept_or_reject_dcc(
                    message.clone(),
                    &tx_backend,
                    tx_frontend.clone(),
                    &message.params[4][0],
                    current_name_chat,
                ),
                DCC_RESUME => self.accept_or_reject_dcc(
                    message.clone(),
                    &tx_backend,
                    tx_frontend.clone(),
                    &message.params[4][0],
                    current_name_chat,
                ),
                DCC_CLOSE => self.close_dcc(message, &tx_backend),
                DCC_ACCEPT => self.join_dcc(message, &tx_backend),

                _ => println!("Undefined message received by the client"),
            }
            glib::Continue(true)
        };
        // Update the UI depending on the message received
        rx_frontend.attach(None, message_actuator);
    }

    ///
    /// Tries to create a new stream to the server with the given ip and port
    ///
    fn create_stream(
        &self,
        message: &Message,
        tx_stream_1: Sender<TcpStream>,
        tx_stream_2: Sender<TcpStream>,
        tx_frontend: Sender<Message>,
    ) {
        println!("Creating stream");
        let server_name = message.params[0][0].clone();
        let server_ip = message.params[0][1].clone();
        let server_port = message.params[0][2].clone();

        let address = format!("{}:{}", server_ip, server_port);

        println!("Connecting to {}", address);

        //Create the TCP connection
        let stream = match TcpStream::connect(address) {
            Ok(stream) => stream,
            Err(_) => {
                println!("Couldn't connect to server");
                // Show error message in the connect window
                self.window_connect.connection_error();
                return;
            }
        };
        self.window_connect.hide();
        self.window_login.show(tx_frontend, server_ip, server_name);

        let stream_clone = stream.try_clone().expect("Couldn't clone the stream");
        tx_stream_1
            .send(stream)
            .map_err(|_| -> ClientError {
                ClientError {
                    kind: NONCRITICAL.to_string(),
                    message: SEND_MESSAGE.to_string(),
                }
            })
            .ok();
        tx_stream_2
            .send(stream_clone)
            .map_err(|_| -> ClientError {
                ClientError {
                    kind: NONCRITICAL.to_string(),
                    message: SEND_MESSAGE.to_string(),
                }
            })
            .ok();
    }

    ///
    /// Get the name of the currently selected chat
    ///
    fn current_chat_name(&self, buffers: &mut HashMap<String, TextBuffer>) -> String {
        let text_view: gtk::TextView = self
            .builder
            .object("chat_text")
            .expect("Couldn't get chat_text");
        let current_buffer = text_view.buffer().expect("Couldn't get buffer");
        let current_user = buffers.iter().find_map(|(key, val)| {
            if val == &current_buffer {
                Some(key)
            } else {
                None
            }
        });
        match current_user {
            Some(user) => user.to_string(),
            None => "".to_string(),
        }
    }

    ///
    /// Handles a correct login
    ///
    fn correct_login(
        &self,
        message: &Message,
        tx_frontend: Sender<Message>,
        tx_backend: gtk::glib::Sender<Message>,
        users: &mut Vec<String>,
        buffers: &mut HashMap<String, TextBuffer>,
    ) {
        // Hide login window
        self.window_login.hide();
        // Initialize and show main window
        gtk_methods::initialize_gtk_window(
            self,
            tx_frontend,
            tx_backend,
            users,
            buffers,
            &message.params[0][0],
        );
    }

    ///
    /// Handles a correct registration
    ///
    fn correct_registration(
        &self,
        message: &Message,
        tx_frontend: Sender<Message>,
        tx_backend: gtk::glib::Sender<Message>,
        users: &mut Vec<String>,
        buffers: &mut HashMap<String, TextBuffer>,
    ) {
        println!("Message params: {:?}", message.params);
        // Hide login window
        self.window_login.hide();
        // Hide registration window
        self.window_login.hide_registration();
        // Initialize and show main window
        gtk_methods::initialize_gtk_window(
            self,
            tx_frontend,
            tx_backend,
            users,
            buffers,
            &message.params[0][0],
        );
    }

    ///
    /// Adds a new chat to the list of chats, creating a new buffer and button for it
    ///
    fn add_list_chats(
        &self,
        message: &Message,
        users: &mut Vec<String>,
        buffers: &mut HashMap<String, TextBuffer>,
        channels: &mut Vec<String>,
    ) {
        let name = message.prefix.clone().expect("No prefix in message"); // Can be a username or a channel name

        // Check if the chat is already in the list
        if users.contains(&name) {
            println!("Chat {} already exists", name);
            return;
        }
        // Check if the chat is a channel
        if name.starts_with('#') || name.starts_with('&') {
            channels.push(name.clone());
        }

        let list_box: gtk::ListBox = self
            .builder
            .object("chats_list")
            .expect("Couldn't get chats_list");
        let new_buffer = TextBuffer::builder().build();

        // Add the new user to the list of users
        users.push(name.clone());
        // Add the new buffer to the list of buffers
        buffers.insert(name.clone(), new_buffer.clone());
        // Create the new chat button
        let user_button = gtk_methods::new_user_chat_button(self, &name, new_buffer);
        list_box.add(&user_button);
        list_box.show_all();
    }

    ///
    /// Deletes a button from the list of chats
    ///
    fn delete_list_chats(&self, name: String) {
        let list_box: gtk::ListBox = self
            .builder
            .object("chats_list")
            .expect("Couldn't get chats_list");
        for widget in list_box.children() {
            let row = widget
                .clone()
                .downcast::<gtk::ListBoxRow>()
                .expect("Couldn't downcast to listboxrow");
            let button = row
                .child()
                .expect("Couldn't get row")
                .downcast::<gtk::Button>()
                .expect("Couldn't downcast to button");
            let button_name = button.label().expect("Couldn't get button label");
            if button_name == name {
                list_box.remove(&widget);
                break;
            }
        }
        list_box.show_all();
    }

    ///
    /// Handles receiving a message
    ///
    fn received_message(
        &self,
        message: &Message,
        buffers: &mut HashMap<String, TextBuffer>,
        channels: &mut [String],
    ) {
        // Get the sender
        let name = match &message.prefix {
            Some(prefix) => prefix.clone(),
            None => self.current_chat_name(buffers),
        };
        // If it's from a channel, and the channel is not on the list, ignore the message
        if (name.starts_with('#') || name.starts_with('&')) && !channels.contains(&name) {
            return;
        }
        let buffer = buffers.get(&name).expect("Couldn't get buffer");
        let mut end = buffer.end_iter();

        let message_type = &*message.params[0][1];

        // Adds a tag to the message depending on the type
        let message_to_print = match message_type {
            PRIVATE_MESSAGE => format!("{}\r\n", message.params[0][0].clone()),
            INFO => format!("{} {} \r\n", "@INFO", message.params[0][0].clone()),
            ERROR => format!("{} {}\r\n", "@ERROR", message.params[0][0].clone()),
            _ => format!("{} {}\r\n", "@UNDEFINED", message.params[0][0].clone()),
        };

        buffer.insert(&mut end, &message_to_print); // Add the new message to the buffer
    }

    ///
    /// Adds the channel names to the list of visible channels
    ///
    fn list_channels(&self, message: Message) {
        let list_channels: gtk::TextView = self
            .builder
            .object("channels_list_text")
            .expect("Couldn't get list_channels");
        let buffer = list_channels.buffer().expect("Couldn't get buffer");
        let mut end = buffer.end_iter();
        buffer.delete(&mut buffer.start_iter(), &mut end);
        if message.params[0].is_empty() {
            buffer.insert(&mut end, "No channels available");
        } else {
            for channel in message.params[0].clone() {
                buffer.insert(&mut end, &channel);
                buffer.insert(&mut end, "\r\n");
            }
        }
    }

    ///
    /// Adds the buttons for creating a new chat when searching for a user
    ///
    fn search_users(&self, message: Message, tx_backend: &gtk::glib::Sender<Message>) {
        let list_box: gtk::ListBox = self
            .builder
            .object("users_list")
            .expect("Couldn't get users_list");
        let self_nickname = self.get_current_nickname();

        // Empty the list
        for button in list_box.children() {
            list_box.remove(&button);
        }

        for user_nickname in message.params[0].clone() {
            // If the user is me, don't add it to the list
            if user_nickname == self_nickname {
                continue;
            }
            let user_button = gtk_methods::new_user_search_button(user_nickname, tx_backend); // Create the new button

            list_box.add(&user_button);
            list_box.show_all();
        }
    }

    ///
    /// Gets the nickname of the user logged in
    ///
    fn get_current_nickname(&self) -> String {
        let nickname_label: gtk::Label = self
            .builder
            .object("nickname_label")
            .expect("Couldn't get nickname label");

        nickname_label
            .text()
            .split(' ')
            .collect::<Vec<&str>>()
            .last()
            .expect("Couldn't get text for label")
            .to_string()
    }

    ///
    /// Handles deleting a chat, removes all references to it and call delete_list_chats
    ///
    fn delete_chat(
        &self,
        message: &Message,
        users: &mut Vec<String>,
        buffers: &mut HashMap<String, TextBuffer>,
        channels: &mut Vec<String>,
    ) {
        let name = message.params[0][0].clone(); // Can be a username or a channel name
        users.retain(|x| x != &name.clone()); // Remove the current user from the list of users
        buffers.remove(&name.clone()); // Remove the current buffer from the list of buffers
        channels.retain(|x| x != &name.clone()); // If it's a channel, remove it from the list of channels
        self.delete_list_chats(name.clone()); // Remove the current user from the list of users

        let text_view: gtk::TextView = self
            .builder
            .object("chat_text")
            .expect("Couldn't get chat_text");
        let self_chat_buffer = buffers.get("You").expect("Couldn't get buffer");
        text_view.set_buffer(Some(self_chat_buffer));
    }

    ///
    /// Handle the dcc request
    /// Show a dialog to the user to accept or reject the request
    ///
    fn accept_or_reject_dcc(
        &self,
        message: Message,
        tx_backend: &gtk::glib::Sender<Message>,
        tx_frontend: Sender<Message>,
        text_show: &str,
        chat_name: String,
    ) {
        let dialog = gtk::MessageDialog::builder()
            .transient_for(&self.window)
            .modal(false)
            .buttons(gtk::ButtonsType::YesNo)
            .text(text_show)
            .build();
        let response = dialog.run();
        dialog.close();
        if response == gtk::ResponseType::Yes {
            if message.command == DCC_RESUME {
                self.resume_dcc(message, tx_backend, tx_frontend, chat_name);
            } else {
                self.join_dcc(message, tx_backend);
            }
        }
    }

    ///
    /// Joins a dcc connection with the user
    ///
    fn join_dcc(&self, message: Message, tx_backend: &gtk::glib::Sender<Message>) {
        // Communication channel from frontend to backend (Client to UI Listener)
        let (dcc_sender, dcc_receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();

        let address = format!("{}:{}", message.params[1][0], message.params[2][0]);

        println!("Connecting to {}", address);

        //Create the TCP connection
        let stream = match TcpStream::connect(address) {
            Ok(stream) => stream,
            Err(_) => {
                // Send start private chat message to the UI
                let _res = tx_backend.send(Message {
                    prefix: Some(message.prefix.unwrap()),
                    command: RECEIVED_MESSAGE.to_string(),
                    params: vec![vec![
                        format!(
                            "Couldn't start private chat due to error or time to connect expired"
                        ),
                        ERROR.to_string(),
                    ]],
                });
                return;
            }
        };

        let tx_backend_clone = tx_backend.clone();
        let user_to_send = message.prefix.clone().unwrap();

        if message.command == DCC_CHAT {
            let mut chats = self.dcc_chats.lock().expect(LOCK_DCC);
            chats.insert(message.prefix.expect("No prefix in message"), dcc_sender);
            // Spawn new thread for the client
            let _ = thread::spawn(move || {
                match handle_dcc_chat(stream, tx_backend_clone, dcc_receiver, &user_to_send) {
                    Ok(_) => println!("Client disconnected: dcc_chat"),
                    Err(err) => println!("Error: {}", err),
                }
            });
        } else if message.command == DCC_SEND {
            let mut chats = self.dcc_chats.lock().expect(LOCK_DCC);
            let name = message.prefix.expect("No prefix in message");
            chats.insert(format!("{}_f", name), dcc_sender);
            // Spawn new thread for the client
            let _ = thread::spawn(move || {
                // connection succeeded
                match handle_dcc_file_received(
                    stream,
                    tx_backend_clone,
                    dcc_receiver,
                    &user_to_send,
                    message.params[0][0].clone(),
                    0,
                ) {
                    Ok(_) => println!("Client disconnected: dcc_chat"),
                    Err(err) => println!("Error: {}", err),
                }
            });
        } else if message.command == DCC_ACCEPT {
            let mut chats = self.dcc_chats.lock().expect(LOCK_DCC);
            let name = message.prefix.expect("No prefix in message");
            chats.insert(format!("{}_f", name), dcc_sender);
            let position = message.params[3][0].clone().parse().unwrap();
            // Spawn new thread for the client
            let _ = thread::spawn(move || {
                // connection succeeded
                match handle_dcc_file_received(
                    stream,
                    tx_backend_clone,
                    dcc_receiver,
                    &user_to_send,
                    message.params[0][0].clone(),
                    position,
                ) {
                    Ok(_) => println!("Client disconnected: dcc_chat"),
                    Err(err) => println!("Error: {}", err),
                }
            });
        } else {
            println!("Unknown DCC command");
        }
    }

    ///
    /// Close a dcc connection with the user
    /// Remove the user from the list of dcc chats
    ///
    fn close_dcc(&self, message: Message, tx_backend: &gtk::glib::Sender<Message>) {
        let other_user_nickname = &message.prefix.expect("No prefix in message");

        let mut chats = self.dcc_chats.lock().expect(LOCK_DCC);
        chats.remove(other_user_nickname);

        tx_backend
            .send(Message {
                prefix: Some(other_user_nickname.clone()),
                command: RECEIVED_MESSAGE.to_string(),
                params: vec![vec![format!("CLOSED PRIVATE CONNECTION"), INFO.to_string()]],
            })
            .unwrap();
    }

    ///
    /// Resume a dcc connection with the user
    /// Create a new thread for the dcc send file
    ///
    fn resume_dcc(
        &self,
        message: Message,
        tx_backend: &gtk::glib::Sender<Message>,
        tx_frontend: Sender<Message>,
        current_name_chat: String,
    ) {
        println!("Resuming DCC SEND");
        let text_view: gtk::TextView = self
            .builder
            .object("chat_text")
            .expect("Couldn't get chat_text");

        println!("Current name chat: {}", current_name_chat);
        let listener = TcpListener::bind("0.0.0.0:0").unwrap();
        // Spawn new thread for the client
        let address = listener.local_addr().unwrap();
        println!("Listening on {}", address);
        let ip = address.ip().to_string();
        let port = address.port().to_string();
        let tx_backend_clone = tx_backend.clone();
        let current_name_chat_clone = current_name_chat.clone();
        let file_path = match self
            .dcc_file_paths
            .lock()
            .expect(LOCK_DCC)
            .get(&message.params[0][0])
        {
            Some(path) => path.clone(),
            None => {
                println!("No file path found");
                return;
            }
        };
        println!("Message in resume: {:?}", message);
        let start_position = message.params[3][0]
            .parse::<u64>()
            .expect("Position not valid");

        let _ = thread::spawn(move || {
            // connection succeeded
            match wait_connection_dcc_file(
                listener,
                tx_backend_clone,
                current_name_chat_clone,
                file_path,
                start_position,
            ) {
                Ok(_) => println!("Private connection ended"),
                Err(err) => println!("Error: {}", err),
            }
        });

        let message = format!(
            "DCC_ACCEPT {} {} {} {}",
            message.params[0][0], ip, port, message.params[3][0]
        );
        println!("Sending message: {}", message);

        send_privmsg(
            &tx_frontend, //start chat and thread to send file
            &message,
            None,
            text_view,
            current_name_chat,
            false,
        );

        tx_backend
            .send(Message {
                prefix: None,
                command: RECEIVED_MESSAGE.to_string(),
                params: vec![vec!["Resuming file transfer".to_string(), INFO.to_string()]],
            })
            .unwrap();
    }

    ///
    /// Updates the error label when trying to join a channel
    ///
    fn error_channel(&self, message: &Message) {
        let error_message = message.params[0][0].clone();
        self.builder
            .object::<gtk::Label>("label_channel_name_error")
            .expect("Couldn't get label")
            .set_text(&error_message);
    }
}

///
/// Creates the client and runs it
///
pub fn init_chat(application: &gtk::Application) {
    let client = Client::new(application);

    client.run();
}

///
/// When the user performs an action, this method sends it through TCP to the server
///
fn ui_listener(
    rx_backend: Receiver<Message>,
    rx_stream: Receiver<TcpStream>,
) -> Result<(), ClientError> {
    let mut stream = rx_stream.recv().map_err(|_| -> ClientError {
        ClientError {
            kind: CRITICAL.to_string(),
            message: RECEIVE_MESSAGE.to_string(),
        }
    })?;

    while match rx_backend.try_recv() {
        Ok(message) => {
            println!("Sending message: {:?}", message);
            stream
                .write_all(message.as_string().as_bytes())
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: CRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })?;
            true
        }
        Err(TryRecvError::Empty) => true,
        Err(TryRecvError::Disconnected) => {
            println!("Client disconnected from server");
            false
        }
    } {}

    Ok(())
}

///
/// Handles the dcc chat
/// Send messages to the UI
/// and call the function to receive messages from the UI
///
fn handle_dcc_chat(
    stream: TcpStream,
    tx_backend: gtk::glib::Sender<Message>,
    dcc_receiver: Receiver<Message>,
    user_to_send: &String,
) -> Result<(), ClientError> {
    println!("I'm in a new thread");
    // Send start private chat message to the UI
    tx_backend
        .send(Message {
            prefix: Some(user_to_send.clone()),
            command: RECEIVED_MESSAGE.to_string(),
            params: vec![vec![
                format!("STARTED PRIVATE CONNECTION WITH {}", user_to_send),
                INFO.to_string(),
            ]],
        })
        .map_err(|_| -> ClientError {
            ClientError {
                kind: CRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            }
        })?;

    let mut reader = BufReader::new(stream.try_clone().unwrap());
    stream
        .set_read_timeout(Some(Duration::from_millis(100))) // this is needed so that it doesnt block
        .map_err(|_| -> ClientError {
            ClientError {
                kind: CRITICAL.to_string(),
                message: "Could not set stream time out".to_string(),
            }
        })?;

    let mut line = String::new();

    println!("Waiting for messages");
    while match reader.read_line(&mut line) {
        Ok(_bytes_read) => {
            // If data was read then handle it
            if !line.is_empty() {
                let message = parser::parse(line.clone()).unwrap();
                let message_to_print =
                    format!("{}: {}", user_to_send, message.params[1][0].clone());
                println!("Received message: {:?}", line);
                tx_backend
                    .send(Message {
                        prefix: Some(user_to_send.to_string()),
                        command: RECEIVED_MESSAGE.to_string(),
                        params: vec![vec![message_to_print, PRIVATE_MESSAGE.to_string()]],
                    })
                    .map_err(|_| -> ClientError {
                        ClientError {
                            kind: CRITICAL.to_string(),
                            message: SEND_MESSAGE.to_string(),
                        }
                    })?; // Send the message to the UI
                line.clear();
            }
            true
        }
        Err(_error) => {
            true // Keep looping
        }
    } {
        // In every execution execute this block
        read_to_send_private_chat(&stream, &dcc_receiver, &tx_backend)?;
    }
    Ok(())
}

///
/// Reads the messages from the UI and sends them to the stream
/// If the message is a close message, it closes the connection
///
fn read_to_send_private_chat(
    mut stream: &TcpStream,
    dcc_receiver: &Receiver<Message>,
    tx_backend: &gtk::glib::Sender<Message>,
) -> Result<(), ClientError> {
    match dcc_receiver.try_recv() {
        Ok(message) => {
            if message.command == *DCC_CLOSE {
                println!("Closing private chat");
                stream
                    .shutdown(Shutdown::Both)
                    .map_err(|_| -> ClientError {
                        ClientError {
                            kind: CRITICAL.to_string(),
                            message: SEND_MESSAGE.to_string(),
                        }
                    })?;
                tx_backend
                    .send(Message {
                        prefix: None,
                        command: RECEIVED_MESSAGE.to_string(),
                        params: vec![vec![format!("CLOSED PRIVATE CONNECTION"), INFO.to_string()]],
                    })
                    .map_err(|_| -> ClientError {
                        ClientError {
                            kind: CRITICAL.to_string(),
                            message: SEND_MESSAGE.to_string(),
                        }
                    })?;
            } else {
                println!("Sending message by private chat: {:?}", message);
                stream
                    .write_all(message.as_string().as_bytes())
                    .map_err(|_| -> ClientError {
                        ClientError {
                            kind: CRITICAL.to_string(),
                            message: SEND_MESSAGE.to_string(),
                        }
                    })?;
            }
        }
        Err(TryRecvError::Empty) => {}
        Err(TryRecvError::Disconnected) => {
            println!("Client disconnected from private chat");
            return Err(ClientError {
                kind: CRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            });
        }
    }
    Ok(())
}

///
/// Handles the dcc file transfer RECEIVER
/// Creates a new file and writes the data received from the stream
/// If the message is a pause message, it kill the connection
///
fn handle_dcc_file_received(
    mut stream: TcpStream,
    tx_backend: gtk::glib::Sender<Message>,
    dcc_receiver: Receiver<Message>,
    user_to_send: &str,
    file_name: String,
    start_position: u64,
) -> Result<(), ClientError> {
    println!("I'm in a new thread waiting for a file");

    tx_backend
        .send(Message {
            prefix: Some(user_to_send.to_string()),
            command: RECEIVED_MESSAGE.to_string(),
            params: vec![vec!["Started receiving file".to_string(), INFO.to_string()]],
        })
        .map_err(|_| -> ClientError {
            ClientError {
                kind: CRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            }
        })?;

    let mut my_file = match start_position {
        0 => {
            File::create("received_files/".to_owned() + &file_name).map_err(|_| -> ClientError {
                ClientError {
                    kind: CRITICAL.to_string(),
                    message: SEND_MESSAGE.to_string(),
                }
            })?
        }
        _ => OpenOptions::new()
            .write(true)
            .append(true)
            .open("received_files/".to_owned() + &file_name)
            .unwrap(),
    };
    let mut buffer = [0; 1024];

    while let Ok(bytes_read) = stream.read(&mut buffer) {
        if bytes_read == 0 {
            break;
        }

        if let Ok(message) = dcc_receiver.try_recv() {
            println!(
                "Received message: {:?}
            this is to close
            works",
                message
            );
            if message.command == *PAUSE {
                stream
                    .shutdown(std::net::Shutdown::Both)
                    .map_err(|_| -> ClientError {
                        ClientError {
                            kind: NONCRITICAL.to_string(),
                            message: SEND_MESSAGE.to_string(),
                        }
                    })?;

                return Err(ClientError {
                    kind: CRITICAL.to_string(),
                    message: SEND_MESSAGE.to_string(),
                });
            }
        }

        let received = bytes_read as u32;
        stream
            .write_all(&received.to_be_bytes())
            .map_err(|_| -> ClientError {
                ClientError {
                    kind: CRITICAL.to_string(),
                    message: SEND_MESSAGE.to_string(),
                }
            })?;

        if bytes_read < 1024 {
            my_file
                .write_all(&buffer[0..bytes_read as usize])
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: CRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })?;
            break;
        }
        my_file.write_all(&buffer).map_err(|_| -> ClientError {
            ClientError {
                kind: CRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            }
        })?;
    }
    tx_backend
        .send(Message {
            prefix: Some(user_to_send.to_string()),
            command: RECEIVED_MESSAGE.to_string(),
            params: vec![vec![
                format!("File received completely from {}", user_to_send),
                INFO.to_string(),
            ]],
        })
        .map_err(|_| -> ClientError {
            ClientError {
                kind: CRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            }
        })?;
    Ok(())
}

///
/// Handles the dcc file transfer SENDER
/// Sends the file to the stream
///
pub fn handle_dcc_file_send(
    mut stream: TcpStream,
    tx_backend: gtk::glib::Sender<Message>,
    user_to_send: &str,
    file_path: PathBuf,
    start_position: u64,
) {
    println!("Handling sending DCC file");

    tx_backend
        .send(Message {
            prefix: None,
            command: RECEIVED_MESSAGE.to_string(),
            params: vec![vec!["Started file transfer".to_string(), INFO.to_string()]],
        })
        .unwrap();

    let mut my_file = File::open(file_path).unwrap();
    println!("Starting at position: {}", start_position);
    my_file.seek(SeekFrom::Start(start_position)).unwrap();
    let mut buffer = [0; 1024];

    while let Ok(bytes_read) = my_file.read(&mut buffer) {
        if bytes_read == 0 {
            break;
        }
        let mut buf = [0; 4];
        stream.write_all(&buffer[0..bytes_read]).unwrap();

        match stream.read_exact(&mut buf) {
            Ok(_) => {}
            Err(_) => {
                println!("Error reading ACK");
                return;
            }
        }
        let received = u32::from_be_bytes(buf);
        if received == bytes_read as u32 {
            //println!("Received correctly");
        } else {
            println!("Something went wrong");
        }
        //println!("Received ACK{:?}", received );
    }
    tx_backend
        .send(Message {
            prefix: None,
            command: RECEIVED_MESSAGE.to_string(),
            params: vec![vec![
                format!("File sent completely to {}", user_to_send),
                INFO.to_string(),
            ]],
        })
        .unwrap();

    stream.shutdown(std::net::Shutdown::Both).unwrap();
}

// These tests are commented, when using applications and gtk they don't work if they are all run together.
// If you run them individually, they work perfectly.

// #[cfg(test)]
// mod tests {
//     use crate::commands::ADD_LIST_CHATS;
//     use super::Message;
//     use super::Client;

//     #[test]
//     fn test_new_client() {
//         let application = gtk::Application::builder().application_id("test_app").build();
//         gtk::init().expect("Failed to initialize GTK.");
//         let client = Client::new(&application);
//         assert_eq!(client.application, application);
//     }

//     #[test]
//     fn test_add_new_user_chat(){
//         let application = gtk::Application::builder().application_id("test_app2").build();
//         gtk::init().expect("Failed to initialize GTK.");
//         let client = Client::new(&application);
//         let mut buffers = client.online_users_buffers.lock().unwrap();
//         let mut users = client.online_users_names.lock().unwrap();
//         let mut channels = client.channels.lock().unwrap();
//         let message = Message{prefix: Some("name_user".to_string()), command: ADD_LIST_CHATS.to_string(), params: vec![vec![]]};
//         client.add_list_chats(&message, &mut users, &mut buffers, &mut channels);

//         assert_eq!(users.len(), 1);
//         assert_eq!(channels.len(), 0);
//         assert_eq!(buffers.len(), 1);
//     }

//     #[test]
//     fn test_add_new_channel_chat(){
//         let application = gtk::Application::builder().application_id("test_app3").build();
//         gtk::init().expect("Failed to initialize GTK.");
//         let client = Client::new(&application);
//         let mut buffers = client.online_users_buffers.lock().unwrap();
//         let mut users = client.online_users_names.lock().unwrap();
//         let mut channels = client.channels.lock().unwrap();
//         let message = Message{prefix: Some("#channel_name".to_string()), command: ADD_LIST_CHATS.to_string(), params: vec![vec![]]};
//         client.add_list_chats(&message, &mut users, &mut buffers, &mut channels);

//         assert_eq!(users.len(), 1);
//         assert_eq!(channels.len(), 1);
//         assert_eq!(buffers.len(), 1);
//     }

//     #[test]
//     fn test_delete_chat(){
//         let application = gtk::Application::builder().application_id("test_app3").build();
//         gtk::init().expect("Failed to initialize GTK.");
//         let client = Client::new(&application);
//         let mut buffers = client.online_users_buffers.lock().unwrap();
//         let mut users = client.online_users_names.lock().unwrap();
//         let mut channels = client.channels.lock().unwrap();
//         // This user is need to be added to the list of users because the delete_chat method send message to the chat You
//         let message = Message{prefix: Some("You".to_string()), command: ADD_LIST_CHATS.to_string(), params: vec![vec![]]};
//         client.add_list_chats(&message, &mut users, &mut buffers, &mut channels);

//         let message = Message{prefix: Some("name_user".to_string()), command: ADD_LIST_CHATS.to_string(), params: vec![vec![]]};
//         // Add user
//         client.add_list_chats(&message, &mut users, &mut buffers, &mut channels);
//         assert_eq!(users.len(), 2);

//         // Delete user
//         let message = Message{prefix: None, command: "test".to_string(), params: vec![vec!["name_user".to_string()]]};
//         client.delete_chat(&message, &mut users, &mut buffers, &mut channels);
//         assert_eq!(users.len(), 1);
//     }
// }
