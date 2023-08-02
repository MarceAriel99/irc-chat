//!
//! This module contains every numeric reply with number and message.
//! NumericReply represents the message and number of the reply as a struct.
//!

// GENERAL
pub const ERR_NEEDMOREPARAMS_NUM: &str = "461";
pub const ERR_NEEDMOREPARAMS_MSG: &str = "Not enough parameters";

pub const ERR_NOSUCHNICK_NUM: &str = "401";
pub const ERR_NOSUCHNICK_MSG: &str = "No such nick/channel";

// LOGIN AND REGISTRATION
pub const ERR_NONICKNAMEGIVEN_NUM: &str = "431";
pub const ERR_NONICKNAMEGIVEN_MSG: &str = "No nickname given";

pub const ERR_ERRONEUSNICKNAME_NUM: &str = "432";
pub const ERR_ERRONEUSNICKNAME_MSG: &str = "Erroneus nickname";

pub const ERR_NICKNAMEINUSE_NUM: &str = "433";
pub const ERR_NICKNAMEINUSE_MSG: &str = "Nickname is already in use";

pub const ERR_NICKCOLLISION_NUM: &str = "436";
pub const ERR_NICKCOLLISION_MSG: &str = "Nickname collision KILL";

pub const ERR_INVALIDLOGIN_NUM: &str = "1";
pub const ERR_INVALIDLOGIN_MSG: &str = "No such user registered";

pub const RPL_CORRECTLOGIN_NUM: &str = "2";
pub const RPL_CORRECTLOGIN_MSG: &str = "Login successful";

pub const RPL_CORRECTREGISTRATION_NUM: &str = "3";
pub const RPL_CORRECTREGISTRATION_MSG: &str = "Registration successful";

pub const ERR_ALREADYREGISTRED_NUM: &str = "462";
pub const ERR_ALREADYREGISTRED_MSG: &str = "You may not reregister";

// OPERATOR REPLIES
pub const ERR_PASSWDMISMATCH_NUM: &str = "464";
pub const ERR_PASSWDMISMATCH_MSG: &str = "Password incorrect";

pub const RPL_YOUREOPER_NUM: &str = "381";
pub const RPL_YOUREOPER_MSG: &str = "You are now an IRC operator";

// CHANNEL
pub const ERR_BADCHANNELKEY_NUM: &str = "475";
pub const ERR_BADCHANNELKEY_MSG: &str = "Cannot join channel (+k)";

pub const ERR_TOOMANYCHANNELS_NUM: &str = "405";
pub const ERR_TOOMANYCHANNELS_MSG: &str = "You have joined too many channels";

pub const ERR_CHANNELISFULL_NUM: &str = "471";
pub const ERR_CHANNELISFULL_MSG: &str = "Cannot join channel (+l)";

pub const ERR_BANNEDFROMCHAN_NUM: &str = "474";
pub const ERR_BANNEDFROMCHAN_MSG: &str = "Cannot join channel (+b)";

pub const ERR_CHANNELHASKEY_NUM: &str = "476";
pub const ERR_CHANNELHASKEY_MSG: &str = "The channel has a key";

pub const ERR_KEYSET_NUM: &str = "467";
pub const ERR_KEYSET_MSG: &str = "Channel key already set";

pub const ERR_USERONCHANNEL_NUM: &str = "443";
pub const ERR_USERONCHANNEL_MSG: &str = "is already on channel";

pub const ERR_CHANOPRIVSNEEDED_NUM: &str = "482";
pub const ERR_CHANOPRIVSNEEDED_MSG: &str = "You're not channel operator";

pub const RPL_INVITING_NUM: &str = "341";

pub const ERR_INVITEONLYCHAN_NUM: &str = "473";
pub const ERR_INVITEONLYCHAN_MSG: &str = "Cannot join channel (+i)";

pub const ERR_NOSUCHCHANNEL_NUM: &str = "403";
pub const ERR_NOSUCHCHANNEL_MSG: &str = "No such channel";

pub const RPL_MODESET_NUM: &str = "9";
pub const RPL_MODESET_MSG: &str = "Mode was set correctly";

pub const ERR_INVALIDLIMIT_NUM: &str = "8";
pub const ERR_INVALIDLIMIT_MSG: &str = "limit is invalid";

pub const ERR_NOTONCHANNEL_NUM: &str = "442";
pub const ERR_NOTONCHANNEL_MSG: &str = "You're not on that channel";

pub const RPL_LISTSTART_NUM: &str = "321";
pub const RPL_LISTSTART_MSG: &str = "Users  Name";

pub const RPL_LISTEND_NUM: &str = "323";
pub const RPL_LISTEND_MSG: &str = "End of /LIST";

pub const RPL_LIST_NUM: &str = "322";

pub const RPL_NAMEREPLY_NUM: &str = "353";

pub const RPL_ENDOFNAMES_NUM: &str = "366";
pub const RPL_ENDOFNAMES_MSG: &str = "End of /NAMES list";

pub const RPL_TOPIC_NUM: &str = "332";

pub const RPL_NOTOPIC_NUM: &str = "331";
pub const RPL_NOTOPIC_MSG: &str = "No topic is set";

pub const ERR_UNKNOWNMODE_NUM: &str = "472";
pub const ERR_UNKNOWNMODE_MSG: &str = "is unknown mode char to me";

//SERVER
pub const ERR_NORECIPIENT_NUM: &str = "411";
pub const ERR_NORECIPIENT_MSG: &str = "No recipient given";

pub const ERR_NOTEXTTOSEND_NUM: &str = "412";
pub const ERR_NOTEXTTOSEND_MSG: &str = "No text to send";

// WHOIS REPLIES
pub const RPL_WHOISUSER_NUM: &str = "311";

pub const RPL_WHOISSERVER_NUM: &str = "312";
pub const RPL_WHOISSERVER_MSG: &str = "server info";

pub const RPL_WHOISOPERATOR_NUM: &str = "313";
pub const RPL_WHOISOPERATOR_MSG: &str = "is an irc operator";

pub const RPL_ENDOFWHOIS_NUM: &str = "318";
pub const RPL_ENDOFWHOIS_MSG: &str = "End of /WHOIS list";

pub const RPL_WHOISCHANNELS_NUM: &str = "319";
pub const RPL_WHOISCHANNELS_MSG: &str = "channel name";

// WHO REPLIES
pub const RPL_ENDOFWHO_NUM: &str = "315";
pub const RPL_ENDOFWHO_MSG: &str = "End of WHO list";

pub const RPL_WHOREPLY_NUM: &str = "352";
pub const RPL_WHOREPLY_MSG: &str = "WHO reply";

// AWAY REPLIES
pub const RPL_UNAWAY_MSG: &str = "You are no longer marked as being away";
pub const RPL_UNAWAY_NUM: &str = "305";
pub const RPL_AWAY_NUM: &str = "301";

pub const RPL_NOWAWAY_MSG: &str = "You have been marked as being away";
pub const RPL_NOWAWAY_NUM: &str = "306";

// SQUIT REPLIES
pub const ERR_NOPRIVILEGES_NUM: &str = "481";
pub const ERR_NOPRIVILEGES_MSG: &str = "Permission Denied- You're not an IRC operator";

pub const ERR_NOSUCHSERVER_NUM: &str = "402";
pub const ERR_NOSUCHSERVER_MSG: &str = "No such server";
#[derive(Debug, PartialEq, Eq)]
pub struct NumericReply {
    message: String,
    number: String,
    params: Vec<String>,
}

impl NumericReply {
    ///
    /// Returns a NumericReply
    ///
    pub fn new(number: &str, message: &str, parameters: Option<Vec<String>>) -> Self {
        let mut params = vec![];

        if let Some(p) = parameters {
            params = p;
        };

        NumericReply {
            message: message.to_string(),
            number: number.to_string(),
            params,
        }
    }

    ///
    /// Returns NumericReply as a string
    ///
    pub fn as_string(&self) -> String {
        let mut params_str = String::new();

        for param in &self.params {
            params_str.push_str(param.as_str());
            params_str.push(' ');
        }

        if !self.message.is_empty() {
            format!("{} {}:{}\r\n", self.number, params_str, self.message)
        } else {
            format!("{} {}\r\n", self.number, params_str)
        }
    }

    ///
    /// It will check if the number of the NumericReply is the same as any of the numbers passed as parameter
    ///
    pub fn has_number(&self, numbers: Vec<&str>) -> bool {
        for num in numbers {
            if self.number.as_str() == num {
                return true;
            }
        }
        false
    }
}
