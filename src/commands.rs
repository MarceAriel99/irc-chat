//!
//! This module contains the available commands
//!

// CONNECTION AND REGISTRATION
pub const PASS: &str = "PASS";
pub const NICK: &str = "NICK";
pub const USER: &str = "USER";
pub const LOGIN: &str = "LOGIN";
pub const REGISTRATION: &str = "REGISTRATION";
pub const QUIT: &str = "QUIT";

// MESSAGES EXCHANGE
pub const PRIVMSG: &str = "PRIVMSG";
pub const NOTICE: &str = "NOTICE";

// CHANNELS
pub const JOIN: &str = "JOIN";
pub const NAMES: &str = "NAMES";
pub const LIST: &str = "LIST";
pub const PART: &str = "PART";
pub const INVITE: &str = "INVITE";
pub const MODE: &str = "MODE";
pub const KICK: &str = "KICK";
pub const TOPIC: &str = "TOPIC";

pub const WHO: &str = "WHO";
pub const WHOIS: &str = "WHOIS";

// modes
pub const MODE_SET_KEY: &str = "+k";
pub const MODE_REMOVE_KEY: &str = "-k";

pub const MODE_SET_LIMIT: &str = "+l";
pub const MODE_REMOVE_LIMIT: &str = "-l";

pub const MODE_SET_INVITE: &str = "+i";
pub const MODE_REMOVE_INVITE: &str = "-i";

pub const MODE_GIVE_OP_PRIVILEGES: &str = "+o";
pub const MODE_TAKE_OP_PRIVILEGES: &str = "-o";

pub const MODE_SET_OP_TOPIC: &str = "+t";
pub const MODE_REMOVE_OP_TOPIC: &str = "-t";

pub const MODE_SET_SECRET: &str = "+s";
pub const MODE_REMOVE_SECRET: &str = "-s";

pub const MODE_SET_BAN: &str = "+b";
pub const MODE_REMOVE_BAN: &str = "-b";

// FRONTEND COMMANDS
pub const RECEIVED_MESSAGE: &str = "RECEIVED_MESSAGE";
pub const ADD_LIST_CHATS: &str = "ADD_LIST_CHATS";
pub const LIST_CHANNELS: &str = "LIST_CHANNELS";
pub const SEARCH_USERS: &str = "SEARCH_USERS";
pub const CORRECT_LOGIN: &str = "CORRECT_LOGIN";
pub const INVALID_LOGIN: &str = "INVALID_LOGIN";
pub const INVALID_REGISTRATION: &str = "INVALID_REGISTRATION";
pub const CORRECT_REGISTRATION: &str = "CORRECT_REGISTRATION";
pub const USER_AWAY: &str = "USER_AWAY";
pub const ERROR_CHANNEL: &str = "ERROR_CHANNEL";
pub const CONNECTION_ATTEMPT: &str = "CONNECTION_ATTEMPT";
pub const DCC_CHAT: &str = "DCC_CHAT";
pub const DCC_SEND: &str = "DCC_SEND";
pub const DCC_CLOSE: &str = "DCC_CLOSE";
pub const DCC_RESUME: &str = "DCC_RESUME";
pub const DCC_ACCEPT: &str = "DCC_ACCEPT";
pub const PAUSE: &str = "PAUSE";
// CHANNELS FRONTEND COMMANDS
pub const PART_CHANNEL: &str = "PART_CHANNEL";
pub const KICK_CHANNEL: &str = "KICK_CHANNEL";
pub const SERVER: &str = "SERVER";
pub const AWAY: &str = "AWAY";
pub const UNAWAY: &str = "UNAWAY";
pub const OPER: &str = "OPER";

// SERVERS COMMANDS
pub const SQUIT: &str = "SQUIT";

pub const USERS_INFO: &str = "USERS_INFO";
pub const CHANNEL_INFO: &str = "CHANNEL_INFO";
pub const SERVER_EXISTS: &str = "SERVER_EXISTS";
pub const IS_OPERATOR: &str = "IS_OPERATOR";
pub const OPERATOR: &str = "OPER";
