//!
//! ServerData represents the information that the server must know and must persist
//!

use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Error, ErrorKind, Write},
};

use super::channel::Channel;
use crate::server_utils::user::User;

const SERVER_IDENTIFIER: &str = "S";
const USER_IDENTIFIER: &str = "U";
const ADMIN_IDENTIFIER: &str = "A";

#[derive(Debug)]
pub struct ServerData {
    pub server_address: String,
    pub server_name: String,
    pub admin_nick: String,
    pub admin_password: String,
    pub file_path: String,
    pub users: HashMap<String, User>,
    pub channels: HashMap<String, Channel>,
    pub main: Option<(String, String)>,
    pub users_file_path: String,
}

impl ServerData {
    ///
    /// Creates a new ServerData. If an error was found while reading a file or
    /// if the server information is incomplete and error ir returned
    ///
    pub fn new(path: String) -> Result<Self, Error> {
        let none = "none".to_string();

        let mut server_data = ServerData {
            server_address: none.clone(),
            server_name: none.clone(),
            admin_nick: none.clone(),
            admin_password: none.clone(),
            file_path: path.clone(),
            users: HashMap::new(),
            channels: HashMap::new(),
            main: None,
            users_file_path: none.clone(),
        };

        set_server_data(&mut server_data, path)?;

        if server_data.server_address == none
            || server_data.server_name == none
            || server_data.admin_nick == none
            || server_data.admin_password == none
        {
            return Err(Error::new(ErrorKind::NotFound, "server info incomplete"));
        }
        Ok(server_data)
    }

    ///
    /// This will save the config of the server
    ///
    pub fn set_server_config(
        &mut self,
        addres: String,
        name: String,
        main: Option<(String, String)>,
        users_file_path: String,
    ) {
        self.server_address = addres;
        self.server_name = name;
        self.main = main;
        self.users_file_path = users_file_path;
    }

    ///
    /// This will save the admin of the server
    ///
    pub fn set_admin_data(&mut self, admin_nick: String, admin_password: String) {
        self.admin_nick = admin_nick;
        self.admin_password = admin_password;
    }

    ///
    /// This will add a user only if server is main
    ///
    pub fn add_user(&mut self, user: User) {
        self.users.insert(user.nickname.clone(), user);
    }

    ///
    /// Checks if server is main server
    ///
    pub fn is_main(&self) -> bool {
        self.main.is_none()
    }
}

/******************************READ SERVER DATA FILE**********************************/

///
/// This will read the server data file and set the server data,
/// It will also read the users and channels files
///
fn set_server_data(server_data: &mut ServerData, path: String) -> Result<(), Error> {
    read_file_and_set_info(server_data, path)?;

    if server_data.is_main() {
        read_file_and_set_info(server_data, server_data.users_file_path.to_string())?;
    }

    Ok(())
}

///
/// Reads the server data file and sets the server data
///
fn read_file_and_set_info(server_data: &mut ServerData, path: String) -> Result<(), Error> {
    let file = File::open(&path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        let line = line.split(';').collect::<Vec<&str>>();
        parse_line(line, server_data);
    }

    Ok(())
}

///
/// This will parse a line of the server data file and update the data in the struct
///
fn parse_line(line: Vec<&str>, server_data: &mut ServerData) {
    let identifier = line[0];

    match identifier {
        SERVER_IDENTIFIER => parse_and_set_server_data(line, server_data),
        USER_IDENTIFIER => parse_and_set_user(line, server_data),
        ADMIN_IDENTIFIER => parse_and_set_admin(line, server_data),
        &_ => println!("Invalid config file line [{:?}]", line),
    }
}

///
/// Reads a line from the user files and returns a valid user with the
/// information from the line
///
fn parse_user_line(line: Vec<&str>) -> User {
    let nickname = line[1].to_string();
    let address = line[2].to_string();
    let username = line[3].to_string();
    let real_name = line[4].to_string();
    let server_name = line[5].to_string();
    let password = line[6].to_string();

    User::new(
        nickname,
        address,
        username,
        real_name,
        server_name,
        password,
    )
}

///
/// This will parse line and set new user in ServerData
///
fn parse_and_set_user(line: Vec<&str>, server_data: &mut ServerData) {
    let user = parse_user_line(line);

    server_data.add_user(user);
}

///
/// This will parse line and set server data in ServerData
///
fn parse_and_set_server_data(line: Vec<&str>, server_data: &mut ServerData) {
    let name = line[1];
    let address = line[2];
    let main_name = line[3];
    let mut main_data = None;
    let mut user_file_path = "none".to_string();

    if main_name != "none" {
        main_data = Some((main_name.to_string(), line[4].to_string()));
    } else {
        user_file_path = line[4].to_string();
    }

    server_data.set_server_config(
        address.to_string(),
        name.to_string(),
        main_data,
        user_file_path,
    );
}

///
/// This will parse line and set admin in ServerData
///
fn parse_and_set_admin(line: Vec<&str>, server_data: &mut ServerData) {
    let password = line[1];
    let nickname = line[2];
    server_data.set_admin_data(nickname.to_string(), password.to_string());
}

/******************************WRITE ON SERVER DATA FILE********************************/

///
/// This function add a user to the server data file
///
pub fn add_user(user: &User, path: String) -> Result<(), std::io::Error> {
    let mut buf = format!("{};", USER_IDENTIFIER);

    buf.push_str(push_char(&user.nickname, ';').as_str());
    buf.push_str(push_char(&user.address, ';').as_str());
    buf.push_str(push_char(&user.username, ';').as_str());
    buf.push_str(push_char(&user.real_name, ';').as_str());
    buf.push_str(push_char(&user.server_name, ';').as_str());
    buf.push_str(&user.password);
    buf.push('\n');

    println!("adding user {} in FILE {}", buf, path);

    let mut file = OpenOptions::new().append(true).open(path)?;
    file.write_all(buf.as_bytes())?;

    Ok(())
}

///
/// Pushes a char into a String
///
fn push_char(string: &String, end_string: char) -> String {
    let mut string = string.to_string();
    string.push(end_string);
    string.to_string()
}

/****************************************TESTS*****************************************/

#[cfg(test)]
mod tests {

    use super::ServerData;
    use crate::server_utils::user::User;

    #[test]
    fn create_server_data_from_file_sets_admin_information_correctly() {
        // A;contrasena;juanireil

        let server_data =
            ServerData::new("saved_files/main_server_data_test.txt".to_string()).unwrap();

        assert_eq!(server_data.admin_nick, "juanireil".to_string());
        assert_eq!(server_data.admin_password, "contrasena".to_string());
    }

    #[test]
    fn create_server_data_for_main_server_from_file_sets_server_info_correctly() {
        // S;test_server;127.0.0.1:3000

        let server_data =
            ServerData::new("saved_files/main_server_data_test.txt".to_string()).unwrap();

        assert_eq!(server_data.server_name, "test_server".to_string());
        assert_eq!(server_data.server_address, "127.0.0.1:3000".to_string());
        assert_eq!(
            server_data.users_file_path,
            "saved_files/users_test.txt".to_string()
        );
        assert_eq!(server_data.main, None);
    }

    #[test]
    fn create_server_data_for_secondary_server_from_file_sets_server_info_correctly() {
        //S;secondary_server;127.0.0.1:3001;main_server;127.0.0.1:3000

        let server_data =
            ServerData::new("saved_files/secondary_server_data_test.txt".to_string()).unwrap();

        assert_eq!(server_data.server_name, "secondary_server".to_string());
        assert_eq!(server_data.server_address, "127.0.0.1:3001".to_string());
        assert_eq!(server_data.users_file_path, "none".to_string());
        assert_eq!(
            server_data.main,
            Some(("test_server".to_string(), "127.0.0.1:3000".to_string()))
        );
    }

    #[test]
    fn create_server_data_for_main_server_sets_users_correctly() {
        //U;juanireil;127.0.0.1;juani;Juan Reil;test_server;password123
        //U;ari;127.0.0.1;arisalese;Ariana Salese;test_server;password123
        //U;marce;127.0.0.1;marce;Marcelo Rondan;secondary_server;password123

        let server_data =
            ServerData::new("saved_files/main_server_data_test.txt".to_string()).unwrap();

        let users = server_data.users;

        let juani: Option<&User> = match users.get("juanireil") {
            Some(user) => Some(user),
            None => None,
        };

        assert!(juani.is_some());

        let juani = juani.expect("Couldn't get user test");

        assert_eq!(juani.nickname, "juanireil".to_string());
        assert_eq!(juani.address, "127.0.0.1".to_string());
        assert_eq!(juani.username, "juani".to_string());
        assert_eq!(juani.real_name, "Juan Reil".to_string());
        assert_eq!(juani.server_name, "test_server".to_string());
        assert_eq!(juani.password, "password123".to_string());

        let ari: Option<&User> = match users.get("ari") {
            Some(user) => Some(user),
            None => None,
        };

        assert!(ari.is_some());

        let ari = ari.expect("Couldn't get user test");

        assert_eq!(ari.nickname, "ari".to_string());
        assert_eq!(ari.address, "127.0.0.1".to_string());
        assert_eq!(ari.username, "arisalese".to_string());
        assert_eq!(ari.real_name, "Ariana Salese".to_string());
        assert_eq!(ari.server_name, "test_server".to_string());
        assert_eq!(ari.password, "password123".to_string());

        let marce: Option<&User> = match users.get("marce") {
            Some(user) => Some(user),
            None => None,
        };

        assert!(marce.is_some());

        let marce = marce.expect("Couldn't get user test");

        assert_eq!(marce.nickname, "marce".to_string());
        assert_eq!(marce.address, "127.0.0.1".to_string());
        assert_eq!(marce.username, "marce".to_string());
        assert_eq!(marce.real_name, "Marcelo Rondan".to_string());
        assert_eq!(marce.server_name, "secondary_server".to_string());
        assert_eq!(marce.password, "password123".to_string());
    }
}
