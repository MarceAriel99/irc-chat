use crate::message::Message;
use std::collections::HashSet;

const CR: u8 = 13;
const LF: u8 = 10;
const COLON: u8 = 58;
const SPACE: u8 = 32;

/// Receives a &str and returns the index of the next whitespace
pub fn next_whitespace(slice: &str) -> Option<usize> {
    for (index, element) in slice.as_bytes().iter().enumerate() {
        if *element == SPACE {
            return Some(index);
        }
    }
    None
}

/// Receives a &str and returns it without the spaces at the start
fn erase_front_whitespaces(mut slice: &str) -> &str {
    for (index, element) in slice.as_bytes().iter().enumerate() {
        if *element != SPACE {
            slice = &slice[index..];
            break;
        }
    }
    slice
}

/// Receives a &str and returns index of CRLF ("\r\n"). If CRLF is not found returns an error.
fn get_index_of_end_of_message(slice: &str) -> Result<usize, String> {
    for (index, element) in slice.as_bytes().iter().enumerate() {
        if *element == LF {
            return Err("Message is broken".to_string());
        }
        if *element == CR {
            if slice.as_bytes().get(index + 1) != Some(&LF) {
                return Err("Message is broken".to_string());
            }
            return Ok(index);
        }
    }

    Err("Message is broken".to_string())
}

/// Splits the parameter according to the received separator and pushes the result to the received vector
/// If the parameter is invalid, returns an error
fn process_last_param(slice: &str, params: &mut Vec<Vec<String>>, sep: char) -> Result<(), String> {
    let index = get_index_of_end_of_message(slice)?;

    // Check if parameters contains any prohibited characters
    if !param_is_valid(&slice[..index]) {
        return Err("Message is broken".to_string());
    }

    let params_as_str: Vec<&str> = slice[..index].split(sep).collect();
    let mut param: Vec<String> = Vec::new();

    for p in params_as_str {
        param.push(p.to_string());
    }

    params.push(param);

    Ok(())
}

/// Returns true if the parameter recieved is valid. The parameter is valid if it does not
/// contain any of the prohibited characters:
/// - Line Feed ("\n")
/// - Carriage Return ("\r")
/// - Colon (":")
fn param_is_valid(slice: &str) -> bool {
    let invalid_characters = HashSet::from([LF, CR, COLON]);

    for element in slice.as_bytes() {
        // Check if parameters contains any prohibited characters
        if invalid_characters.contains(element) {
            return false;
        }
    }

    true
}

/// Returns a Message struct with the data of the received String, or an error if the String is invalid
///
/// # Arguments
///
/// * `message` - A String that holds the message to be parsed
///
pub fn parse(message: String) -> Result<Message, String> {
    let mut remaining_message = message.as_str();

    let prefix: Option<String>;
    let command: String;
    let mut params: Vec<Vec<String>> = vec![];

    // If the first character is a colon, the message has a prefix
    if remaining_message.as_bytes().first() == Some(&COLON) {
        let next_space_index = match next_whitespace(remaining_message) {
            Some(num) => num,
            None => return Err("Message is broken".to_string()),
        };

        prefix = Some(remaining_message[1..next_space_index].to_string());
        remaining_message = &remaining_message[next_space_index..];
    } else {
        prefix = None;
    }

    // Wipe the whitespaces at the start of the remaining message
    remaining_message = erase_front_whitespaces(remaining_message);

    // Get the command and the remaining message, if there are no more spaces, there are no parameters
    let next_space_index = match next_whitespace(remaining_message) {
        Some(num) => num,
        None => {
            let index = get_index_of_end_of_message(remaining_message)?;
            command = remaining_message[..index].to_string();
            return Ok(Message {
                prefix,
                command,
                params,
            });
        }
    };
    command = remaining_message[0..next_space_index].to_string();
    remaining_message = &remaining_message[next_space_index..];

    // Wipe the whitespaces at the start of the remaining message
    remaining_message = erase_front_whitespaces(remaining_message);

    // Loop until the end of the message, saving the parameters
    loop {
        // Looks for ':' (trailing)
        if remaining_message.as_bytes().first() == Some(&COLON) {
            // Push last param without splitting the remaining message, and skipping ":". Then return the message
            process_last_param(&remaining_message[1..], &mut params, '\n')?;
            return Ok(Message {
                prefix,
                command,
                params,
            });
        }
        // Look for next param index
        let next_space_index = match next_whitespace(remaining_message) {
            Some(num) => num,
            None => {
                // If there are no more whitespaces, push last param splitted by comma and return the message
                process_last_param(remaining_message, &mut params, ',')?;
                return Ok(Message {
                    prefix,
                    command,
                    params,
                });
            }
        };
        // Check if the next parameter is valid
        if !param_is_valid(&remaining_message[..next_space_index]) {
            return Err("Message is broken".to_string());
        };

        // Push the parameters to the vector, splitted by comma
        let params_as_str: Vec<&str> = remaining_message[..next_space_index].split(',').collect();
        let mut param: Vec<String> = Vec::new();

        for p in params_as_str {
            param.push(p.to_string());
        }

        params.push(param);

        //Update remaining message
        remaining_message = &remaining_message[next_space_index + 1..];
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn message_with_prefix_and_one_simple_parameter_is_parsed_correctly() {
        let message_str = ":WiZ NICK Kilroy\r\n".to_string();
        let message = parse(message_str).unwrap();

        assert_eq!(message.prefix, Some("WiZ".to_string()));
        assert_eq!(message.command, "NICK");
        assert_eq!(message.params, vec![vec!["Kilroy"]]);
    }

    #[test]
    fn message_with_one_parameter_with_multiple_components_is_parsed_correctly() {
        let message_str = "NAMES #twilight_zone,#42\r\n".to_string();
        let message = parse(message_str).unwrap();

        assert_eq!(message.prefix, None);
        assert_eq!(message.command, "NAMES");
        assert_eq!(message.params, vec![vec!["#twilight_zone", "#42"]]);
    }

    #[test]
    fn message_with_one_simple_param_and_another_with_trailing_is_parsed_correctly() {
        let message_str = "PRIVMSG jto@tolsun.oulu.fi :Hello !\r\n".to_string();
        let message = parse(message_str).unwrap();

        assert_eq!(message.prefix, None);
        assert_eq!(message.command, "PRIVMSG");
        assert_eq!(
            message.params,
            vec![vec!["jto@tolsun.oulu.fi"], vec!["Hello !"]]
        );
    }

    #[test]
    fn message_with_one_simple_param_and_another_with_multiple_components_is_parsed_correctly() {
        let message_str = "INVITE Wiz #Twilight_Zone,#Rust\r\n".to_string();
        let message = parse(message_str).unwrap();

        assert_eq!(message.prefix, None);
        assert_eq!(message.command, "INVITE");
        assert_eq!(
            message.params,
            vec![vec!["Wiz"], vec!["#Twilight_Zone", "#Rust"]]
        );
    }

    #[test]
    fn message_with_params_with_line_feed_returns_error() {
        let message_str = "INVITE Wiz\n #Twilight_Zone,#Rust\r\n".to_string();

        assert!(parse(message_str).is_err());
    }

    #[test]
    fn message_with_params_with_colon_returns_error() {
        let message_str = "INVITE Wiz #Twilight:_Zone,#Rust\r\n".to_string();

        assert!(parse(message_str).is_err());
    }

    #[test]
    fn message_with_params_with_carriage_return_returns_error() {
        let message_str = "INVITE Wiz #Twilight:_Zone,#Rust\r\n".to_string();

        assert!(parse(message_str).is_err());
    }

    #[test]
    fn message_with_inverted_ending_characters_returns_error() {
        let message_str = "INVITE Wiz #Twilight_Zone,#Rust\n\r".to_string();

        assert!(parse(message_str).is_err());
    }

    #[test]
    fn message_with_incomplete_ending_characters_returns_error() {
        let message_str_cr = "INVITE Wiz #Twilight_Zone,#Rust\r".to_string();
        let message_str_lf = "INVITE Wiz #Twilight_Zone,#Rust\n".to_string();

        assert!(parse(message_str_cr).is_err());
        assert!(parse(message_str_lf).is_err());
    }

    #[test]
    fn message_without_parameters_should_generate_a_message_with_empy_list() {
        let message_str = ":user NAMES\r\n".to_string();
        let message = parse(message_str).unwrap();
        assert_eq!(message.prefix, Some("user".to_string()));
        assert_eq!(message.command, "NAMES");
        assert!(message.params.is_empty());
    }
}
