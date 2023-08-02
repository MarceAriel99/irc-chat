//!
//! Channel respresents a group chat.
//!

use crate::{
    commands::{CHANNEL_INFO, MODE_SET_INVITE, MODE_SET_KEY},
    custom_errors::errors::NONCRITICAL,
    message::Message,
    numeric_reply::{
        NumericReply, ERR_BADCHANNELKEY_MSG, ERR_BADCHANNELKEY_NUM, ERR_BANNEDFROMCHAN_MSG,
        ERR_BANNEDFROMCHAN_NUM, ERR_CHANNELHASKEY_MSG, ERR_CHANNELHASKEY_NUM,
        ERR_CHANNELISFULL_MSG, ERR_CHANNELISFULL_NUM, ERR_CHANOPRIVSNEEDED_MSG,
        ERR_CHANOPRIVSNEEDED_NUM, ERR_INVALIDLIMIT_MSG, ERR_INVALIDLIMIT_NUM,
        ERR_INVITEONLYCHAN_MSG, ERR_INVITEONLYCHAN_NUM, ERR_KEYSET_MSG, ERR_KEYSET_NUM,
        ERR_NEEDMOREPARAMS_MSG, ERR_NEEDMOREPARAMS_NUM, ERR_NOSUCHCHANNEL_MSG, ERR_NOSUCHNICK_MSG,
        ERR_NOSUCHNICK_NUM, ERR_NOTONCHANNEL_MSG, ERR_NOTONCHANNEL_NUM, ERR_TOOMANYCHANNELS_MSG,
        ERR_TOOMANYCHANNELS_NUM, ERR_USERONCHANNEL_MSG, ERR_USERONCHANNEL_NUM, RPL_NOTOPIC_MSG,
        RPL_NOTOPIC_NUM, RPL_TOPIC_NUM,
    },
    server_utils::user::User,
};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use crate::custom_errors::server_error::ServerError;

#[derive(Clone, Debug)]
pub struct Channel {
    pub name: String,
    pub topic: Option<String>,
    pub users: HashMap<String, User>,
    pub key: Option<String>,
    pub operators: Vec<String>, //nicknames of users given operator privileges
    pub invites: Vec<String>,   //nicknames of users invited
    pub limit: Option<usize>,
    pub enter_mode: Option<String>,
    pub operator_settable_topic: bool,
    pub secret: bool,
    pub banned: HashSet<String>,
}

impl Channel {
    ///
    /// Returns a Channel
    ///
    pub fn new(name: String, operator: &User) -> Channel {
        let users = HashMap::from([(operator.nickname.clone(), operator.clone())]);

        Channel {
            name,
            topic: None,
            users,
            key: None,
            operators: vec![operator.nickname.clone()],
            invites: Vec::new(),
            limit: None,
            enter_mode: None,
            operator_settable_topic: false,
            secret: false,
            banned: HashSet::new(),
        }
    }

    /********************************JOIN FUNCTIONS**********************************/

    ///
    /// Joins a user to the channel if possible. If user is already on channel the RPL_TOPIC
    // or RPL_NO_TOPIC is returned, just like when a user joins correctly. Could return the
    /// following numeric replies and the
    /// user will not join:
    ///
    /// ERR_TOOMANYCHANNELS: user reached limit of channels(10).
    /// ERR_CHANNELISFULL: if channels has a limit of participants and reached it.
    /// ERR_INVITEONLYCHAN: channel is invite only and user was not invited.
    /// ERR_NEEDMOREPARAMS: channels requiers key and was not provided.
    ///
    pub fn join(&mut self, user: User, key: Option<String>) -> Result<NumericReply, ServerError> {
        // Check if user is already on channel
        if self.is_user_on_channel(&user.nickname) {
            return Ok(self.get_topic_reply());
        }

        // Check if user is banned
        if self.is_banned(&user.nickname) {
            return Ok(NumericReply::new(
                ERR_BANNEDFROMCHAN_NUM,
                ERR_BANNEDFROMCHAN_MSG,
                Some(vec![self.name.clone()]),
            ));
        }

        // Check if user reached limit of channels (10)
        if user.channels.len() == 10 {
            return Ok(NumericReply::new(
                ERR_TOOMANYCHANNELS_NUM,
                ERR_TOOMANYCHANNELS_MSG,
                Some(vec![self.name.clone()]),
            ));
        }

        // Check if channel has limit
        if self.limit.is_some() {
            // If limit is reached then channel is full
            if self.users.len() >= *self.limit.as_ref().unwrap() {
                return Ok(NumericReply::new(
                    ERR_CHANNELISFULL_NUM,
                    ERR_CHANNELISFULL_MSG,
                    Some(vec![self.name.clone()]),
                ));
            }
        }

        // Check enter mode
        if let Some(mode) = self.enter_mode.clone() {
            if mode.as_str() == MODE_SET_INVITE && !self.invites.contains(&user.nickname) {
                return Ok(NumericReply::new(
                    ERR_INVITEONLYCHAN_NUM,
                    ERR_INVITEONLYCHAN_MSG,
                    Some(vec![self.name.clone()]),
                ));
            } else if mode.as_str() == MODE_SET_KEY {
                // Check if key was given
                if key.is_none() {
                    return Ok(NumericReply::new(
                        ERR_CHANNELHASKEY_NUM,
                        ERR_CHANNELHASKEY_MSG,
                        None,
                    ));
                }
                // Check if key given is correct
                if &key.unwrap() != self.key.as_ref().unwrap() {
                    return Ok(NumericReply::new(
                        ERR_BADCHANNELKEY_NUM,
                        ERR_BADCHANNELKEY_MSG,
                        Some(vec![self.name.clone()]),
                    ));
                }
            }
        }

        self.users.insert(user.nickname.clone(), user);

        Ok(self.get_topic_reply())
    }

    /********************************REMOVE USER FUNCTIONS**********************************/

    ///
    /// Removes user from channel if possible. If user removed is last operator then a random
    /// user is set as an operator. If channel is left empty the it is the server responsability
    /// to delete it.  Could return the following numeric replies and the user will not be
    /// removed:
    ///
    /// ERR_NOTONCHANNEL: user leaving not in channel.
    ///
    pub fn part(&mut self, user: User) -> Option<NumericReply> {
        let nickname = &user.nickname;

        let user_to_remove = self.remove_user(nickname);

        if user_to_remove.is_none() {
            return Some(NumericReply::new(
                ERR_NOTONCHANNEL_NUM,
                ERR_NOTONCHANNEL_MSG,
                Some(vec![user.nickname.clone(), self.name.clone()]),
            ));
        };

        // If channel is empty return
        if self.is_empty() {
            return None;
        }

        // Check if user is operator
        if self.operators.is_empty() {
            let users_nicks: Vec<String> = self.users.clone().into_keys().collect();
            let nickname = users_nicks.get(0).unwrap();
            self.operators.push(nickname.to_string());
        }

        None
    }

    ///
    /// Tries to remove user from channel, it returns the user that was removed
    ///
    pub fn remove_user(&mut self, nickname: &String) -> Option<User> {
        // User removed
        let user = self.users.remove(nickname);

        if user.is_some() && self.is_operator(nickname) {
            // User removed was oper, removing from operators
            let index = self
                .operators
                .iter()
                .position(|nick| nick == nickname)
                .unwrap();
            self.operators.remove(index);
        }

        user
    }

    /********************************KEY FUNCTIONS**********************************/

    ///
    /// Sets key of channel. If enter mode of channel is invite the it gets discard and
    /// all the invitation get deleted. Could return the following numeric replies and
    /// the key will not be set:
    ///
    /// ERR_NEEDMOREPARAMS: no limit was given.
    /// ERR_NOTONCHANNEL: user trying to set mode is not on channel.
    /// ERR_CHANOPRIVSNEEDED: user trying to set mode is not an operator.
    /// ERR_KEYSET: key is already set.
    ///
    pub fn set_key(
        &mut self,
        message: Message,
        nickname_user_setting_mode: String,
    ) -> Result<(), NumericReply> {
        if message.params_total_count() < 3 {
            return Err(NumericReply::new(
                ERR_NEEDMOREPARAMS_NUM,
                ERR_NEEDMOREPARAMS_MSG,
                None,
            ));
        }

        if let Some(reply) = self.reply_user_using_privileges(&nickname_user_setting_mode) {
            return Err(reply);
        }

        if self.key.is_some() {
            return Err(NumericReply::new(ERR_KEYSET_NUM, ERR_KEYSET_MSG, None));
        }

        self.invites.clear();
        self.enter_mode = Some(MODE_SET_KEY.to_string());
        self.key = Some(message.params[2][0].clone());

        Ok(())
    }

    ///
    /// Removes key. In case of error could return the following numeric replies:
    ///
    /// ERR_NOTONCHANNEL: user trying to set mode is not on channel.
    /// ERR_CHANOPRIVSNEEDED: user trying to set mode is not an operator.
    ///
    pub fn remove_key(&mut self, nickname_user_setting_mode: String) -> Result<(), NumericReply> {
        if let Some(reply) = self.reply_user_using_privileges(&nickname_user_setting_mode) {
            return Err(reply);
        }

        self.enter_mode = None;
        self.key = None;

        Ok(())
    }

    /*****************************LIMIT FUNCTIONS********************************/

    ///
    /// Sets a limit of amount of participants able to join. If amout of actual participants
    /// is higher that limit then no action is taken, but no more users will be able to
    /// join. In case of error could return the following numeric replies:
    ///
    /// ERR_NEEDMOREPARAMS: no limit was given.
    /// ERR_NOTONCHANNEL: user trying to set mode is not on channel.
    /// ERR_CHANOPRIVSNEEDED: user trying to set mode is not an operator.
    /// If the limit received in message is not a number ERR_NEEDMOREPARAMS is returned.
    ///
    pub fn set_limit(
        &mut self,
        message: Message,
        nickname_user_setting_mode: String,
    ) -> Result<(), NumericReply> {
        if message.params_total_count() < 3 {
            return Err(NumericReply::new(
                ERR_NEEDMOREPARAMS_NUM,
                ERR_NEEDMOREPARAMS_MSG,
                None,
            ));
        }

        if let Some(reply) = self.reply_user_using_privileges(&nickname_user_setting_mode) {
            return Err(reply);
        }

        let limit = message.params[2][0].clone();
        self.limit = match limit.parse::<usize>() {
            Ok(limit) => Some(limit),
            Err(_) => {
                return Err(NumericReply::new(
                    ERR_INVALIDLIMIT_NUM,
                    ERR_INVALIDLIMIT_MSG,
                    Some(vec![limit]),
                ))
            }
        };

        Ok(())
    }

    ///
    /// Removes limit of amount of participants able to join. In case of error
    /// could return the following numeric replies:
    ///
    /// ERR_NOTONCHANNEL: user trying to set mode is not on channel.
    /// ERR_CHANOPRIVSNEEDED: user trying to set mode is not an operator.
    ///
    pub fn remove_limit(&mut self, nickname_user_setting_mode: String) -> Result<(), NumericReply> {
        if let Some(reply) = self.reply_user_using_privileges(&nickname_user_setting_mode) {
            return Err(reply);
        }

        self.limit = None;
        Ok(())
    }

    /***************************INVITE ONLY FUNCTIONS******************************/

    ///
    /// Sets channel as invite only. If a key was set then gets deleted. In case of error
    /// could return the following numeric replies:
    ///
    /// ERR_NOTONCHANNEL: user trying to set mode is not on channel.
    /// ERR_CHANOPRIVSNEEDED: user trying to set mode is not an operator.
    ///
    pub fn set_as_invite_only(
        &mut self,
        nickname_user_setting_mode: String,
    ) -> Result<(), NumericReply> {
        if let Some(reply) = self.reply_user_using_privileges(&nickname_user_setting_mode) {
            return Err(reply);
        }

        self.key = None;
        self.enter_mode = Some(MODE_SET_INVITE.to_string());

        Ok(())
    }

    ///
    /// Removes invite only mode. Deletes all previous invitations. If channel was not in
    /// invite only mode then no action is taken. In case of error could return the
    /// following replies:
    ///
    /// ERR_NOTONCHANNEL: user trying to set mode is not on channel.
    /// ERR_CHANOPRIVSNEEDED: user trying to set mode is not an operator.
    ///
    pub fn remove_invite_only_status(
        &mut self,
        nickname_user_setting_mode: String,
    ) -> Result<(), NumericReply> {
        if let Some(reply) = self.reply_user_using_privileges(&nickname_user_setting_mode) {
            return Err(reply);
        }

        if self.enter_mode == Some(MODE_SET_INVITE.to_string()) {
            self.enter_mode = None;
            self.invites.clear();
        }

        Ok(())
    }

    ///
    /// Saves nickname of user invited. Could return the following numeric replies
    /// and the invite will not be saved:
    ///
    /// ERR_CHANOPRIVSNEEDED: user trying to invite other user is not an operator.
    /// ERR_USERONCHANNEL: user to be invited is already on channel.
    ///
    pub fn save_invite(
        &mut self,
        nickname: &String,
        oper_nickname: String,
    ) -> Option<NumericReply> {
        if let Some(reply) = self.reply_user_using_privileges(&oper_nickname) {
            return Some(reply);
        }

        if self.is_user_on_channel(nickname) {
            return Some(NumericReply::new(
                ERR_USERONCHANNEL_NUM,
                ERR_USERONCHANNEL_MSG,
                Some(vec![self.name.clone(), nickname.clone()]),
            ));
        }

        self.invites.push(nickname.to_string());

        None
    }

    pub fn is_user_invited(&self, nickname: &String) -> bool {
        self.invites.contains(nickname)
    }

    /*************************OPERATOR PRIVILEGES FUNCTIONS****************************/

    ///
    /// Gives operator privileges to nickname specified in message. If the user giving privileges to
    /// has privileges already no action is taken. In case of error could return the following
    /// numeric replies:
    ///
    /// ERR_NEEDMOREPARAMS: no user was given.
    /// ERR_NOTONCHANNEL: user trying to set mode is not on channel.
    /// ERR_CHANOPRIVSNEEDED: user trying to set mode is not an operator.
    /// ERR_NOSUCHNICK: user being kicked is not on channel.
    ///
    pub fn give_operator_privileges(
        &mut self,
        message: Message,
        nickname_user_setting_mode: String,
    ) -> Result<(), NumericReply> {
        if message.params_total_count() < 3 {
            return Err(NumericReply::new(
                ERR_NEEDMOREPARAMS_NUM,
                ERR_NEEDMOREPARAMS_MSG,
                None,
            ));
        }

        if let Some(reply) = self.reply_user_using_privileges(&nickname_user_setting_mode) {
            return Err(reply);
        }

        let nickname_user_giving_privileges_to = &message.params[2][0];

        if !self.is_user_on_channel(nickname_user_giving_privileges_to) {
            return Err(NumericReply::new(
                ERR_NOSUCHNICK_NUM,
                ERR_NOSUCHNICK_MSG,
                Some(vec![
                    self.name.clone(),
                    nickname_user_giving_privileges_to.to_string(),
                ]),
            ));
        }

        if !self.is_operator(nickname_user_giving_privileges_to) {
            self.operators
                .push(nickname_user_giving_privileges_to.to_string());
        }

        Ok(())
    }

    ///
    /// Removes operator privileges from nickname specified in message. If the nickname of
    /// the user removing the privileges is the same as the one whose privileges are being taken
    /// the no action is taken. Also, if the user whose privileges are being taken from has no
    /// privileges already no action is taken. In case of error could return the following
    /// numeric replies:
    ///
    /// ERR_NEEDMOREPARAMS: no user was given.
    /// ERR_NOTONCHANNEL: user trying to set mode is not on channel.
    /// ERR_CHANOPRIVSNEEDED: user trying to set mode is not an operator.
    /// ERR_NOSUCHNICK: user being kicked is not on channel.
    ///
    pub fn remove_operator_privileges(
        &mut self,
        message: Message,
        nickname_user_setting_mode: String,
    ) -> Result<(), NumericReply> {
        if message.params_total_count() < 3 {
            return Err(NumericReply::new(
                ERR_NEEDMOREPARAMS_NUM,
                ERR_NEEDMOREPARAMS_MSG,
                None,
            ));
        }

        if let Some(reply) = self.reply_user_using_privileges(&nickname_user_setting_mode) {
            return Err(reply);
        }

        let nickname_user_taking_privileges_from = &message.params[2][0];

        if &nickname_user_setting_mode == nickname_user_taking_privileges_from {
            return Ok(());
        }

        if !self.is_user_on_channel(nickname_user_taking_privileges_from) {
            return Err(NumericReply::new(
                ERR_NOSUCHNICK_NUM,
                ERR_NOSUCHCHANNEL_MSG,
                Some(vec![
                    self.name.clone(),
                    nickname_user_taking_privileges_from.to_string(),
                ]),
            ));
        }

        if self.is_operator(nickname_user_taking_privileges_from) {
            let index = self
                .operators
                .iter()
                .position(|nick| nick == nickname_user_taking_privileges_from)
                .unwrap();
            self.operators.remove(index);
        }

        Ok(())
    }

    /*****************************SECRET FUNCTIONS********************************/

    ///
    /// Sets channel as secret. In case of error could return the following
    /// numeric replies:
    ///
    /// ERR_NOTONCHANNEL: user trying to set mode is not on channel.
    /// ERR_CHANOPRIVSNEEDED: user trying to set mode is not an operator.
    ///
    pub fn set_as_secret(
        &mut self,
        nickname_user_setting_mode: String,
    ) -> Result<(), NumericReply> {
        if let Some(reply) = self.reply_user_using_privileges(&nickname_user_setting_mode) {
            return Err(reply);
        }

        self.secret = true;
        Ok(())
    }

    ///
    /// Removes secret status. In case of error could return the following
    /// numeric replies:
    ///
    /// ERR_NOTONCHANNEL: user trying to set mode is not on channel.
    /// ERR_CHANOPRIVSNEEDED: user trying to set mode is not an operator.
    ///
    pub fn remove_secret_status(
        &mut self,
        nickname_user_setting_mode: String,
    ) -> Result<(), NumericReply> {
        if let Some(reply) = self.reply_user_using_privileges(&nickname_user_setting_mode) {
            return Err(reply);
        }

        self.secret = false;
        Ok(())
    }

    /*****************************KICK FUNCTIONS********************************/

    ///
    /// Kicks user with nickname given. If a user is trying to kick itself no action in taken.
    // In case of error could return the following
    /// numeric replies:
    ///
    /// ERR_NOTONCHANNEL: user trying to set mode is not on channel.
    /// ERR_CHANOPRIVSNEEDED: user trying to set mode is not an operator.
    /// ERR_NOSUCHNICK: user being kicked is not on channel.
    ///
    pub fn kick(
        &mut self,
        nickname_user_getting_kicked: &String,
        nickname_user_kicking: &String,
    ) -> Option<NumericReply> {
        if let Some(reply) = self.reply_user_using_privileges(nickname_user_kicking) {
            return Some(reply);
        }

        if !self.is_user_on_channel(nickname_user_getting_kicked) {
            // User being kicked is not in channel
            return Some(NumericReply::new(
                ERR_NOSUCHNICK_NUM,
                ERR_NOSUCHNICK_MSG,
                Some(vec![
                    self.name.clone(),
                    nickname_user_getting_kicked.clone(),
                ]),
            ));
        }

        if nickname_user_getting_kicked == nickname_user_kicking {
            return None;
        }

        let user = self.remove_user(nickname_user_getting_kicked);

        println!("user getting kicked (in kick channel) {:?}", user);

        None
    }

    /*****************************BAN FUNCTIONS********************************/

    ///
    /// Sets ban for nicknames specified in messages. In case of error could return the following
    /// numeric replies:
    ///
    /// ERR_NEEDMOREPARAMS: no users were given.
    /// ERR_NOTONCHANNEL: user trying to set mode is not on channel.
    /// ERR_CHANOPRIVSNEEDED: user trying to set mode is not an operator.
    ///
    pub fn set_ban(
        &mut self,
        message: Message,
        nickname_user_setting_mode: String,
    ) -> Result<(), NumericReply> {
        if message.params_total_count() < 3 {
            return Err(NumericReply::new(
                ERR_NEEDMOREPARAMS_NUM,
                ERR_NEEDMOREPARAMS_MSG,
                None,
            ));
        }

        if let Some(reply) = self.reply_user_using_privileges(&nickname_user_setting_mode) {
            return Err(reply);
        }

        let nicknames = &message.params[2];

        for nickname in nicknames {
            if !self.is_banned(nickname) {
                self.banned.insert(nickname.to_string());
            }
        }

        Ok(())
    }

    ///
    /// Removes ban from nicknames specified in message. In case of error could return the following
    /// numeric replies:
    ///
    /// ERR_NOTONCHANNEL: user trying to set mode is not on channel.
    /// ERR_CHANOPRIVSNEEDED: user trying to set mode is not an operator.
    ///
    pub fn remove_ban(
        &mut self,
        message: Message,
        nickname_user_setting_mode: String,
    ) -> Result<(), NumericReply> {
        if let Some(reply) = self.reply_user_using_privileges(&nickname_user_setting_mode) {
            return Err(reply);
        }

        if message.params_total_count() == 2 {
            self.banned.clear();
            return Ok(());
        }

        let nicknames = &message.params[2];

        for nickname in nicknames {
            if self.is_banned(nickname) {
                self.banned.remove(nickname);
            }
        }

        Ok(())
    }

    /*****************************TOPIC FUNCTIONS********************************/

    ///
    /// Returns RPL_TOPIC or RPL_NOTOPIC accoding to channel topic
    ///
    pub fn get_topic_reply(&self) -> NumericReply {
        if self.topic.is_none() {
            return NumericReply::new(
                RPL_NOTOPIC_NUM,
                RPL_NOTOPIC_MSG,
                Some(vec![self.name.clone()]),
            );
        }
        return NumericReply::new(
            RPL_TOPIC_NUM,
            self.topic.clone().unwrap().as_str(),
            Some(vec![self.name.clone()]),
        );
    }

    ///
    /// Sets new topic. If topic is set correctly then RPL_TOPIC is returned. If an
    /// error was found the following numeric relpies will be returned:
    ///
    /// ERR_NOTONCHANNEL: user trying to set mode is not on channel.
    /// ERR_CHANOPRIVSNEEDED: topic is only settable by operator and user is not an operator.
    ///
    pub fn set_topic(
        &mut self,
        nickname: &String,
        topic: &str,
    ) -> Result<NumericReply, NumericReply> {
        println!("cheking if {} is oper", nickname);

        if !self.is_user_on_channel(nickname) {
            return Err(NumericReply::new(
                ERR_NOTONCHANNEL_NUM,
                ERR_NOTONCHANNEL_MSG,
                Some(vec![self.name.clone()]),
            ));
        }

        if self.operator_settable_topic && !self.is_operator(nickname) {
            return Err(NumericReply::new(
                ERR_CHANOPRIVSNEEDED_NUM,
                ERR_CHANOPRIVSNEEDED_MSG,
                Some(vec![self.name.clone()]),
            ));
        }

        self.topic = Some(topic.to_owned());

        Ok(NumericReply::new(
            RPL_TOPIC_NUM,
            topic,
            Some(vec![self.name.clone()]),
        ))
    }

    ///
    /// Sets topic as operator settable only. In case of error could return the following
    /// numeric replies:
    ///
    /// ERR_NOTONCHANNEL: user trying to set mode is not on channel.
    /// ERR_CHANOPRIVSNEEDED: user trying to set mode is not an operator.
    ///
    pub fn set_operator_settable_topic(
        &mut self,
        nickname_user_setting_mode: String,
    ) -> Result<(), NumericReply> {
        if let Some(reply) = self.reply_user_using_privileges(&nickname_user_setting_mode) {
            return Err(reply);
        }

        self.operator_settable_topic = true;
        Ok(())
    }

    ///
    /// Removes topic as operator settable only status. In case of error could return the
    /// following numeric replies:
    ///
    /// ERR_NOTONCHANNEL: user trying to set mode is not on channel.
    /// ERR_CHANOPRIVSNEEDED: user trying to set mode is not an operator.
    ///
    pub fn remove_operator_settable_topic(
        &mut self,
        nickname_user_setting_mode: String,
    ) -> Result<(), NumericReply> {
        if let Some(reply) = self.reply_user_using_privileges(&nickname_user_setting_mode) {
            return Err(reply);
        }

        self.operator_settable_topic = false;
        Ok(())
    }

    pub fn channel_has_topic(&self, topic: &String) -> bool {
        if self.topic.is_none() {
            return false;
        }

        self.topic.clone().unwrap() == *topic
    }

    /*****************************STATUS FUNCTIONS********************************/

    ///
    /// Checks if user with nickname given is an operator
    ///
    fn is_operator(&self, nickname: &String) -> bool {
        self.operators.contains(nickname)
    }

    ///
    /// Checks if user with nickname given is an banned
    ///
    fn is_banned(&self, nickname: &String) -> bool {
        self.banned.contains(nickname)
    }

    ///
    /// Checks if channel is multiserver according to channel name
    ///
    pub fn is_multiserver(&self) -> bool {
        self.name.starts_with('#')
    }

    ///
    /// Checks if channel is secret
    ///
    pub fn is_secret(&self) -> bool {
        self.secret
    }

    ///
    /// Checks if user with the given nickname is on channel
    ///
    pub fn is_user_on_channel(&self, nickname: &String) -> bool {
        self.users.get(nickname).is_some()
    }

    ///
    /// Tells whether the channel is empty or not
    ///
    pub fn is_empty(&self) -> bool {
        self.users.is_empty()
    }

    /*****************************GENERAL FUNCTIONS********************************/

    ///
    /// Returns:
    ///
    /// ERR_NOTONCHANNEL: user not on channel.
    /// ERR_CHANOPRIVSNEEDED: user is not an operator.
    /// None: non of the above is true.
    ///
    pub fn reply_user_using_privileges(&self, nickname: &String) -> Option<NumericReply> {
        if !self.is_user_on_channel(nickname) {
            return Some(NumericReply::new(
                ERR_NOTONCHANNEL_NUM,
                ERR_NOTONCHANNEL_MSG,
                Some(vec![self.name.clone()]),
            ));
        }

        if !self.is_operator(nickname) {
            return Some(NumericReply::new(
                ERR_CHANOPRIVSNEEDED_NUM,
                ERR_CHANOPRIVSNEEDED_MSG,
                Some(vec![self.name.clone()]),
            ));
        }

        None
    }

    ///
    /// Function needed to inform of channels existing. It returns a message with all the information of the channel
    ///
    pub fn channel_to_message(&self) -> Message {
        let topic = match self.topic {
            Some(ref topic) => topic.clone(),
            None => "None".to_string(),
        };
        let users: Vec<String> = self.users.keys().cloned().collect();
        let key = match self.key {
            Some(ref key) => key.clone(),
            None => "None".to_string(),
        };
        let limit = match self.limit {
            Some(ref limit) => limit.to_string(),
            None => "0".to_string(),
        };
        let mode = match self.enter_mode {
            Some(ref mode) => mode.clone(),
            None => "None".to_string(),
        };
        let mut params = vec![vec![
            self.name.clone(),
            topic,
            key,
            limit,
            mode,
            self.operator_settable_topic.to_string(),
            self.secret.to_string(),
        ]];
        params.push(users);
        params.push(self.operators.clone());
        let invites = self.invites.clone();
        if invites.is_empty() {
            params.push(vec!["None".to_string()]);
        } else {
            params.push(invites);
        }
        let banned = self.banned.clone();
        if banned.is_empty() {
            params.push(vec!["None".to_string()]);
        } else {
            params.push(self.banned.iter().cloned().collect());
        }

        Message {
            prefix: Some(self.clone().name),
            command: CHANNEL_INFO.to_string(),
            params,
        }
    }

    ///
    /// It will return a channel from a message containing all the information of the channels
    ///
    pub fn channel_from_message(
        message: Message,
        users: Arc<Mutex<HashMap<String, User>>>,
    ) -> Result<Channel, ServerError> {
        let params = message.params;
        let operators = params[2].clone();
        let mut invites = params[3].clone();
        let mut banned: HashSet<String> = params[4].clone().iter().cloned().collect();
        let mut topic = Some(params[0][1].clone());

        if params[0][1] == "None" {
            topic = None;
        }
        let mut key = Some(params[0][2].clone());
        if params[0][2] == "None" {
            key = None;
        }
        let mut limit = Some(params[0][3].clone().parse::<usize>().unwrap());
        if limit.unwrap() == 0 {
            limit = None;
        }
        let mut mode = Some(params[0][4].clone());
        if params[0][4] == "None" {
            mode = None;
        }
        if invites.contains(&"None".to_string()) {
            invites.clear();
        }
        if banned.contains(&"None".to_string()) {
            banned.clear();
        }

        let nicks_users = params[1].clone();
        let users = users.lock().map_err(|_| -> ServerError {
            ServerError {
                kind: NONCRITICAL.to_string(),
                message: "Could not access channels".to_string(),
            }
        })?;
        let mut channel_users = HashMap::new();

        for nick in nicks_users {
            let user = match users.get(&nick) {
                Some(user) => user,
                None => {
                    return Err(ServerError {
                        kind: NONCRITICAL.to_string(),
                        message: "User not found".to_string(),
                    })
                }
            };

            println!("adding user {:?}", user);

            channel_users.insert(user.nickname.clone(), user.clone());
        }

        Ok(Channel {
            name: params[0][0].clone(),
            topic,
            key,
            limit,
            enter_mode: mode,
            operator_settable_topic: params[0][5].parse::<bool>().unwrap(),
            secret: params[0][6].parse::<bool>().unwrap(),
            users: channel_users,
            operators,
            invites,
            banned,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::Channel;
    use crate::server_utils::user::User;

    #[test]
    fn test_new_channel() {
        let user = User::new(
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );
        let channel = Channel::new("test".to_string(), &user);

        assert_eq!(channel.name, "test");
        assert_eq!(channel.topic, None);
        //assert_eq!(channel.operator, "test");
        assert_eq!(channel.users.len(), 1);
        assert!(channel.users.contains_key("test"));
        assert_eq!(channel.invites.len(), 0);
    }

    #[test]
    fn test_join_channel_correctly() {
        let user = User::new(
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );
        let mut channel = Channel::new("test".to_string(), &user);
        let another_user = User::new(
            "test2".to_string(),
            "test2".to_string(),
            "test2".to_string(),
            "test2".to_string(),
            "test2".to_string(),
            "password".to_string(),
        );

        channel.join(another_user, None).unwrap();

        assert_eq!(channel.users.len(), 2);
        assert!(channel.users.contains_key("test"));
        //assert_eq!(channel.operator, "test");
        assert!(channel.users.contains_key("test2"));
    }

    #[test]
    fn test_user_already_in_channel() {
        let user = User::new(
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );
        let mut channel = Channel::new("test".to_string(), &user);
        let another_user = User::new(
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );

        channel.join(another_user, None).unwrap();

        assert_eq!(channel.users.len(), 1);
        assert!(channel.users.contains_key("test"));
        //assert_eq!(channel.operator, "test");
    }

    #[test]
    fn test_invite_user_already_in_channel() {
        let user = User::new(
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );
        let user_nick = user.nickname.clone();
        let mut channel = Channel::new("test".to_string(), &user);

        channel.save_invite(&user_nick, user.nickname.clone());

        assert_eq!(channel.invites.len(), 0);
    }

    #[test]
    fn test_invite_new_user() {
        let user = User::new(
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            "password".to_string(),
        );
        let another_user = User::new(
            "test2".to_string(),
            "test2".to_string(),
            "test2".to_string(),
            "test2".to_string(),
            "test2".to_string(),
            "password".to_string(),
        );
        let user_nick = another_user.nickname.clone();
        let mut channel = Channel::new("test".to_string(), &user);

        channel.save_invite(&user_nick, user.nickname.clone());

        assert_eq!(channel.invites.len(), 1);
        assert_eq!(channel.invites[0], "test2");
    }
}
