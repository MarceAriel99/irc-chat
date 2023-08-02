//!
//! Message represents the message from client to server or server to server.
//!

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub prefix: Option<String>,
    pub command: String,
    pub params: Vec<Vec<String>>,
}

impl Message {
    ///
    /// Returns the amount of parameters in the message
    ///
    pub fn params_total_count(&self) -> i32 {
        let mut count = 0;

        for param_x in &self.params {
            for _ in param_x {
                count += 1;
            }
        }
        count
    }

    ///
    /// Returns a new message with the prefix provided and clones the rest
    /// of the attributes
    ///
    pub fn set_prefix(&self, prefix: String) -> Message {
        Message {
            prefix: Some(prefix),
            command: self.command.clone(),
            params: self.params.clone(),
        }
    }

    ///
    /// Returns message as string
    ///
    pub fn as_string(&self) -> String {
        // Obtain prefix
        let mut message_as_string = match self.prefix.clone() {
            Some(prefix) => ":".to_string() + &prefix + " ",
            None => "".to_string(),
        };

        // Add command
        message_as_string.push_str(self.command.clone().as_str());

        // Add parameters
        let total_params_count = self.params_total_count();
        let mut current_param_counter = 0;

        for param_list in self.params.clone() {
            if param_list.is_empty() {
                continue;
            }
            message_as_string.push(' ');
            for (param_number, param) in param_list.iter().enumerate() {
                current_param_counter += 1;

                if current_param_counter == total_params_count && param.contains(' ') {
                    message_as_string.push(':');
                } else if param_number != 0 {
                    message_as_string.push(',');
                }
                message_as_string.push_str(param.as_str());
            }
        }

        // Add EOL
        message_as_string.push_str("\r\n");
        message_as_string
    }
}

/************************************TESTS*******************************************/

#[cfg(test)]
mod test {
    use super::Message;

    fn setup() -> Message {
        Message {
            prefix: None,
            command: "COMMAND".to_string(),
            params: vec![
                vec!["param".to_string()],
                vec!["param".to_string()],
                vec!["param".to_string(), "param".to_string()],
            ],
        }
    }

    #[test]
    fn message_is_created_correctly() {
        let message = setup();

        assert_eq!(message.prefix, None);
        assert_eq!(message.command, "COMMAND".to_string());
        assert_eq!(
            message.params,
            vec![
                vec!["param".to_string()],
                vec!["param".to_string()],
                vec!["param".to_string(), "param".to_string()]
            ]
        );
    }

    #[test]
    fn params_total_count_return_correct_count() {
        let message = setup();

        assert_eq!(message.params_total_count(), 4);
    }

    #[test]
    fn message_with_prefix_is_correct() {
        let message = setup();
        let message_with_prefix = message.set_prefix("prefix".to_string());

        assert_eq!(message_with_prefix.prefix, Some("prefix".to_string()));
        assert_eq!(message_with_prefix.command, "COMMAND".to_string());
        assert_eq!(
            message_with_prefix.params,
            vec![
                vec!["param".to_string()],
                vec!["param".to_string()],
                vec!["param".to_string(), "param".to_string()]
            ]
        );
    }
}
