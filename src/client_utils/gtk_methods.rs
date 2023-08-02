use gtk::prelude::*;
use gtk::TextBuffer;

use super::message_types::{ERROR, INFO};
use super::Client;
use crate::client_utils::client::handle_dcc_chat;
use crate::client_utils::client::handle_dcc_file_send;
use crate::commands::DCC_CLOSE;
use crate::commands::PAUSE;
use crate::commands::{
    ADD_LIST_CHATS, AWAY, DCC_CHAT, INVITE, JOIN, KICK, LIST, MODE, NAMES, OPER, OPERATOR, PART,
    PART_CHANNEL, PRIVMSG, QUIT, RECEIVED_MESSAGE, SQUIT, TOPIC, UNAWAY, WHO, WHOIS,
};
use crate::custom_errors::client_error::ClientError;
use crate::custom_errors::errors::LOCK_DCC;
use crate::custom_errors::errors::{LOCK_USERS, NONCRITICAL, SEND_MESSAGE};
use crate::message::Message;
use crate::parser;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::ErrorKind;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

///
/// Initializes main chat window
///
pub fn initialize_gtk_window(
    client: &Client,
    tx_frontend: Sender<Message>,
    tx_backend: gtk::glib::Sender<Message>,
    users: &mut Vec<String>,
    buffers: &mut HashMap<String, TextBuffer>,
    nickname: &str,
) {
    let window: gtk::ApplicationWindow = client
        .builder
        .object("window")
        .expect("Couldn't get window");
    window.set_application(Some(&client.application));

    // Setup all of the UI components
    setup_nickname_label(client, &nickname.to_owned());
    setup_own_chat(client, users, buffers);
    setup_channel_join(client, tx_frontend.clone());
    setup_send_button(client, tx_frontend.clone(), tx_backend.clone(), nickname);
    setup_channel_refresh_button(client, tx_frontend.clone());
    setup_search_user_button(client, tx_frontend.clone());
    setup_send_file_button(client, tx_frontend.clone(), tx_backend.clone());
    setup_pause_transfer_button(client);
    setup_resume_transfer_button(client, tx_frontend.clone());

    window.resize(1000, 600);
    window.show_all();
    window.connect_delete_event(move |_, _| {
        println!("Finished application.");
        tx_frontend
            .send(Message {
                prefix: None,
                command: QUIT.to_string(),
                params: vec![],
            })
            .map_err(|_| -> ClientError {
                ClientError {
                    kind: NONCRITICAL.to_string(),
                    message: SEND_MESSAGE.to_string(),
                }
            })
            .ok();
        tx_backend
            .send(Message {
                prefix: None,
                command: QUIT.to_string(),
                params: vec![],
            })
            .map_err(|_| -> ClientError {
                ClientError {
                    kind: NONCRITICAL.to_string(),
                    message: SEND_MESSAGE.to_string(),
                }
            })
            .ok();
        gtk::Inhibit(false)
    });
    hide_dummy_entry(client);
}

// This entry is used to get the focus on the chat window
pub fn hide_dummy_entry(client: &Client) {
    let dummy_entry: gtk::Entry = client
        .builder
        .object("dummy_no_focus")
        .expect("Couldn't get dummy entry");
    dummy_entry.hide();
}

// Sets up the nickname label with the nickname of the user
fn setup_nickname_label(client: &Client, nickname: &String) {
    let nickname_label: gtk::Label = client
        .builder
        .object("nickname_label")
        .expect("Couldn't get nickname label");
    nickname_label.set_text(&format!("Logged in as {}", nickname));
}

///
/// Setup the starting chat
/// The buffer for this chat is the default of the textview
///
fn setup_own_chat(
    client: &Client,
    users: &mut Vec<String>,
    buffers: &mut HashMap<String, TextBuffer>,
) {
    let list_box: gtk::ListBox = client
        .builder
        .object("chats_list")
        .expect("Couldn't get chats_list");
    let text_view: gtk::TextView = client
        .builder
        .object("chat_text")
        .expect("Couldn't get chat_you");

    // Modify word wrap mode
    text_view.set_wrap_mode(gtk::WrapMode::Word);
    // Modify indentation of text
    text_view.set_indent(0);

    let buffer = text_view.buffer().expect("Couldn't get buffer");
    setup_own_chat_tutorial(&buffer);
    let chat_name = "You".to_string();

    // Add the new user to the list of users
    users.push(chat_name.clone());
    buffers.insert(chat_name.clone(), buffer.clone());

    // Creates the new button
    let button = new_user_chat_button(client, &chat_name, buffer);
    list_box.add(&button);
    list_box.show_all();
}

///
/// Changes the text of the chat to the tutorial
///
fn setup_own_chat_tutorial(buffer: &TextBuffer) {
    let tutorial_text = "✉️ Welcome to Panicked at pensar Nombre's IRC client! ✉️\n
This is your own chat.\n
Apart from the UI, you can also use the following shortcuts:
♦️ /away [message] - Set your away status
♦️ /unaway - Remove your away status
♦️ /whois [nickname] - Get information about a user
♦️ /oper [password] - Become an IRC operator
♦️ /quit [message] - Quit the IRC server
♦️ /squit [server] [comment] - Disconnect a server from the IRC network
♦️ /names - Get a list of users in current channel
♦️ /topic [topic] - Set the topic of the current channel
♦️ /part - Leave current channel
♦️ /invite [nickname] - Invite a user to the current channel
♦️ /kick [nickname] - Kick a user from the current channel
♦️ /mode [mode] - Set the mode of the current channel
♦️ /dcc_chat - Send a DCC chat request to a user
♦️ /dcc_close - Close a DCC chat
Possible modes are:
⚪️ +k [key] - Set a channel key
⚪️ -k - Remove the channel key
⚪️ +l [limit] - Set a user limit
⚪️ -l - Remove the user limit
⚪️ (+/-)i - Set/remove the channel's invite-only mode
⚪️ (+/-)o [nickname] - Give/Remove a user operator status
⚪️ (+/-)t - Set/Remove the topic operator-only mode
⚪️ (+/-)s - Set/Remove the secret mode
⚪️ (+/-)b [nickname] - Ban/Unban a user from the channel

✉️ Have fun! ✉️\n\n";
    buffer.insert(&mut buffer.end_iter(), tutorial_text);
}

///
/// Setup join submenu and join button functionality
/// When the Join button is clicked, it sends the corresponding commands to the thread, for the client to send to the server
///
fn setup_channel_join(client: &Client, tx_frontend: Sender<Message>) {
    let channel_button: gtk::Button = client
        .builder
        .object("channel_button")
        .expect("Couldn't get channel_button");

    let channel_entry: gtk::Entry = client
        .builder
        .object("channel_entry")
        .expect("Couldn't get channel_entry");
    let channel_password_entry: gtk::Entry = client
        .builder
        .object("channel_password_entry")
        .expect("Couldn't get channel_password_entry");
    let label_channel_name_error: gtk::Label = client
        .builder
        .object("label_channel_name_error")
        .expect("Couldn't get label_channel_name_error");
    let channels_clone = client.channels.clone();
    channel_button.connect_clicked(move |_| {
        // If entry is empty, show an error message
        if channel_entry.text().is_empty() {
            label_channel_name_error.set_text("Please choose a channel name");
            return;
        }

        let channel_name = channel_entry.text().to_string();

        // If channel name doesn't start with # or &, show an error message
        if !(channel_name.starts_with('#') || channel_name.starts_with('&')) {
            label_channel_name_error.set_text("Must start with # or &");
            channel_entry.delete_text(0, -1);
            return;
        }

        // If starts with # or &, but is only 1 character long, show an error message
        if channel_entry.text().len() < 2 {
            label_channel_name_error.set_text("Channel name can't be empty");
            return;
        }

        let channels_clone = channels_clone.lock().expect("Couldn't lock channels");
        // If channel is already in the list of channels, don't send messages
        if channels_clone.contains(&channel_name) {
            label_channel_name_error.set_text("You're already in this channel");
            return;
        }

        let channel_password = channel_password_entry.text().to_string();

        // If password is empty, send a JOIN command without password, else, send a JOIN command with password
        if channel_password.is_empty() {
            tx_frontend
                .send(Message {
                    prefix: None,
                    command: JOIN.to_string(),
                    params: vec![vec![channel_name.clone()]],
                })
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: NONCRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })
                .ok();
        } else {
            tx_frontend
                .send(Message {
                    prefix: None,
                    command: JOIN.to_string(),
                    params: vec![vec![channel_name.clone()], vec![channel_password]],
                })
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: NONCRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })
                .ok();
        }

        tx_frontend
            .send(Message {
                prefix: None,
                command: LIST.to_string(),
                params: vec![],
            })
            .map_err(|_| -> ClientError {
                ClientError {
                    kind: NONCRITICAL.to_string(),
                    message: SEND_MESSAGE.to_string(),
                }
            })
            .ok();

        // Get names of users in channel
        tx_frontend
            .send(Message {
                prefix: None,
                command: NAMES.to_string(),
                params: vec![vec![channel_name]],
            })
            .map_err(|_| -> ClientError {
                ClientError {
                    kind: NONCRITICAL.to_string(),
                    message: SEND_MESSAGE.to_string(),
                }
            })
            .ok();

        label_channel_name_error.set_text("");
        channel_entry.delete_text(0, -1);
        channel_password_entry.delete_text(0, -1);
    });
}

///
/// Setup button "Refresh List" in the channel submenu.
/// When clicked, it sends the corresponding command to the thread, for the client to send to the server
///
fn setup_channel_refresh_button(client: &Client, tx_frontend: Sender<Message>) {
    let channel_refresh_button = client
        .builder
        .object::<gtk::Button>("channel_refresh_button")
        .expect("Couldn't get channel_refresh_button");
    channel_refresh_button.connect_clicked(move |_| {
        tx_frontend
            .send(Message {
                prefix: None,
                command: LIST.to_string(),
                params: vec![],
            })
            .map_err(|_| -> ClientError {
                ClientError {
                    kind: NONCRITICAL.to_string(),
                    message: SEND_MESSAGE.to_string(),
                }
            })
            .ok();
    });
}

///
/// Setup button "Send"
/// When clicked, it sends a message to the client with two possible commands:
/// If the message does not start with a slash, it sends a PRIVMSG command
/// If a message starts with a /, it sends a /COMMAND command
///
fn setup_send_button(
    client: &Client,
    tx_frontend: Sender<Message>,
    tx_backend: gtk::glib::Sender<Message>,
    nickname: &str,
) {
    let send_button: gtk::Button = client
        .builder
        .object("send_button")
        .expect("Couldn't get send_button");
    let message_entry: gtk::Entry = client
        .builder
        .object("message_entry")
        .expect("Couldn't get message_entry");
    let text_view: gtk::TextView = client
        .builder
        .object("chat_text")
        .expect("Couldn't get chat_text");
    let users_clone = client.online_chats_buffers.clone();
    let dcc_chats_clone = client.dcc_chats.clone();
    let nickname_clone = nickname.to_owned();
    send_button.connect_clicked(move |_| {
        let message = message_entry.text().to_string();
        let users = users_clone.lock().expect(LOCK_USERS).clone();

        let current_name_chat = find_user_by_current_buffer(users, &text_view); // Can be user or channel
        if !message.is_empty() {
            println!("Current name chat: {}", current_name_chat);
            if message.starts_with('/') {
                send_command(
                    dcc_chats_clone.clone(),
                    message.as_str(),
                    &tx_frontend,
                    current_name_chat,
                    &tx_backend,
                    &message_entry,
                    &nickname_clone,
                );
                return;
            }

            let dcc_chats = dcc_chats_clone.lock().expect(LOCK_DCC);

            let corresponding_sender = match dcc_chats.get(&current_name_chat) {
                Some(sender) => sender,
                None => &tx_frontend,
            };

            send_privmsg(
                corresponding_sender,
                &message,
                Some(message_entry.clone()),
                text_view.clone(),
                current_name_chat,
                true,
            );
        }
    });
}

///
/// This function searches for the user or channel that is currently being chatted with.
/// It returns the name of the user or channel
///
pub fn find_user_by_current_buffer(
    users: HashMap<String, TextBuffer>,
    text_view: &gtk::TextView,
) -> String {
    let mut user = "";
    for (key, val) in users.iter() {
        if val == &text_view.buffer().expect("Couldn't get buffer") {
            user = key;
            break;
        }
    }
    user.to_string()
}

///
/// This function sends a PRIVMSG command to the client.
///
pub fn send_privmsg(
    tx_frontend: &Sender<Message>,
    message: &String,
    message_entry: Option<gtk::Entry>,
    text_view: gtk::TextView,
    user_to_send: String,
    print_to_self_buffer: bool,
) {
    println!("User to send PRIVMSG: {}", user_to_send);
    // If the user is not in the self chat, send it
    if user_to_send != *"You".to_string() && !user_to_send.is_empty() {
        println!("Sending PRIVMSG");
        tx_frontend
            .send(Message {
                prefix: None,
                command: PRIVMSG.to_string(),
                params: vec![vec![user_to_send], vec![message.clone()]],
            })
            .map_err(|_| -> ClientError {
                ClientError {
                    kind: NONCRITICAL.to_string(),
                    message: SEND_MESSAGE.to_string(),
                }
            })
            .ok();
    }
    println!("end of send privmsg");

    if !print_to_self_buffer {
        return;
    };

    let text_to_print = format!("You: {}\r\n", message);
    let buffer = text_view.buffer().expect("Couldn't get buffer");
    buffer.insert(&mut buffer.end_iter(), &text_to_print);
    if let Some(entry) = message_entry {
        entry.delete_text(0, -1)
    }
}

///
/// This function parses and sends a command to the client.
///
pub fn send_command(
    dcc_chats: Arc<Mutex<HashMap<String, Sender<Message>>>>,
    message: &str,
    tx_frontend: &Sender<Message>,
    current_chat_name: String,
    tx_backend: &gtk::glib::Sender<Message>,
    message_entry: &gtk::Entry,
    nickname: &str,
) {
    let general_purpose_commands: HashSet<&str> =
        HashSet::from_iter(vec![AWAY, UNAWAY, WHOIS, OPER, QUIT, SQUIT]);
    let user_only_commands: HashSet<&str> = HashSet::from_iter(vec![DCC_CHAT, DCC_CLOSE]);
    let commands_with_messages: HashSet<&str> = HashSet::from_iter(vec![TOPIC, AWAY, SQUIT, QUIT]);
    let max_amount_params: HashMap<&str, usize> = HashMap::from_iter(vec![
        (AWAY, 1),
        (UNAWAY, 0),
        (WHOIS, 1),
        (OPER, 1),
        (QUIT, 1),
        (SQUIT, 2),
        (NAMES, 0),
        (TOPIC, 1),
        (PART, 0),
        (INVITE, 1),
        (KICK, 1),
        (MODE, 2),
        (DCC_CHAT, 0),
        (DCC_CLOSE, 0),
    ]);

    let space_index = parser::next_whitespace(message);

    //  Get the command
    let command = match space_index {
        Some(space_index) => message[1..space_index].to_uppercase(),
        None => message[1..].to_uppercase(),
    };

    //Check if the command is valid
    if !max_amount_params.contains_key(&command.as_str()) {
        let text_to_print = format!("'{}' is not a valid command", command);
        tx_backend
            .send(Message {
                prefix: None,
                command: RECEIVED_MESSAGE.to_string(),
                params: vec![vec![text_to_print, ERROR.to_string()]],
            })
            .map_err(|_| -> ClientError {
                ClientError {
                    kind: NONCRITICAL.to_string(),
                    message: SEND_MESSAGE.to_string(),
                }
            })
            .ok();
        return;
    }

    // Get the params
    let params_str = match space_index {
        Some(space_index) => &message[space_index + 1..],
        None => "",
    };

    // Check if the user tried to separate the params with a comma, which is not allowed
    if params_str.contains(',') {
        tx_backend
            .send(Message {
                prefix: None,
                command: RECEIVED_MESSAGE.to_string(),
                params: vec![vec![
                    "Please separate the parameters with spaces".to_string(),
                    ERROR.to_string(),
                ]],
            })
            .map_err(|_| -> ClientError {
                ClientError {
                    kind: NONCRITICAL.to_string(),
                    message: SEND_MESSAGE.to_string(),
                }
            })
            .ok();
    }

    // Check if the command has a message (If it contains a single parameter with spaces)
    let params =
        match params_str.contains(' ') && (commands_with_messages.contains(command.as_str())) {
            true => vec![params_str.to_string()], // If the command has a single parameter with spaces, don't split it
            false => {
                // Else, split it and put it in a vector
                let mut params = vec![];
                for param in params_str.split(' ') {
                    if !param.is_empty() {
                        params.push(param.to_string());
                    }
                }
                params
            }
        };

    message_entry.delete_text(0, -1);

    //Check if there are too many parameters
    let max_amount_current_param = max_amount_params
        .get(&command.as_str())
        .expect("Command not found");
    if &params.len() > max_amount_current_param {
        let text_to_print = match max_amount_current_param {
            0 => format!(
                "Too many parameters, '{}' doesn't take any parameters",
                command
            ),
            1 => format!(
                "Too many parameters, '{}' only takes one parameter at max",
                command
            ),
            _ => format!(
                "Too many parameters, '{}' only takes {} parameters at max",
                command, max_amount_current_param
            ),
        };
        tx_backend
            .send(Message {
                prefix: None,
                command: RECEIVED_MESSAGE.to_string(),
                params: vec![vec![text_to_print, ERROR.to_string()]],
            })
            .map_err(|_| -> ClientError {
                ClientError {
                    kind: NONCRITICAL.to_string(),
                    message: SEND_MESSAGE.to_string(),
                }
            })
            .ok();
        return;
    }

    // General purpose commands
    send_general_purpose_commands(&command, &params, tx_frontend, tx_backend, nickname);

    // If not in a channel chat, don't continue, next commands are only for channel chats
    if !(current_chat_name.starts_with('#') || current_chat_name.starts_with('&')) {
        if !general_purpose_commands.contains(command.as_str())
            && !user_only_commands.contains(command.as_str())
        {
            let text_to_print = format!("'{}' can only be used in channels", command);
            tx_backend
                .send(Message {
                    prefix: None,
                    command: RECEIVED_MESSAGE.to_string(),
                    params: vec![vec![text_to_print, ERROR.to_string()]],
                })
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: NONCRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })
                .ok();
        }
        if user_only_commands.contains(command.as_str()) {
            // Send user only commands
            send_user_only_commands(
                dcc_chats,
                &command,
                tx_frontend,
                tx_backend,
                current_chat_name,
            );
        }
        return;
    }

    // Channel-exclusive commands
    send_channel_only_commands(
        &command,
        params,
        tx_frontend,
        tx_backend,
        current_chat_name,
        nickname,
    );
}

///
/// For commands that can only be used by users
/// Builds the message to send depending on the command and sends it to the client
///
pub fn send_user_only_commands(
    dcc_chats: Arc<Mutex<HashMap<String, Sender<Message>>>>,
    command: &str,
    tx_frontend: &Sender<Message>,
    tx_backend: &gtk::glib::Sender<Message>,
    user_to_send: String,
) {
    match command {
        DCC_CHAT => {
            let listener = TcpListener::bind("0.0.0.0:0").unwrap();
            // Spawn new thread for the client
            let addres = listener.local_addr().unwrap();
            println!("Listening on {}", addres);
            let tx_backend_clone = tx_backend.clone();
            let user_to_send_clone = user_to_send.clone();
            let _ = thread::spawn(move || {
                // connection succeeded
                match wait_connection_dcc_chat(
                    listener,
                    tx_backend_clone,
                    dcc_chats,
                    user_to_send_clone,
                ) {
                    Ok(_) => println!("Private connection ended"),
                    Err(err) => println!("Error: {}", err),
                }
            });
            let message = format!("DCC_CHAT chat {} {}", addres.ip(), addres.port());
            println!("Sending message DCC_CHAT");
            tx_frontend
                .send(Message {
                    prefix: None,
                    command: PRIVMSG.to_string(),
                    params: vec![vec![user_to_send], vec![message]],
                })
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: NONCRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })
                .ok();
        }
        DCC_CLOSE => {
            let mut dcc_chats = dcc_chats.lock().unwrap();
            let sender = dcc_chats.remove(&user_to_send);
            drop(dcc_chats);
            if let Some(sender) = sender {
                let _ = sender.send(Message {
                    prefix: None,
                    command: DCC_CLOSE.to_string(),
                    params: vec![],
                });
            }
            tx_frontend
                .send(Message {
                    prefix: None,
                    command: PRIVMSG.to_string(),
                    params: vec![vec![user_to_send], vec![DCC_CLOSE.to_string()]],
                })
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: NONCRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })
                .ok();
        }
        _ => {}
    };
}

///
/// Waits for 10 seconds the acceptance of dcc chat connection. If the dcc chat connection is
/// accepted whitin 10 seconds, then handle_dcc_chat is called. Whether the connection is accepted
/// or not, the client is notified so the ui is updated.
///
pub fn wait_connection_dcc_chat(
    listener: TcpListener,
    tx_backend: gtk::glib::Sender<Message>,
    dcc_chats: Arc<Mutex<HashMap<String, Sender<Message>>>>,
    user_to_send: String,
) -> Result<(), ClientError> {
    listener.set_nonblocking(true).map_err(|_| -> ClientError {
        ClientError {
            kind: NONCRITICAL.to_string(),
            message: SEND_MESSAGE.to_string(),
        }
    })?;

    let mut i = 0;
    let one_second = std::time::Duration::from_millis(1000);
    let mut stream: Option<TcpStream> = None;

    // Try to accept connection for 10 seconds
    while i < 10 {
        match listener.accept() {
            Ok(result) => {
                stream = Some(result.0);
                break;
            }
            Err(error) => {
                if error.kind() == ErrorKind::WouldBlock {
                    i += 1;
                    thread::sleep(one_second);
                    continue;
                } else {
                    break;
                }
            }
        };
    }

    // If stream is None then the connection was rejected
    if stream.is_none() {
        tx_backend
            .send(Message {
                prefix: None,
                command: RECEIVED_MESSAGE.to_string(),
                params: vec![vec![
                    format!("The connection wasn't accepted"),
                    INFO.to_string(),
                ]],
            })
            .map_err(|_| -> ClientError {
                ClientError {
                    kind: NONCRITICAL.to_string(),
                    message: SEND_MESSAGE.to_string(),
                }
            })?;
    } else {
        println!("Connection accepted");
        // If stream exists then the connection was accepted
        let mut dcc_chats = dcc_chats.lock().unwrap();
        let (tx, rx) = channel();
        dcc_chats.insert(user_to_send.clone(), tx);
        println!("DCC CHAT {:?}", dcc_chats);
        drop(dcc_chats);

        // Start reading and writing messages in dcc_chat
        handle_dcc_chat(stream.unwrap(), tx_backend, rx, &user_to_send)?;
    }

    Ok(())
}

///
/// For commands that can be used in any chat
/// Builds the message to send depending on the command and sends it to the client
///
pub fn send_general_purpose_commands(
    command: &str,
    params: &Vec<String>,
    tx_frontend: &Sender<Message>,
    tx_backend: &gtk::glib::Sender<Message>,
    nickname: &str,
) {
    match command {
        AWAY => {
            tx_frontend
                .send(Message {
                    prefix: None,
                    command: AWAY.to_string(),
                    params: vec![params.clone()],
                })
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: NONCRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })
                .ok();
        }
        UNAWAY => {
            tx_frontend
                .send(Message {
                    prefix: None,
                    command: AWAY.to_string(),
                    params: vec![],
                })
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: NONCRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })
                .ok();
        }
        WHOIS => {
            let params_to_send = match params.len() {
                1 => vec![vec![params[0].clone()]], // If it only has one parameter, it's the nickname of the user to get info from
                2 => vec![vec![params[0].clone()], vec![params[1].clone()]], // If it has two parameters, it's the server to get it the info from and the nickname of the user
                _ => vec![vec![]], // Error, send an empty vector
            };
            tx_frontend
                .send(Message {
                    prefix: None,
                    command: WHOIS.to_string(),
                    params: params_to_send,
                })
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: NONCRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })
                .ok();
        }
        OPER => {
            let params_to_send = match params.len() {
                0 => vec![vec![nickname.to_string()]], // If it has no parameters, send only the nickname
                1 => vec![vec![nickname.to_string()], vec![params[0].clone()]], // If it has one parameter, send the nickname and the password
                _ => vec![vec![]], // If it has more than one parameters, it's an error
            };
            tx_frontend
                .send(Message {
                    prefix: None,
                    command: OPERATOR.to_string(),
                    params: params_to_send,
                })
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: NONCRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })
                .ok();
        }
        QUIT => {
            tx_frontend
                .send(Message {
                    prefix: None,
                    command: QUIT.to_string(),
                    params: vec![params.clone()],
                })
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: NONCRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })
                .ok();
            tx_backend
                .send(Message {
                    prefix: None,
                    command: QUIT.to_string(),
                    params: vec![],
                })
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: NONCRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })
                .ok();
        }
        SQUIT => {
            tx_frontend
                .send(Message {
                    prefix: None,
                    command: SQUIT.to_string(),
                    params: vec![params.clone()],
                })
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: NONCRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })
                .ok();
        }
        _ => {}
    }
}

///
/// For commands that can only be used in channels
/// Builds the message to send depending on the command and sends it to the client
///
pub fn send_channel_only_commands(
    command: &str,
    params: Vec<String>,
    tx_frontend: &Sender<Message>,
    tx_backend: &gtk::glib::Sender<Message>,
    current_chat_name: String,
    nickname: &str,
) {
    match command {
        NAMES => {
            tx_frontend
                .send(Message {
                    prefix: None,
                    command: NAMES.to_string(),
                    params: vec![vec![current_chat_name]],
                })
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: NONCRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })
                .ok();
        }
        TOPIC => match params.is_empty() {
            true => {
                tx_frontend
                    .send(Message {
                        prefix: None,
                        command: TOPIC.to_string(),
                        params: vec![vec![current_chat_name]],
                    })
                    .map_err(|_| -> ClientError {
                        ClientError {
                            kind: NONCRITICAL.to_string(),
                            message: SEND_MESSAGE.to_string(),
                        }
                    })
                    .ok();
            }
            false => {
                tx_frontend
                    .send(Message {
                        prefix: None,
                        command: TOPIC.to_string(),
                        params: vec![vec![current_chat_name], params],
                    })
                    .map_err(|_| -> ClientError {
                        ClientError {
                            kind: NONCRITICAL.to_string(),
                            message: SEND_MESSAGE.to_string(),
                        }
                    })
                    .ok();
            }
        },
        PART => part_channel(tx_frontend, current_chat_name, tx_backend),
        INVITE => {
            tx_frontend
                .send(Message {
                    prefix: None,
                    command: INVITE.to_string(),
                    params: vec![params, vec![current_chat_name]],
                })
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: NONCRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })
                .ok();
        }
        KICK => {
            if params[0] == nickname {
                let text_to_print =
                    "You can't kick yourself from a channel, consider using '/part' instead"
                        .to_string();
                tx_backend
                    .send(Message {
                        prefix: None,
                        command: RECEIVED_MESSAGE.to_string(),
                        params: vec![vec![text_to_print, ERROR.to_string()]],
                    })
                    .map_err(|_| -> ClientError {
                        ClientError {
                            kind: NONCRITICAL.to_string(),
                            message: SEND_MESSAGE.to_string(),
                        }
                    })
                    .ok();
                return;
            }
            tx_frontend
                .send(Message {
                    prefix: None,
                    command: KICK.to_string(),
                    params: vec![vec![current_chat_name], params],
                })
                .map_err(|_| -> ClientError {
                    ClientError {
                        kind: NONCRITICAL.to_string(),
                        message: SEND_MESSAGE.to_string(),
                    }
                })
                .ok();
        }
        MODE => {
            if params.is_empty() {
                tx_backend
                    .send(Message {
                        prefix: None,
                        command: RECEIVED_MESSAGE.to_string(),
                        params: vec![vec![
                            "You must specify a mode".to_string(),
                            ERROR.to_string(),
                        ]],
                    })
                    .map_err(|_| -> ClientError {
                        ClientError {
                            kind: NONCRITICAL.to_string(),
                            message: SEND_MESSAGE.to_string(),
                        }
                    })
                    .ok();
            } else {
                match params.len() >= 2 {
                    // Some modes have parameters and others no
                    true => {
                        tx_frontend
                            .send(Message {
                                prefix: None,
                                command: MODE.to_string(),
                                params: vec![
                                    vec![current_chat_name],
                                    vec![params[0].clone()],
                                    vec![params[1].clone()],
                                ],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })
                            .ok();
                    }
                    false => {
                        tx_frontend
                            .send(Message {
                                prefix: None,
                                command: MODE.to_string(),
                                params: vec![vec![current_chat_name], vec![params[0].clone()]],
                            })
                            .map_err(|_| -> ClientError {
                                ClientError {
                                    kind: NONCRITICAL.to_string(),
                                    message: SEND_MESSAGE.to_string(),
                                }
                            })
                            .ok();
                    }
                }
            }
        }
        _ => println!("Command not found"),
    }
}

///
/// Setup button "Search" in the user submenu
/// When clicked, it sends a WHO command to the client
///
pub fn setup_search_user_button(client: &Client, tx_frontend: Sender<Message>) {
    let search_button: gtk::Button = client
        .builder
        .object("search_user_button")
        .expect("Couldn't get send_button");
    let search_entry: gtk::Entry = client
        .builder
        .object("search_user_entry")
        .expect("Couldn't get search_entry");

    search_button.connect_clicked(move |_| {
        tx_frontend
            .send(Message {
                prefix: None,
                command: WHO.to_string(),
                params: vec![vec![search_entry.text().to_string()]],
            })
            .map_err(|_| -> ClientError {
                ClientError {
                    kind: NONCRITICAL.to_string(),
                    message: SEND_MESSAGE.to_string(),
                }
            })
            .ok();
    });
}

///
/// This function creates a new button with the nickname of the user
/// Returns the new button
/// When the button is clicked, it shows the chat with the corresponding user
///
pub fn new_user_chat_button(client: &Client, name: &str, buffer: TextBuffer) -> gtk::Button {
    let button = gtk::Button::with_label(name);
    let text_view: gtk::TextView = client
        .builder
        .object("chat_text")
        .expect("Couldn't get chat_text");
    let message_entry: gtk::Entry = client
        .builder
        .object("message_entry")
        .expect("Couldn't get message_entry");

    let user_button_clicked = move |button: &gtk::Button| {
        // Obtain user nickname looking at buttons label
        let chat_name = button
            .label()
            .expect("Couldn't get button label")
            .to_string();
        // Set placeholer text of entry
        message_entry
            .set_placeholder_text(Some(&format!("Type your message to {} here", chat_name)));
        //Set the text_view buffer to that
        text_view.set_buffer(Some(&buffer));
    };
    button.connect_clicked(user_button_clicked);
    button
}

///
/// This function creates a new button with the nickname of the user
/// Returns the new button
/// When the button is clicked, it sends a message to create a new chat with the corresponding user
///
pub fn new_user_search_button(
    name: String,
    tx_backend: &gtk::glib::Sender<Message>,
) -> gtk::Button {
    let button = gtk::Button::with_label(&name);

    let tx_backend_clone = tx_backend.clone();
    let user_button_clicked = move |_: &gtk::Button| {
        let name_string = name.clone();
        tx_backend_clone
            .send(Message {
                prefix: Some(name_string),
                command: ADD_LIST_CHATS.to_string(),
                params: vec![vec![]],
            })
            .map_err(|_| -> ClientError {
                ClientError {
                    kind: NONCRITICAL.to_string(),
                    message: SEND_MESSAGE.to_string(),
                }
            })
            .ok();
    };
    button.connect_clicked(user_button_clicked);
    button
}

///
/// Prints a message in the chat window and sends the PART or PART_CHANNEL command to the client
///
pub fn part_channel(
    tx_frontend: &Sender<Message>,
    channel: String,
    tx_backend: &gtk::glib::Sender<Message>,
) {
    let text_to_print = "left the channel.".to_string();
    tx_frontend
        .send(Message {
            prefix: None,
            command: PRIVMSG.to_string(),
            params: vec![vec![channel.clone()], vec![text_to_print]],
        })
        .map_err(|_| -> ClientError {
            ClientError {
                kind: NONCRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            }
        })
        .ok();
    tx_frontend
        .send(Message {
            prefix: None,
            command: PART.to_string(),
            params: vec![vec![channel.clone()]],
        })
        .map_err(|_| -> ClientError {
            ClientError {
                kind: NONCRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            }
        })
        .ok();

    let text_to_print = format!("You left the channel {}", channel);
    tx_backend
        .send(Message {
            prefix: Some("You".to_string()),
            command: RECEIVED_MESSAGE.to_string(),
            params: vec![vec![text_to_print, INFO.to_string()]],
        })
        .map_err(|_| -> ClientError {
            ClientError {
                kind: NONCRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            }
        })
        .ok();
    tx_backend
        .send(Message {
            prefix: None,
            command: PART_CHANNEL.to_string(),
            params: vec![vec![channel]],
        })
        .map_err(|_| -> ClientError {
            ClientError {
                kind: NONCRITICAL.to_string(),
                message: SEND_MESSAGE.to_string(),
            }
        })
        .ok();
}

///
/// Setup "Send file" button
/// When clicked, it sends DCC_SEND with the correspinging parameters to start a file transfer
///
pub fn setup_send_file_button(
    client: &Client,
    tx_frontend: Sender<Message>,
    tx_backend: gtk::glib::Sender<Message>,
) {
    let file_chooser: gtk::FileChooserButton = client
        .builder
        .object("file_chooser")
        .expect("Couldn't get file_chooser");
    let send_file_button: gtk::Button = client
        .builder
        .object("send_file_button")
        .expect("Couldn't get send_file_button");
    let text_view: gtk::TextView = client
        .builder
        .object("chat_text")
        .expect("Couldn't get chat_text");
    let users_clone = client.online_chats_buffers.clone();
    let file_paths = client.dcc_file_paths.clone();
    send_file_button.connect_clicked(move |_| {
        println!("Send file button clicked");
        let file_path = match file_chooser.filename() {
            Some(name) => name,
            None => {
                tx_backend
                    .send(Message {
                        prefix: None,
                        command: RECEIVED_MESSAGE.to_string(),
                        params: vec![vec!["Please select a file".to_string(), INFO.to_string()]],
                    })
                    .unwrap();
                return;
            }
        };
        let file_path_clone = file_path.clone();
        let file_name = file_path_clone.to_str().expect("Couldn't get file name");
        let file_name = file_name.split('/').last().expect("Couldn't get file name");
        let name = &file_name.replace(' ', "_");
        println!("File name: {}", name);
        let users = users_clone.lock().expect(LOCK_USERS).clone();
        let current_name_chat = find_user_by_current_buffer(users, &text_view);

        // Save name and path of the file to the hashmap
        file_paths
            .lock()
            .unwrap()
            .insert(name.to_string(), file_path.clone());

        println!("Current name chat: {}", current_name_chat);
        let listener = TcpListener::bind("0.0.0.0:0").unwrap();
        // Spawn new thread for the client
        let address = listener.local_addr().unwrap();
        println!("Listening on {}", address);
        let tx_backend_clone = tx_backend.clone();
        let current_name_chat_clone = current_name_chat.clone();
        let file_size = file_path.metadata().unwrap().len();
        let _ = thread::spawn(move || {
            // connection succeeded
            match wait_connection_dcc_file(
                listener,
                tx_backend_clone,
                current_name_chat_clone,
                file_path,
                0,
            ) {
                Ok(_) => println!("Private connection ended"),
                Err(err) => println!("Error: {}", err),
            }
        });

        let message = format!(
            "DCC_SEND {} {} {} {}",
            name,
            address.ip(),
            address.port(),
            file_size
        );
        println!("Sending message: {}", message);
        send_privmsg(
            &tx_frontend, //start chat and thread to send file
            &message,
            None,
            text_view.clone(),
            current_name_chat,
            false,
        );
    });
}

///
/// Waits for 10 seconds the acceptance of dcc send. If the connection is
/// accepted whitin 10 seconds, then handle_dcc_file_send is called. Whether the connection is accepted
/// or not, the client is notified so the ui is updated.
///
pub fn wait_connection_dcc_file(
    listener: TcpListener,
    tx_backend: gtk::glib::Sender<Message>,
    user_to_send: String,
    file_path: PathBuf,
    start_position: u64,
) -> Result<(), String> {
    println!("Waiting for connection");

    listener.set_nonblocking(true).unwrap();

    let mut i = 0;
    let one_second = std::time::Duration::from_millis(1000);
    let mut stream: Option<TcpStream> = None;

    // Try to accept connection for 10 seconds
    while i < 10 {
        match listener.accept() {
            Ok(result) => {
                stream = Some(result.0);
                break;
            }
            Err(error) => {
                if error.kind() == ErrorKind::WouldBlock {
                    i += 1;
                    thread::sleep(one_second);
                    continue;
                } else {
                    break;
                }
            }
        };
    }

    match stream {
        None => {
            tx_backend
                .send(Message {
                    prefix: None,
                    command: RECEIVED_MESSAGE.to_string(),
                    params: vec![vec![
                        "The other user rejected the transfer".to_string(),
                        INFO.to_string(),
                    ]],
                })
                .unwrap();
            Err("Connection rejected".to_string())
        }
        Some(_) => {
            handle_dcc_file_send(
                stream.unwrap(),
                tx_backend,
                &user_to_send,
                file_path,
                start_position,
            );
            Ok(())
        }
    }
}

///
/// Setup the button to resume a file transfer
/// If the button is clicked, send a DCC_RESUME message to the user
///
pub fn setup_resume_transfer_button(client: &Client, tx_frontend: Sender<Message>) {
    let file_chooser: gtk::FileChooserButton = client
        .builder
        .object("file_chooser")
        .expect("Couldn't get file_chooser");
    let resume_file_button: gtk::Button = client
        .builder
        .object("resume_file_button")
        .expect("Couldn't get resume_file_button");
    let text_view: gtk::TextView = client
        .builder
        .object("chat_text")
        .expect("Couldn't get chat_text");
    let users_clone = client.online_chats_buffers.clone();
    resume_file_button.connect_clicked(move |_| {
        println!("Resume file button clicked");
        let file_path = match file_chooser.filename() {
            Some(name) => name,
            None => return,
        };
        println!("File path: {:?}", file_path);
        let file_path_clone = file_path.clone();
        let file_path_str = file_path_clone.to_str().expect("Couldn't get file name");

        let mut file_path_splitted = file_path_str.split('/');

        let file_name = file_path_splitted
            .clone()
            .last()
            .expect("Couldn't get file name");

        let directory = file_path_splitted.nth_back(1).unwrap();
        if directory == "received_files" {
            println!("File already in received_files");
            let users = users_clone.lock().expect(LOCK_USERS).clone();
            let current_name_chat = find_user_by_current_buffer(users, &text_view);

            let file = File::open(file_path).unwrap();
            let file_size = file.metadata().unwrap().len();

            let message = format!("DCC_RESUME {} 0.0.0.0 0 {}", file_name, file_size);
            println!("Sending message: {}", message);

            send_privmsg(
                &tx_frontend, //start chat and thread to send file
                &message,
                None,
                text_view.clone(),
                current_name_chat,
                false,
            );
        }
    });
}

///
/// Setup the button to pause a file transfer
/// If the button is clicked, send a DCC_PAUSE message to the user
///
pub fn setup_pause_transfer_button(client: &Client) {
    let pause_file_button: gtk::Button = client
        .builder
        .object("pause_file_button")
        .expect("Couldn't get pause_file_button");
    let text_view: gtk::TextView = client
        .builder
        .object("chat_text")
        .expect("Couldn't get chat_text");
    let users_clone = client.online_chats_buffers.clone();
    let dcc_chats_clone = client.dcc_chats.clone();
    pause_file_button.connect_clicked(move |_| {
        println!("Pause file button clicked");
        let users = users_clone.lock().expect(LOCK_USERS).clone();
        let current_name_chat = find_user_by_current_buffer(users, &text_view);
        let mut chats = dcc_chats_clone.lock().unwrap();

        let sender = match chats.remove(&format!("{}_f", current_name_chat)) {
            Some(sender) => sender,
            None => {
                println!("No chat with that user");
                return;
            }
        };
        let message = Message {
            prefix: None,
            command: PAUSE.to_string(),
            params: vec![vec![]],
        };
        sender.send(message).unwrap();
        println!("Pause sent");
    });
}
