use bevy::prelude::*;
use renetcode::{
    NetcodeServer, ServerAuthentication, ServerConfig, ServerResult, NETCODE_KEY_BYTES,
    NETCODE_MAX_PACKET_BYTES, NETCODE_USER_DATA_BYTES,
};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::HashMap, thread};
use std::{
    net::{SocketAddr, UdpSocket},
    time::Instant,
};

use crate::client::*;

pub const PRIVATE_KEY: &[u8; 32] = b"an example very very secret key."; // 32-bytes
pub const PROTOCOL_ID: u64 = 123456789;
pub const PORT: u32 = 5000;

pub struct Username(pub String);
impl Username {
    pub fn to_netcode_user_data(&self) -> [u8; NETCODE_USER_DATA_BYTES] {
        let mut user_data = [0u8; NETCODE_USER_DATA_BYTES];
        if self.0.len() > NETCODE_USER_DATA_BYTES - 8 {
            panic!("Username is too big");
        }
        user_data[0..8].copy_from_slice(&(self.0.len() as u64).to_le_bytes());
        user_data[8..self.0.len() + 8].copy_from_slice(self.0.as_bytes());

        user_data
    }

    fn from_user_data(user_data: &[u8; NETCODE_USER_DATA_BYTES]) -> Self {
        let mut buffer = [0u8; 8];
        buffer.copy_from_slice(&user_data[0..8]);
        let mut len = u64::from_le_bytes(buffer) as usize;
        len = len.min(NETCODE_USER_DATA_BYTES - 8);
        let data = user_data[8..len + 8].to_vec();
        let username = String::from_utf8(data).unwrap();
        Self(username)
    }
}

pub fn server_main() {
    let server_addr: SocketAddr = format!("127.0.0.1:{}", PORT).parse().unwrap();
    server(server_addr, *PRIVATE_KEY);
}

fn server(addr: SocketAddr, private_key: [u8; NETCODE_KEY_BYTES]) {
    let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let config = ServerConfig {
        current_time,
        max_clients: 16,
        protocol_id: PROTOCOL_ID,
        public_addresses: vec![addr],
        authentication: ServerAuthentication::Secure { private_key },
    };
    let mut server: NetcodeServer = NetcodeServer::new(config);
    let udp_socket = UdpSocket::bind(addr).unwrap();
    udp_socket.set_nonblocking(true).unwrap();
    let mut messages_to_deliver: Vec<(Destination, MultiplayerMessage)> = vec![];
    let mut last_updated = Instant::now();
    let mut buffer = [0u8; NETCODE_MAX_PACKET_BYTES];
    let mut usernames: HashMap<u64, String> = HashMap::new();
    let mut players: HashMap<u64, Player> = HashMap::new();
    let mut last_ping = Instant::now();
    loop {
        server.update(Instant::now() - last_updated);
        messages_to_deliver.clear();

        loop {
            match udp_socket.recv_from(&mut buffer) {
                Ok((len, addr)) => {
                    // println!("Received decrypted message {:?} from {}.", &buffer[..len], addr);
                    let server_result = server.process_packet(addr, &mut buffer[..len]);
                    handle_server_result(
                        server_result,
                        &udp_socket,
                        &mut messages_to_deliver,
                        &mut usernames,
                        &mut players,
                    );
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => panic!("Socket error: {}", e),
            };
        }

        for (destination, message) in messages_to_deliver.iter() {
            for client_id in server.clients_id().iter().filter(|cid| match destination {
                Destination::All => true,
                Destination::Player(id) => id == *cid,
                Destination::NotPlayer(id) => id != *cid,
            }) {
                match message.get_buf() {
                    Ok(buf) => {
                        let (addr, payload) =
                            server.generate_payload_packet(*client_id, &buf).unwrap();
                        udp_socket.send_to(payload, addr).unwrap();
                    }
                    Err(e) => {
                        eprintln!("Error buffering MultiplayerMessage: {}", e);
                    }
                };
            }
        }

        let lapsed = Instant::now() - last_ping;
        if lapsed.as_secs_f32() > 2.0 {
            last_ping = Instant::now();
            for client_id in server.clients_id().into_iter() {
                let server_result = server.update_client(client_id);
                handle_server_result(
                    server_result,
                    &udp_socket,
                    &mut messages_to_deliver,
                    &mut usernames,
                    &mut players,
                );
            }
        }

        last_updated = Instant::now();
        thread::sleep(Duration::from_millis(50));
    }
}

fn handle_server_result(
    server_result: ServerResult,
    socket: &UdpSocket,
    messages_to_deliver: &mut Vec<(Destination, MultiplayerMessage)>,
    usernames: &mut HashMap<u64, String>,
    players: &mut HashMap<u64, Player>,
) {
    match server_result {
        ServerResult::Payload { client_id, payload } => {
            //let text = String::from_utf8(payload.to_vec()).unwrap();
            let multiplayer_message = MultiplayerMessage::get(payload);
            match multiplayer_message {
                Ok(mess) => {
                    let id = mess.get_id();
                    //let username = usernames.get(&client_id).unwrap();
                    let opt_player: Option<&mut Player> = players.get_mut(&client_id);
                    match opt_player {
                        Some(player) => {
                            let username: &str = player.name.as_ref();
                            //println!( "Client {} ({}) sent message {:?}.", username, client_id, text);
                            println!("Client {} ({}) sent message {:?}.", username, client_id, id);
                            match mess {
                                MultiplayerMessage::Connect { .. } => println!(
                                   "Client should not send MultiplayerMessage::Connect to the server."
                                ),
                                MultiplayerMessage::Disconnect { .. } => println!(
                                    "Client should not send MultiplayerMessage::Disconnect to the server."
                                ),
                                MultiplayerMessage::Move {
                                    client_id,
                                    location,
                                } => {
                                    println!("Player moved so setting player location");
                                    player.location = location;
                                    messages_to_deliver.push((Destination::NotPlayer(client_id), mess));
                                },
                                MultiplayerMessage::None => {
                                    eprintln!("MultiplayerMessage::None received at the server from cid {}", client_id);
                                }
                            };
                        }
                        None => {
                            println!("Player does not exist! Can not move.");
                        }
                    }
                }
                _ => {
                    println!("multiplayer message error??")
                }
            };
            //let text = format!("{}: {}", username, text);
            //messages_to_deliver.push(text);
        }
        ServerResult::PacketToSend { payload, addr } => {
            socket.send_to(payload, addr).unwrap();
        }
        ServerResult::ClientConnected {
            client_id,
            user_data,
            payload,
            addr,
        } => {
            let username = Username::from_user_data(&user_data);
            println!("Client {} with id {} connected.", username.0, client_id);
            //Store references to new player.
            usernames.insert(client_id, username.0.clone());
            let player: Player = initialise_new_player(players, username.0);
            players.insert(client_id, player);
            //Acknowledge ClientConnected message.
            socket.send_to(payload, addr).unwrap();
            //Send connect messages to the existing players and the new player.
            push_new_client_messages(client_id, messages_to_deliver, players);
        }
        ServerResult::ClientDisconnected {
            client_id,
            addr,
            payload,
        } => {
            println!("Client {} disconnected.", client_id);
            usernames.remove_entry(&client_id);
            players.remove_entry(&client_id);
            //Acknowledge disconnect.
            if let Some(payload) = payload {
                socket.send_to(payload, addr).unwrap();
            }
            push_disconnect_client_messages(client_id, messages_to_deliver);
        }

        ServerResult::None => {}
    }
}

fn push_disconnect_client_messages(
    disconnect_client_id: u64,
    messages_to_deliver: &mut Vec<(Destination, MultiplayerMessage)>,
) {
    let msg = MultiplayerMessage::Disconnect {
        client_id: disconnect_client_id,
    };
    messages_to_deliver.push((Destination::All, msg));
}

fn push_new_client_messages(
    new_client_id: u64,
    messages_to_deliver: &mut Vec<(Destination, MultiplayerMessage)>,
    players: &mut HashMap<u64, Player>,
) {
    let new_player: &Player = players.get(&new_client_id).unwrap();
    let msg = MultiplayerMessage::Connect {
        client_id: new_client_id,
        location: new_player.location,
        direction: new_player.direction,
        name: new_player.name.clone(),
    };

    //Send the new player connect to itself and all existing players.
    messages_to_deliver.push((Destination::All, msg));

    //Send Multiplayer::Connect to the new client for all existing players.
    for (c_id, player) in players.iter() {
        if *c_id != new_client_id {
            //get the details of an existing player
            let existing_player_msg = MultiplayerMessage::Connect {
                client_id: *c_id,
                location: player.location,
                direction: player.direction,
                name: player.name.clone(),
            };
            //send the message to the new player.
            messages_to_deliver.push((Destination::Player(new_client_id), existing_player_msg));
            println!("send message for existing player:{}", player.name);
        }
    }
}

//Calculate the spawn point for the new player.
fn initialise_new_player(players: &mut HashMap<u64, Player>, name: String) -> Player {
    let mut num = get_player_num(players) as f32;
    num = num + 1.0;
    Player {
        location: Vec3::new(num * 4., 4., 0.),
        direction: Vec3::new(0., 0., 0.),
        name,
        num: num as u8,
    }
}

//Used to calculate the spawn point.
fn get_player_num(players: &HashMap<u64, Player>) -> u8 {
    players.iter().fold(0, |max_num, player| {
        if player.1.num > max_num {
            player.1.num
        } else {
            max_num
        }
    })
}

struct Player {
    location: Vec3,
    direction: Vec3,
    name: String,
    //used to calculate the starting point of littleman.
    num: u8,
}

enum Destination {
    Player(u64),
    NotPlayer(u64),
    All,
}

#[test]
fn test_get_player_num() {
    let mut players: HashMap<u64, Player> = HashMap::new();
    assert_eq!(0, get_player_num(&players));
    let player = Player {
        location: Vec3::new(0., 0., 0.),
        direction: Vec3::new(0., 0., 0.),
        name: "shrubbo".to_string(),
        num: 0,
    };
    players.insert(111, player);
    let player = Player {
        location: Vec3::new(0., 0., 0.),
        direction: Vec3::new(0., 0., 0.),
        name: "shrubbo1".to_string(),
        num: 5,
    };
    players.insert(222, player);
    assert_eq!(5, get_player_num(&players));
    let player = Player {
        location: Vec3::new(0., 0., 0.),
        direction: Vec3::new(0., 0., 0.),
        name: "shrubbo".to_string(),
        num: 6,
    };
    players.insert(311, player);
    assert_eq!(6, get_player_num(&players));
}
