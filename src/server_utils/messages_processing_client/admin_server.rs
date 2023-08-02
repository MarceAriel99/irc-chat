use std::sync::mpsc::{Receiver, Sender};

use crate::{
    custom_errors::{errors::CRITICAL, server_error::ServerError},
    numeric_reply::{
        NumericReply, ERR_NOPRIVILEGES_MSG, ERR_NOPRIVILEGES_NUM, ERR_NOSUCHSERVER_MSG,
        ERR_NOSUCHSERVER_NUM,
    },
};

use crate::message::Message;

pub fn handle_quit_server(
    message: Message,
    sender: &Sender<Message>,
    receiver: &Receiver<Message>,
) -> Result<Option<NumericReply>, ServerError> {
    println!("Quit in server handler");
    if message.params_total_count() == 0 {
        let reply = NumericReply::new(ERR_NOSUCHSERVER_NUM, ERR_NOSUCHSERVER_MSG, None);
        return Ok(Some(reply));
    }
    sender.send(message).map_err(|_| -> ServerError {
        ServerError {
            kind: CRITICAL.to_string(),
            message: "Could not send message".to_string(),
        }
    })?;
    let answer = receiver.recv().map_err(|_| -> ServerError {
        ServerError {
            kind: CRITICAL.to_string(),
            message: "Could not receive message".to_string(),
        }
    })?;
    println!("Handle quit_server answer: {:?}", answer);
    if answer.params[0][0] == "You are not an operator" {
        let reply = NumericReply::new(ERR_NOPRIVILEGES_NUM, ERR_NOPRIVILEGES_MSG, None);
        return Ok(Some(reply));
    }
    if answer.params[0][0] == "Server not found" {
        println!("Server not found");
        let reply = NumericReply::new(ERR_NOSUCHSERVER_NUM, ERR_NOSUCHSERVER_MSG, None);
        return Ok(Some(reply));
    }
    Ok(None)
}

/**************************************TESTS**************************************/
#[cfg(test)]
mod tests {
    use crate::commands::QUIT;
    use crate::custom_errors::errors::CRITICAL;
    use crate::custom_errors::server_error::ServerError;
    use crate::message::Message;
    use crate::numeric_reply::{
        NumericReply, ERR_NOPRIVILEGES_MSG, ERR_NOPRIVILEGES_NUM, ERR_NOSUCHSERVER_MSG,
        ERR_NOSUCHSERVER_NUM,
    };
    use crate::server_utils::messages_processing_client::admin_server::handle_quit_server;
    use std::sync::mpsc;
    #[test]
    fn test_quit_non_existant_server() {
        let (sender, receiver) = mpsc::channel();
        let message = Message {
            command: QUIT.to_string(),
            params: vec![vec!["non_existant_server".to_string()]],
            prefix: None,
        };
        let mut answer = message.clone();
        answer.params = vec![vec!["Server not found".to_string()]];
        sender.send(answer).unwrap();
        let result = handle_quit_server(message, &sender, &receiver);
        assert!(result.is_ok());
        let reply = result.unwrap();
        assert!(reply.is_some());
        let reply = reply.unwrap();
        assert_eq!(
            reply,
            NumericReply::new(ERR_NOSUCHSERVER_NUM, ERR_NOSUCHSERVER_MSG, None)
        );
    }
    #[test]
    fn test_quit_need_more_params() {
        let (sender, receiver) = mpsc::channel();
        let message = Message {
            command: QUIT.to_string(),
            params: vec![vec![]],
            prefix: None,
        };
        let result = handle_quit_server(message, &sender, &receiver);
        assert!(result.is_ok());
        let reply = result.unwrap();
        assert!(reply.is_some());
        let reply = reply.unwrap();
        assert_eq!(
            reply,
            NumericReply::new(ERR_NOSUCHSERVER_NUM, ERR_NOSUCHSERVER_MSG, None)
        );
    }

    #[test]
    fn test_not_oper() {
        let (sender, receiver) = mpsc::channel();
        let message = Message {
            command: QUIT.to_string(),
            params: vec![vec!["some_server".to_string()]],
            prefix: Some("not_oper".to_string()),
        };
        let mut answer = message.clone();
        answer.params = vec![vec!["You are not an operator".to_string()]];
        sender
            .send(answer)
            .map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Could not send message".to_string(),
                }
            })
            .ok();
        let result = handle_quit_server(message, &sender, &receiver);
        assert!(result.is_ok());
        let reply = result.unwrap();
        assert!(reply.is_some());
        let reply = reply.unwrap();
        assert_eq!(
            reply,
            NumericReply::new(ERR_NOPRIVILEGES_NUM, ERR_NOPRIVILEGES_MSG, None)
        );
    }
    #[test]
    fn test_squit_correct() {
        let (sender, receiver) = mpsc::channel();
        let message = Message {
            command: QUIT.to_string(),
            params: vec![vec!["some_server".to_string()]],
            prefix: Some("not_oper".to_string()),
        };
        let answer = message.clone();
        sender
            .send(answer)
            .map_err(|_| -> ServerError {
                ServerError {
                    kind: CRITICAL.to_string(),
                    message: "Could not send message".to_string(),
                }
            })
            .ok();
        let result = handle_quit_server(message, &sender, &receiver);
        assert!(result.is_ok());
        let reply = result.unwrap();
        assert!(reply.is_none());
    }
}
