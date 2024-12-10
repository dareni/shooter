use bevy::prelude::*;
use renetcode::{ClientAuthentication, ConnectToken, NetcodeClient, NETCODE_MAX_PACKET_BYTES};
use std::{
    io::{Read,Write},
    net::{SocketAddr, UdpSocket},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use crate::server::*;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum Multiplayer {
    #[default]
    Disconnected,
    Disconnecting,
    Connected,
}

#[repr(u8)]
enum MultiPlayerMessage {
    Connect {
        player_id: u16,
        pos: Vec3,
        direction: Vec3,
        name: String,
    },
    Move {
        player_id: u16,
        pos: Vec3,
        direction: Vec3,
    },
    None,
}
impl MultiPlayerMessage {
    pub fn get_id(&self) -> u8 {
        match self {
            MultiPlayerMessage::None => 0,
            MultiPlayerMessage::Connect { .. } => 1,
            MultiPlayerMessage::Move { .. } => 2,
        }
    }

    pub fn get_buf(&self) -> Result<[u8; 100], std::io::Error> {
        let buf = [0u8; 100];
        let mut cursor = std::io::Cursor::new(buf);

        match self {
            MultiPlayerMessage::Connect {
                player_id,
                pos,
                direction,
                name,
            } => {
                cursor.write(&self.get_id().to_le_bytes())?;
                cursor.write(&player_id.to_le_bytes())?;
                cursor.write(&pos.x.to_le_bytes())?;
                cursor.write(&pos.y.to_le_bytes())?;
                cursor.write(&pos.z.to_le_bytes())?;
                cursor.write(&direction.x.to_le_bytes())?;
                cursor.write(&direction.y.to_le_bytes())?;
                cursor.write(&direction.z.to_le_bytes())?;
                let size: u8 = name.len() as u8;
                cursor.write(&[size])?;
                cursor.write(&name.as_bytes())?;
                Ok(cursor.into_inner())
            }
            MultiPlayerMessage::Move { .. } => Ok(cursor.into_inner()),
            MultiPlayerMessage::None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "No message??",
                ))
            }
        }
    }

    pub fn get(buf: &[u8]) -> Result<MultiPlayerMessage, std::io::Error> {
        let cursor = &mut std::io::Cursor::new(buf);
        let message_id: [u8; 1] = read_bytes::<1>(cursor)?;

        match message_id {
            [1] => {
                //let _id: u16 = u16::from_le_bytes(read_bytes::<2>(cursor)?);
                let player_id: u16 = u16::from_le_bytes(read_bytes::<2>(cursor)?);
                let pos: Vec3 = Vec3::new(
                    f32::from_le_bytes(read_bytes::<4>(cursor)?),
                    f32::from_le_bytes(read_bytes::<4>(cursor)?),
                    f32::from_le_bytes(read_bytes::<4>(cursor)?),
                );
                let direction: Vec3 = Vec3::new(
                    f32::from_le_bytes(read_bytes::<4>(cursor)?),
                    f32::from_le_bytes(read_bytes::<4>(cursor)?),
                    f32::from_le_bytes(read_bytes::<4>(cursor)?),
                );
                let size: u8 = u8::from_le_bytes(read_bytes::<1>(cursor)?);
                let mut _name = vec![0u8; size as usize];
                let name_array = _name.as_mut_slice();
                assert_eq!(size as usize, name_array.len());
                cursor.read_exact(name_array)?;
                let name = String::from_utf8(_name)
                    .expect("Error extract name from buf for MultiPlayerMessage");
                Ok(MultiPlayerMessage::Connect {
                    player_id,
                    pos,
                    direction,
                    name,
                })
            }
            //[2] => {}
            _ => Ok(MultiPlayerMessage::None),
        }
    }
}

#[test]
fn test_multiplayermessage_connect() {
    let mess = MultiPlayerMessage::Connect {
        player_id: 77u16,
        pos: Vec3::new(1.1f32, 2.2f32, 3.3f32),
        direction: Vec3::new(4.4f32, 5.5f32, 6.6f32),
        name: "ikky".to_string(),
    };
    let buf = mess.get_buf().unwrap();
    println!("buf:{:?}", buf);
    let connect: MultiPlayerMessage = MultiPlayerMessage::get(&buf).unwrap().into();
    match connect {
        MultiPlayerMessage::Connect {
            player_id,
            pos,
            direction,
            name,
        } => {
            assert_eq!(player_id, 77);
            assert_eq!(pos, Vec3::new(1.1f32, 2.2f32, 3.3f32));
            assert_eq!(direction, Vec3::new(4.4f32, 5.5f32, 6.6f32));
            assert_eq!(name, "ikky".to_string());
        }
        _ => panic!("test_multiplayermessage_connect fail!"),
    };
}

#[inline]
pub fn read_bytes<const N: usize>(src: &mut impl std::io::Read) -> Result<[u8; N], std::io::Error> {
    let mut data = [0u8; N];
    src.read_exact(&mut data)?;
    Ok(data)
}

#[derive(Resource)]
pub struct RenetClient {
    client: Option<NetcodeClient>,
    udp_socket: Option<UdpSocket>,
    buffer: [u8; NETCODE_MAX_PACKET_BYTES],
    last_updated: Instant,
}

impl RenetClient {
    pub fn new() -> RenetClient {
        RenetClient {
            client: None,
            udp_socket: None,
            buffer: [0u8; NETCODE_MAX_PACKET_BYTES],
            last_updated: Instant::now(),
        }
    }

    pub fn connect(&mut self, user_name: String) {
        let server_addr: SocketAddr = format!("127.0.0.1:{}", PORT).parse().unwrap();
        let username = Username(user_name);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        println!(
            "Stating connecting at {:?} with username {}",
            now, username.0,
        );
        let client_id = now.as_millis() as u64;
        let connect_token = ConnectToken::generate(
            now,
            PROTOCOL_ID,
            300,
            client_id,
            15,
            vec![server_addr],
            Some(&username.to_netcode_user_data()),
            PRIVATE_KEY,
        )
        .unwrap();
        let authentication = ClientAuthentication::Secure { connect_token };
        self.udp_socket = Some(UdpSocket::bind("127.0.0.1:0").unwrap());
        self.udp_socket
            .as_ref()
            .unwrap()
            .set_nonblocking(true)
            .unwrap();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        self.client = Some(NetcodeClient::new(now, authentication).unwrap());
        self.last_updated = Instant::now();
    }

    pub fn get_server_data(&mut self) {
        //loop {

        let r_client = self.client.as_mut().expect("RenetClient not initialized!");
        if let Some(err) = r_client.disconnect_reason() {
            panic!("Client error: {:?}", err);
        }
        let r_socket = self
            .udp_socket
            .as_ref()
            .expect("RenetClient not initialized!");

        //Send data from this client to all other clients.
        if r_client.is_connected() {
            let text = "abc".to_string();
            let (addr, payload) = r_client.generate_payload_packet(text.as_bytes()).unwrap();
            self.udp_socket
                .as_ref()
                .unwrap()
                .send_to(payload, addr)
                .unwrap();
        } else {
            println!("Client is not yet connected");
        }

        loop {
            match r_socket.recv_from(&mut self.buffer.as_mut()) {
                Ok((len, addr)) => {
                    if addr != r_client.server_addr() {
                        // Ignore packets that are not from the server
                        continue;
                    }
                    println!(
                        "Received decrypted message {:?} from server {}",
                        &self.buffer[..len].as_mut(),
                        addr
                    );
                    if let Some(payload) = r_client.process_packet(&mut self.buffer[..len].as_mut())
                    {
                        let text = String::from_utf8(payload.to_vec()).unwrap();
                        println!("Received message from server: {}", text);
                        //Update the world here with data from other clients.
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => panic!("Socket error: {}", e),
            };
        }

        if let Some((packet, addr)) = r_client.update(Instant::now() - self.last_updated) {
            r_socket.send_to(packet, addr).unwrap();
        }
        self.last_updated = Instant::now();
    }

    //Issue the disconnect client command and send the generated packet to the server.
    pub fn disconnect(&mut self) {
        let client = self.client.as_mut().expect("RenetClient not initialised!");
        let r_socket = self
            .udp_socket
            .as_ref()
            .expect("RenetClient not initialized!");
        match client.disconnect() {
            Ok((addr, packet)) => {
                r_socket.send_to(packet, addr).unwrap();
            }
            Err(e) => {
                println!("Error during disconnect. {}", e);
            }
        };
    }

    pub fn is_disconnected(&mut self) -> bool {
        let client = self.client.as_ref();
        match client {
            Some(cl) => cl.is_disconnected(),
            None => true,
        }
    }
}

pub fn do_multiplayer_disconnect(mut r_client: ResMut<RenetClient>) {
    println!("doing disconnect");
    r_client.disconnect();
}

pub fn do_multiplayer_server(mut r_client: ResMut<RenetClient>) {
    //Pass entity data to the server from here.
    r_client.get_server_data();
}

pub struct ClientPlugin;
impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<Multiplayer>();
        app.add_systems(
            Update,
            do_multiplayer_server.run_if(in_state(Multiplayer::Connected)),
        );
        app.add_systems(
            OnEnter(Multiplayer::Disconnecting),
            do_multiplayer_disconnect,
        );
    }
}

#[cfg(test)]
mod test {

    use crate::server::*;
    use renetcode::{ClientAuthentication, ConnectToken, NetcodeClient, NETCODE_MAX_PACKET_BYTES};
    use std::{
        net::{SocketAddr, UdpSocket},
        time::Instant,
    };
    use std::{
        sync::mpsc::{self, Receiver, TryRecvError},
        thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    fn client_main(user_name: String) {
        let server_addr: SocketAddr = format!("127.0.0.1:{}", PORT).parse().unwrap();
        let username = Username(user_name);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        println!(
            "Stating connecting at {:?} with username {}",
            now, username.0,
        );
        let client_id = now.as_millis() as u64;
        let connect_token = ConnectToken::generate(
            now,
            PROTOCOL_ID,
            300,
            client_id,
            15,
            vec![server_addr],
            Some(&username.to_netcode_user_data()),
            PRIVATE_KEY,
        )
        .unwrap();
        let auth = ClientAuthentication::Secure { connect_token };
        client(auth);
    }

    fn client(authentication: ClientAuthentication) {
        let udp_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        udp_socket.set_nonblocking(true).unwrap();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let mut client = NetcodeClient::new(now, authentication).unwrap();
        let stdin_channel = spawn_stdin_channel();
        let mut buffer = [0u8; NETCODE_MAX_PACKET_BYTES];

        let mut last_updated = Instant::now();
        loop {
            if let Some(err) = client.disconnect_reason() {
                panic!("Client error: {:?}", err);
            }

            match stdin_channel.try_recv() {
                Ok(text) => {
                    if client.is_connected() {
                        let (addr, payload) =
                            client.generate_payload_packet(text.as_bytes()).unwrap();
                        udp_socket.send_to(payload, addr).unwrap();
                    } else {
                        println!("Client is not yet connected");
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => panic!("Stdin channel disconnected"),
            }

            loop {
                match udp_socket.recv_from(&mut buffer) {
                    Ok((len, addr)) => {
                        if addr != client.server_addr() {
                            // Ignore packets that are not from the server
                            continue;
                        }
                        // println!("Received decrypted message {:?} from server {}", &buffer[..len], addr);
                        if let Some(payload) = client.process_packet(&mut buffer[..len]) {
                            let text = String::from_utf8(payload.to_vec()).unwrap();
                            println!("Received message from server: {}", text);
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(e) => panic!("Socket error: {}", e),
                };
            }

            if let Some((packet, addr)) = client.update(Instant::now() - last_updated) {
                udp_socket.send_to(packet, addr).unwrap();
            }
            last_updated = Instant::now();
            thread::sleep(Duration::from_millis(50));
        }
    }

    fn spawn_stdin_channel() -> Receiver<String> {
        let (tx, rx) = mpsc::channel::<String>();
        thread::spawn(move || loop {
            let mut buffer = String::new();
            std::io::stdin().read_line(&mut buffer).unwrap();
            tx.send(buffer.trim_end().to_string()).unwrap();
        });
        rx
    }

    #[test]
    #[ignore]
    pub fn test_client_connect() {
        //needs the server running.
        client_main("shrubbo".to_string());
    }
}
