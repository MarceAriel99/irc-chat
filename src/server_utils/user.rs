//!
//! User saves all the important data of a user
//!

use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct User {
    pub nickname: String,
    pub address: String,
    pub username: String,
    pub real_name: String,
    pub server_name: String,
    pub password: String,
    pub channels: HashSet<String>,
    pub away_message: Option<String>,
}

impl User {
    ///
    /// Creates a new user with some given values
    ///
    pub fn new(
        nickname: String,
        address: String,
        username: String,
        real_name: String,
        server_name: String,
        password: String,
    ) -> Self {
        User {
            nickname,
            address,
            username,
            real_name,
            server_name,
            password,
            channels: HashSet::new(),
            away_message: None,
        }
    }

    ///
    /// Returns true if the user has any atribute that equals the name received
    ///
    pub fn has_atribute_name(&self, name: &str) -> bool {
        self.nickname == name
            || self.username == name
            || self.real_name == name
            || self.address == name
            || self.server_name == name
    }

    ///
    /// Add a channel to the user, it means that the user is in that channel
    ///
    pub fn add_channel(&mut self, channel_name: &String) {
        self.channels.insert(channel_name.to_string());
    }

    ///
    /// When a user exits a channel, it is removed from the channels list
    ///
    pub fn remove_channel(&mut self, channel_name: &String) {
        self.channels.remove(channel_name);
    }

    ///
    /// Checks if user is away
    ///
    pub fn is_away(&self) -> bool {
        self.away_message.is_some()
    }

    ///
    /// Sets user as no longer away
    ///
    pub fn no_longer_away(&mut self) {
        self.away_message = None;
    }

    ///
    /// Checks if user has away message provided
    ///
    pub fn has_away_message(&self, away_message: &String) -> bool {
        if let Some(message) = self.away_message.clone() {
            return &message == away_message;
        }

        false
    }
}
