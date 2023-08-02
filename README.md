<h1 align="center" style="color:black"> IRC-CHAT </h1> 

This is a Rust implementation of a multi-server IRC chat system. Some of the most important features supported are:
* Connection and registration
* Private messages
* Channels
* Channel operators / topics / private channels ...
* File transfer between users

#### Screenshot
![Screenshot](https://github.com/MarceAriel99/irc-chat/assets/60658991/af0c5507-dd40-45ea-bd50-9d79f931a9bc)

#### This application was developed for the subject 'Taller de programación 1' in colaboration with:
<div>
    <a href="https://github.com/lucasbilo"> 
      <img src="https://github.com/lucasbilo.png?size=50" width="50">
    </a>
    <a href="https://github.com/juanireil">
      <img src="https://github.com/juanireil.png?size=50" width="50">
    </a>
    <a href="https://github.com/ariana-salese">
      <img src="https://github.com/ariana-salese.png?size=50" width="50">
    </a>
</div>

### Languages and tools
<div>
  <img src="https://github.com/devicons/devicon/blob/master/icons/rust/rust-plain.svg" title="RUST" alt="Rust" width="60" height="60"/>&nbsp;
  <img src="https://github.com/MarceAriel99/irc-chat/assets/60658991/92dd89e3-99d8-41d3-a5fc-a10df470d08e" title="GTK3" alt="Gtk3" width="60" height="60"/>&nbsp;
</div>

(This application was developed in Rust 1.64.0)

## Start the server

    cargo run --bin server <server_persistency_file>

server_persistency_file contains the information about the server (One of these is server_data.txt)

#### **_MAIN SERVER_**
The main server is unique and receives connections from secondary servers.
The persistency file can contain the following lines:

#### Config information: 
```
    S;server_name;address;main_server;users_file_path
```
Example:
```
    S;rust;127.0.0.1:3000;none;saved_files/users.txt
```

#### Server admin information: 
```
    A;password;nickname
```
Example:
```
    A;password123;juanireil
```

#### **_SECONDARY SERVER_**
The secondary server is the one that connects to the primary server.

You must provide the ip, port and the correct name of the main server so it can start working correctly.
In the repository there are different secondary server files (eg server_data_sec_1, server_data_sec_2).
The persistency file can contain the following lines:

#### Config information:

    S;server_name;server_addres;main_server_name;main_server_addres

Example:

    S;secondary_server_1;127.0.0.1:3001;main_server;127.0.0.1:3000
    
#### Server admin information: 

Same as main server

### Registered users persistency
*users.txt* contains the information of all registered users.
The file can contain the following lines:

```
    U;nickname;adress;username;real_name;server_name;password
```
Example:
    
```
    U;juanireil;127.0.0.1;juani;Juan Reil;rust;password123
```

## Start Client
    
    cargo run --bin client

A first window will appear, in which the name of the server, the IP, and the port are requested. If these fields are correct, it will connect and proceed to log in or register.

## Run tests  
    cargo test

## Generate documentation
The code is documented according to the Rust standards present in its manual.
In order to generate and view the documentation, you must use the command:

    cargo doc --open

## Showcase
https://github.com/MarceAriel99/irc-chat/assets/60658991/a11a5f79-a900-48af-93d4-1d16af39c025

