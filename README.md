# 22C2-Panicked-At-Pensar-Nombre-

## Integrantes:

- [Bilo Lucas](https://github.com/lucasbilo)
- [Reil Luz Juan Ignacio](https://github.com/juanireil)
- [Rondán Marcelo Ariel](https://github.com/MarceAriel99)
- [Salese D'Assaro Ariana Magalí](https://github.com/ariana-salese)

La aplicacion está programada con Rust y gtk 3, ambos son necesarios para poder correrlo.

## Correr server

Para poder conectarse en múltiples computadoras, es necesario poner en el archivo de configuracion del server, en el campo del address la ip de la computadora que esta hosteando al servidor junto a su puerto en formato ip_address:port.

Para correr el servidor, se debe ejecutar el comando:

    cargo run --bin server <archivo_persistencia_server>

archivo_persistencia_server es el nombre del archivo en el cuál se encuentra la información del servidor que se quiere levantar. 
Un ejemplo de estos es server_data.txt

### Archivo de persistencia del server
#### **_SERVIDOR PRINCIPAL_**
El servidor principal es unico y recibe conexiones de servidores secundarios. Su archivo se compone de la siguiente manera:

#### Si es la informacion de configuracion: 
```
    S;server_name;address;main_server;users_file_path
```

El users_file_path hace referencia al archivo en el cual se guarda la informacion de los usuarios registrados

Por ejemplo:
    
```
    S;rust;127.0.0.1:3000;none;saved_files/users.txt
```

#### Si es la informacion de un administrador del server: 
```
    A;password;nickname
```

Por ejemplo:
    
```
    A;password123;juanireil
```

server_data.txt es el nombre del archivo de persistencia del servidor principal creado por nostros.

En caso de querer levantar un servidor propio, se debe crear un nuevo archivo con el mismo formato y pasarselo.

#### **_SERVIDOR SECUNDARIO_**
El servidor secundario es el que se conecta al servidor principal. 
Se debe poner la ip, puerto y el nombre correcto del servidor principal para que este pueda arrancar a funcionar correctamente.
En el repositorio se encuentran distintos archivos de servidores secundarios (ej. server_data_sec_1, server_data_sec_2). 
Para el servidor secundario, cada linea de su archivo se compone de la siguiente manera:

#### Si es la informacion de configuracion:

    S;server_name;server_addres;main_server_name;main_server_addres

Por ejemplo:

    S;secondary_server_1;127.0.0.1:3001;main_server;127.0.0.1:3000
    
#### Si es la informacion del administrador del servidor: 

Es igual que para el servidor principal

### Persistencia de usuarios
users.txt contiene la información de todos los usuarios registrados. Poseé la siguiente información: 

```
    U;nickname;adress;username;real_name;server_name;password
```

Por ejemplo:
    
```
    U;juanireil;127.0.0.1;juani;Juan Reil;rust;password123
```

## Correr Client

Para correr el cliente, el comando es 
    
    cargo run --bin client

Allí mostrará una primera ventana en la que se pide el nombre del servidor con el cuál se conectará, la ip (la que hayan puesto en el archivo de configuracion) y el puerto. Si estos datos son correctos se conectarán y procederán a iniciar sesion o registrarse.
## Correr tests
Si se desea correr los tests el comando a ejecutar es:
    
    cargo test

## Generar documentacion

El código se encuentra documentado según los estandares de Rust presentes en su manual.
Para poder generar y visualizar la documentacion, se debe usar el comando

    cargo doc --open
