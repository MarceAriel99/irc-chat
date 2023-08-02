//!
//! INTEGRATION TESTS OF SERVER
//!
//! WARNING! this tests must be run one by one (without cargo test)
//!

use irc::{
    commands::PRIVMSG,
    message::Message,
    numeric_reply::{
        NumericReply, ERR_BADCHANNELKEY_MSG, ERR_BADCHANNELKEY_NUM, ERR_CHANNELHASKEY_MSG,
        ERR_CHANNELHASKEY_NUM, ERR_CHANOPRIVSNEEDED_MSG, ERR_CHANOPRIVSNEEDED_NUM,
        ERR_INVITEONLYCHAN_MSG, ERR_INVITEONLYCHAN_NUM, RPL_INVITING_NUM, RPL_MODESET_MSG,
        RPL_MODESET_NUM, RPL_NOTOPIC_MSG, RPL_NOTOPIC_NUM, RPL_TOPIC_NUM,
    },
};
use std::{
    io::{BufRead, BufReader, Write},
    thread,
    time::Duration,
};

use crate::common::*;
mod common;

#[test]
fn user_can_login_correctly() {
    let _handle = thread::spawn(move || {
        let result = run_server();
        assert!(result.is_ok());
    });

    std::thread::sleep(Duration::new(5, 0));

    login_ari();
}

#[test]
fn user_can_send_private_message_correctly() {
    let _handle = thread::spawn(move || {
        let result = run_server();
        assert!(result.is_ok());
    });

    std::thread::sleep(Duration::new(5, 0));

    let mut socket_ari = login_ari();

    let socket_juani = login_juani();

    std::thread::sleep(Duration::new(5, 0));

    let priv_msg_message = "PRIVMSG juanireil :Hola juani\r\n";
    let result = socket_ari.write_all(priv_msg_message.as_bytes());
    assert!(result.is_ok());

    let mut reader = BufReader::new(socket_juani);

    let mut data = String::new();
    let result = reader.read_line(&mut data);
    assert!(result.is_ok());

    let message = Message {
        prefix: Some("ari".to_string()),
        command: PRIVMSG.to_string(),
        params: vec![
            vec!["juanireil".to_string()],
            vec!["Hola juani".to_string()],
        ],
    };

    assert_eq!(message.as_string(), data)
}

#[test]
fn user_can_join_and_communicate_in_channels_correctly() {
    let _handle = thread::spawn(move || {
        let result = run_server();
        assert!(result.is_ok());
    });

    std::thread::sleep(Duration::new(5, 0));

    let mut socket_ari = login_ari();
    let mut socket_juani = login_juani();

    let mut data = String::new();
    let mut reader_ari = BufReader::new(socket_ari.try_clone().unwrap());
    let mut reader_juani = BufReader::new(socket_juani.try_clone().unwrap());

    let reply = NumericReply::new(
        RPL_NOTOPIC_NUM,
        RPL_NOTOPIC_MSG,
        Some(vec!["#canal".to_string()]),
    )
    .as_string();

    std::thread::sleep(Duration::new(5, 0));

    let join_message = "JOIN #canal\r\n";
    let result = socket_ari.write_all(join_message.as_bytes());
    assert!(result.is_ok());

    let result = reader_ari.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, reply);
    data.clear();

    let join_message = "JOIN #canal\r\n";
    let result = socket_juani.write_all(join_message.as_bytes());
    assert!(result.is_ok());

    let result = reader_juani.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, reply);
    data.clear();

    let priv_msg_message = "PRIVMSG #canal :Hola grupo\r\n";
    let result = socket_juani.write_all(priv_msg_message.as_bytes());
    assert!(result.is_ok());

    let result = reader_ari.read_line(&mut data);
    assert!(result.is_ok());

    let message = Message {
        prefix: Some("juanireil".to_string()),
        command: PRIVMSG.to_string(),
        params: vec![vec!["#canal".to_string()], vec!["Hola grupo".to_string()]],
    };

    assert_eq!(message.as_string(), data)
}

#[test]
fn user_can_join_and_set_mode_to_invite_and_other_user_cant_join_without_invitation() {
    let _handle = thread::spawn(move || {
        let result = run_server();
        assert!(result.is_ok());
    });

    std::thread::sleep(Duration::new(5, 0));

    let mut socket_ari = login_ari();
    let mut socket_juani = login_juani();

    let mut data = String::new();
    let mut reader_ari = BufReader::new(socket_ari.try_clone().unwrap());
    let mut reader_juani = BufReader::new(socket_juani.try_clone().unwrap());

    let no_topic_reply = NumericReply::new(
        RPL_NOTOPIC_NUM,
        RPL_NOTOPIC_MSG,
        Some(vec!["#canal".to_string()]),
    )
    .as_string();

    std::thread::sleep(Duration::new(5, 0));

    let join_message = "JOIN #canal\r\n";
    let result = socket_ari.write_all(join_message.as_bytes());
    assert!(result.is_ok());

    let result = reader_ari.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, no_topic_reply);
    data.clear();

    let mode_message = "MODE #canal +i\r\n";
    let result = socket_ari.write_all(mode_message.as_bytes());
    assert!(result.is_ok());

    let mode_set_reply = NumericReply::new(
        RPL_MODESET_NUM,
        RPL_MODESET_MSG,
        Some(vec!["#canal".to_string(), "+i".to_string()]),
    )
    .as_string();

    let result = reader_ari.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, mode_set_reply);
    data.clear();

    let join_message = "JOIN #canal\r\n";
    let result = socket_juani.write_all(join_message.as_bytes());
    assert!(result.is_ok());

    let invite_only_err = NumericReply::new(
        ERR_INVITEONLYCHAN_NUM,
        ERR_INVITEONLYCHAN_MSG,
        Some(vec!["#canal".to_string()]),
    )
    .as_string();

    let result = reader_juani.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, invite_only_err);
}

#[test]
fn user_can_join_and_set_mode_to_invite_and_can_invite_other_user_and_it_can_join() {
    let _handle = thread::spawn(move || {
        let result = run_server();
        assert!(result.is_ok());
    });

    std::thread::sleep(Duration::new(5, 0));

    let mut socket_ari = login_ari();
    let mut socket_juani = login_juani();

    let mut data = String::new();
    let mut reader_ari = BufReader::new(socket_ari.try_clone().unwrap());
    let mut reader_juani = BufReader::new(socket_juani.try_clone().unwrap());

    let no_topic_reply = NumericReply::new(
        RPL_NOTOPIC_NUM,
        RPL_NOTOPIC_MSG,
        Some(vec!["#canal".to_string()]),
    )
    .as_string();

    std::thread::sleep(Duration::new(5, 0));

    let join_message = "JOIN #canal\r\n";
    let result = socket_ari.write_all(join_message.as_bytes());
    assert!(result.is_ok());

    let result = reader_ari.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, no_topic_reply);
    data.clear();

    let mode_message = "MODE #canal +i\r\n";
    let result = socket_ari.write_all(mode_message.as_bytes());
    assert!(result.is_ok());

    let mode_set_reply = NumericReply::new(
        RPL_MODESET_NUM,
        RPL_MODESET_MSG,
        Some(vec!["#canal".to_string(), "+i".to_string()]),
    )
    .as_string();

    let result = reader_ari.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, mode_set_reply);
    data.clear();

    let invite_message = "INVITE juanireil #canal\r\n";
    let result = socket_ari.write_all(invite_message.as_bytes());
    assert!(result.is_ok());

    let invite_reply = NumericReply::new(
        RPL_INVITING_NUM,
        "",
        Some(vec!["#canal".to_string(), "juanireil".to_string()]),
    )
    .as_string();

    let result = reader_ari.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, invite_reply);
    data.clear();

    let join_message = "JOIN #canal\r\n";
    let result = socket_juani.write_all(join_message.as_bytes());
    assert!(result.is_ok());

    let result = reader_juani.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, no_topic_reply);
}

#[test]
fn user_can_join_and_set_key_and_other_users_cant_join_without_it() {
    let _handle = thread::spawn(move || {
        let result = run_server();
        assert!(result.is_ok());
    });

    std::thread::sleep(Duration::new(5, 0));

    let mut socket_ari = login_ari();
    let mut socket_juani = login_juani();

    let mut data = String::new();
    let mut reader_ari = BufReader::new(socket_ari.try_clone().unwrap());
    let mut reader_juani = BufReader::new(socket_juani.try_clone().unwrap());

    let no_topic_reply = NumericReply::new(
        RPL_NOTOPIC_NUM,
        RPL_NOTOPIC_MSG,
        Some(vec!["#canal".to_string()]),
    )
    .as_string();

    std::thread::sleep(Duration::new(5, 0));

    let join_message = "JOIN #canal\r\n";
    let result = socket_ari.write_all(join_message.as_bytes());
    assert!(result.is_ok());

    let result = reader_ari.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, no_topic_reply);
    data.clear();

    let mode_message = "MODE #canal +k pass\r\n";
    let result = socket_ari.write_all(mode_message.as_bytes());
    assert!(result.is_ok());

    let mode_set_reply = NumericReply::new(
        RPL_MODESET_NUM,
        RPL_MODESET_MSG,
        Some(vec!["#canal".to_string(), "+k".to_string()]),
    )
    .as_string();

    let result = reader_ari.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, mode_set_reply);
    data.clear();

    let join_message = "JOIN #canal\r\n";
    let result = socket_juani.write_all(join_message.as_bytes());
    assert!(result.is_ok());

    let key_set_err =
        NumericReply::new(ERR_CHANNELHASKEY_NUM, ERR_CHANNELHASKEY_MSG, None).as_string();

    let result = reader_juani.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, key_set_err);
    data.clear();

    let join_message = "JOIN #canal incorrectpass\r\n";
    let result = socket_juani.write_all(join_message.as_bytes());
    assert!(result.is_ok());

    let bad_key_err = NumericReply::new(
        ERR_BADCHANNELKEY_NUM,
        ERR_BADCHANNELKEY_MSG,
        Some(vec!["#canal".to_string()]),
    )
    .as_string();

    let result = reader_juani.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, bad_key_err);
}

#[test]
fn user_can_join_and_set_key_and_other_users_can_join_with_it_correctly() {
    let _handle = thread::spawn(move || {
        let result = run_server();
        assert!(result.is_ok());
    });

    std::thread::sleep(Duration::new(5, 0));

    let mut socket_ari = login_ari();
    let mut socket_juani = login_juani();

    let mut data = String::new();
    let mut reader_ari = BufReader::new(socket_ari.try_clone().unwrap());
    let mut reader_juani = BufReader::new(socket_juani.try_clone().unwrap());

    let no_topic_reply = NumericReply::new(
        RPL_NOTOPIC_NUM,
        RPL_NOTOPIC_MSG,
        Some(vec!["#canal".to_string()]),
    )
    .as_string();

    std::thread::sleep(Duration::new(5, 0));

    let join_message = "JOIN #canal\r\n";
    let result = socket_ari.write_all(join_message.as_bytes());
    assert!(result.is_ok());

    let result = reader_ari.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, no_topic_reply);
    data.clear();

    let mode_message = "MODE #canal +k pass\r\n";
    let result = socket_ari.write_all(mode_message.as_bytes());
    assert!(result.is_ok());

    let mode_set_reply = NumericReply::new(
        RPL_MODESET_NUM,
        RPL_MODESET_MSG,
        Some(vec!["#canal".to_string(), "+k".to_string()]),
    )
    .as_string();

    let result = reader_ari.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, mode_set_reply);
    data.clear();

    let join_message = "JOIN #canal pass\r\n";
    let result = socket_juani.write_all(join_message.as_bytes());
    assert!(result.is_ok());

    let result = reader_juani.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, no_topic_reply);
}

#[test]
fn user_can_set_and_ask_for_channel_topic_correctly_with_mode_topic_or_not() {
    let _handle = thread::spawn(move || {
        let result = run_server();
        assert!(result.is_ok());
    });

    std::thread::sleep(Duration::new(5, 0));

    let mut socket_ari = login_ari();
    let mut socket_juani = login_juani();

    let mut data = String::new();
    let mut reader_ari = BufReader::new(socket_ari.try_clone().unwrap());
    let mut reader_juani = BufReader::new(socket_juani.try_clone().unwrap());

    let reply = NumericReply::new(
        RPL_NOTOPIC_NUM,
        RPL_NOTOPIC_MSG,
        Some(vec!["#canal".to_string()]),
    )
    .as_string();

    std::thread::sleep(Duration::new(5, 0));

    let join_message = "JOIN #canal\r\n";
    let result = socket_ari.write_all(join_message.as_bytes());
    assert!(result.is_ok());

    let result = reader_ari.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, reply);
    data.clear();

    let join_message = "JOIN #canal\r\n";
    let result = socket_juani.write_all(join_message.as_bytes());
    assert!(result.is_ok());

    let result = reader_juani.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, reply);
    data.clear();

    let topic_message = "TOPIC #canal :Nuevo topic de juani\r\n";
    let result = socket_juani.write_all(topic_message.as_bytes());
    assert!(result.is_ok());

    let topic_reply = NumericReply::new(
        RPL_TOPIC_NUM,
        "Nuevo topic de juani",
        Some(vec!["#canal".to_string()]),
    )
    .as_string();

    let result = reader_juani.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, topic_reply);
    data.clear();

    let topic_message = "TOPIC #canal\r\n";
    let result = socket_ari.write_all(topic_message.as_bytes());
    assert!(result.is_ok());

    let result = reader_ari.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, topic_reply);
    data.clear();

    let mode_t_message = "MODE #canal +t\r\n";
    let result = socket_ari.write_all(mode_t_message.as_bytes());
    assert!(result.is_ok());

    let mode_set_reply = NumericReply::new(
        RPL_MODESET_NUM,
        RPL_MODESET_MSG,
        Some(vec!["#canal".to_string(), "+t".to_string()]),
    )
    .as_string();

    let result = reader_ari.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, mode_set_reply);
    data.clear();

    let topic_message = "TOPIC #canal :Nuevo topic de juani sin ser operador\r\n";
    let result = socket_juani.write_all(topic_message.as_bytes());
    assert!(result.is_ok());

    let op_err = NumericReply::new(
        ERR_CHANOPRIVSNEEDED_NUM,
        ERR_CHANOPRIVSNEEDED_MSG,
        Some(vec!["#canal".to_string()]),
    )
    .as_string();

    let result = reader_juani.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, op_err);
    data.clear();

    let topic_message = "TOPIC #canal\r\n";
    let result = socket_ari.write_all(topic_message.as_bytes());
    assert!(result.is_ok());

    let result = reader_ari.read_line(&mut data);
    assert!(result.is_ok());
    assert_eq!(data, topic_reply);
    data.clear();
}
