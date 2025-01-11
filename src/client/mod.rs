use bevy::prelude::*;
use renetcode::{ClientAuthentication, ConnectToken, NetcodeClient, NETCODE_MAX_PACKET_BYTES};
use std::{
    io::{Error, Read, Write},
    net::{SocketAddr, UdpSocket},
    sync::{
        mpsc,
        mpsc::{Receiver, Sender},
        Mutex,
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use crate::input_n_state::MultiplayerState;
use crate::server::*;
use crate::*;

#[derive(Resource)]
pub struct MultiplayerMessageSender {
    pub sender: Sender<MultiplayerMessage>,
}
#[derive(Resource)]
pub struct MultiplayerMessageReceiver {
    pub receiver: Mutex<Receiver<MultiplayerMessage>>,
}

#[repr(u8)]
pub enum MultiplayerMessage {
    Connect {
        client_id: u64,
        location: Vec3,
        direction: Vec3,
        name: String,
    },
    Disconnect {
        client_id: u64,
    },
    Move {
        client_id: u64,
        location: Vec3,
    },
    //    Rotate {
    //        client_id: u64,
    //        direction: Vec2,
    //    },
    None,
}
impl MultiplayerMessage {
    pub fn get_id(&self) -> u8 {
        match self {
            MultiplayerMessage::None => 0,
            MultiplayerMessage::Connect { .. } => 1,
            MultiplayerMessage::Disconnect { .. } => 2,
            MultiplayerMessage::Move { .. } => 3,
            //MultiplayerMessage::Rotate { .. } => 4,
        }
    }

    pub fn get_buf(&self) -> Result<[u8; 100], Error> {
        let buf = [0u8; 100];
        let mut cursor = std::io::Cursor::new(buf);

        match self {
            MultiplayerMessage::Connect {
                client_id,
                location,
                direction,
                name,
            } => {
                cursor.write(&self.get_id().to_le_bytes())?;
                cursor.write(&client_id.to_le_bytes())?;
                cursor.write(&location.x.to_le_bytes())?;
                cursor.write(&location.y.to_le_bytes())?;
                cursor.write(&location.z.to_le_bytes())?;
                cursor.write(&direction.x.to_le_bytes())?;
                cursor.write(&direction.y.to_le_bytes())?;
                cursor.write(&direction.z.to_le_bytes())?;
                let size: u8 = name.len() as u8;
                cursor.write(&[size])?;
                cursor.write(&name.as_bytes())?;
                Ok(cursor.into_inner())
            }
            MultiplayerMessage::Disconnect { client_id } => {
                cursor.write(&self.get_id().to_le_bytes())?;
                cursor.write(&client_id.to_le_bytes())?;
                Ok(cursor.into_inner())
            }
            MultiplayerMessage::Move {
                client_id,
                location,
            } => {
                cursor.write(&self.get_id().to_le_bytes())?;
                cursor.write(&client_id.to_le_bytes())?;
                cursor.write(&location.x.to_le_bytes())?;
                cursor.write(&location.y.to_le_bytes())?;
                cursor.write(&location.z.to_le_bytes())?;
                Ok(cursor.into_inner())
            }
            MultiplayerMessage::None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "No message??",
                ))
            }
        }
    }

    pub fn get(buf: &[u8]) -> Result<MultiplayerMessage, Error> {
        let cursor = &mut std::io::Cursor::new(buf);
        let message_id: [u8; 1] = read_bytes::<1>(cursor)?;

        match message_id {
            [1] => {
                //let _id: u16 = u16::from_le_bytes(read_bytes::<2>(cursor)?);
                let client_id: u64 = u64::from_le_bytes(read_bytes::<8>(cursor)?);
                let location: Vec3 = Vec3::new(
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
                    .expect("Error extract name from buf for MultiplayerMessage");
                Ok(MultiplayerMessage::Connect {
                    client_id,
                    location,
                    direction,
                    name,
                })
            }
            [2] => {
                let client_id: u64 = u64::from_le_bytes(read_bytes::<8>(cursor)?);
                Ok(MultiplayerMessage::Disconnect { client_id })
            }
            [3] => {
                let client_id: u64 = u64::from_le_bytes(read_bytes::<8>(cursor)?);
                let location: Vec3 = Vec3::new(
                    f32::from_le_bytes(read_bytes::<4>(cursor)?),
                    f32::from_le_bytes(read_bytes::<4>(cursor)?),
                    f32::from_le_bytes(read_bytes::<4>(cursor)?),
                );
                Ok(MultiplayerMessage::Move {
                    client_id,
                    location,
                })
            }
            _ => Ok(MultiplayerMessage::None),
        }
    }
}

#[inline]
pub fn read_bytes<const N: usize>(src: &mut impl std::io::Read) -> Result<[u8; N], Error> {
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
    client_id_16: u16,
    //The renclient:
    //sender sends messages from the server to a channel for bevy to receive;
    //sender: None,
    sender: Sender<MultiplayerMessage>,
    //receiver has access to a channel, containing messages from the bevy app,
    //for transmission to the server. Receiver is wrapped in a mutex for concurrency.
    receiver: Mutex<Receiver<MultiplayerMessage>>,
}

impl RenetClient {
    pub fn new() -> RenetClient {
        //Use 2 multi-producer single consumer fifos to bridge messaging between the renet client and
        //the bevy application. Initialise the client with the tx and rx of one channel. On
        //connect() a second channel is created and RenetClient.sender is configured with the
        //Sender of the this new channel. The resulting configuration allows transmission and
        //reception of messages with the multiplayer server via 2 mpsc channels.
        let (tx_app, rx_server) = mpsc::channel::<MultiplayerMessage>();
        let receiver = Mutex::new(rx_server);
        let sender = tx_app;

        RenetClient {
            client: None,
            udp_socket: None,
            buffer: [0u8; NETCODE_MAX_PACKET_BYTES],
            last_updated: Instant::now(),
            client_id_16: 0,
            sender,
            receiver,
        }
    }

    pub fn connect(
        &mut self,
        user_name: String,
    ) -> (Sender<MultiplayerMessage>, Receiver<MultiplayerMessage>) {
        let server_addr: SocketAddr = format!("127.0.0.1:{}", PORT).parse().unwrap();
        let username = Username(user_name.clone());
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        println!(
            "Stating connecting at {:?} with username {}",
            now, username.0,
        );
        let client_id = now.as_millis() as u64;
        self.client_id_16 = client_id as u16;
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
        let (tx_server, rx_app) = mpsc::channel::<MultiplayerMessage>();
        let tx_app = self.sender.to_owned();
        self.sender = tx_server;
        (tx_app, rx_app)
    }

    pub fn server_transact(&mut self) -> Result<(), Error> {
        //Runs in the bevy app loop while the state is connected.
        //Checks the tcp socket for messages from the server for processing by the bevy app.
        //Checks for messages from bevy for transmission to the server.

        //loop {

        let r_client = self.client.as_mut().expect("RenetClient not initialized!");
        if let Some(err) = r_client.disconnect_reason() {
            //Send a message to bevy to : Command.remove_resource<RenetClient>()
            println!("Client error: {:?}", err);
            let msg = MultiplayerMessage::Disconnect {
                client_id: r_client.client_id(),
            };
            if let Err(e) = self.sender.send(msg) {
                println!("Could not send disconnect message to bevy. {}", e);
            }
        }
        let r_socket = self
            .udp_socket
            .as_ref()
            .expect("RenetClient not initialized!");

        //Send data from this client to all other clients.
        if r_client.is_connected() {
            let mut _rx = self.receiver.lock().unwrap();
            for message in _rx.try_iter() {
                //let (addr, payload) = r_client.generate_payload_packet(&message.get_buf()?).unwrap();
                let (addr, payload) = r_client
                    .generate_payload_packet(&message.get_buf()?)
                    .unwrap();
                self.udp_socket
                    .as_ref()
                    .unwrap()
                    .send_to(payload, addr)
                    .unwrap();
            }
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
                    // println!(
                    //     "Received decrypted message {:?} from server {}",
                    //     &self.buffer[..len].as_mut(),
                    //     addr
                    // );
                    if let Some(payload) = r_client.process_packet(&mut self.buffer[..len].as_mut())
                    {
                        //let text = String::from_utf8(payload.to_vec()).unwrap();
                        //println!("Received message from server: {}", text);
                        //Update the world here with data from other clients.
                        let server_message = MultiplayerMessage::get(payload);
                        //TODO: replace server_message with Ok(server_message)
                        match server_message {
                            Ok(msg) => {
                                let msg_id = msg.get_id();
                                println!("Received msg type {} from server.", msg_id);
                                if let Err(e) = self.sender.send(msg) {
                                    eprintln!(
                                        "Received a faulty  MultiplayerMessage from the server? {}",
                                        e
                                    );
                                }
                            }
                            Err(e) => {
                                eprintln!("Error receiving server message: {}", e);
                            }
                        }
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
        Ok(())
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

    pub fn get_client_id(&self) -> u64 {
        self.client.as_ref().unwrap().client_id()
    }
}

pub fn do_multiplayer_disconnect(mut r_client: ResMut<RenetClient>) {
    println!("doing disconnect...");
    r_client.disconnect();
}

pub fn do_finish_disconnect(
    r_client: ResMut<RenetClient>,
    mut commands: Commands,
    mut players: Query<(Entity, &ClientId, Option<&FirstPerson>)>,
) {
    match r_client.client.as_ref() {
        Some(client) => {
            if client.is_disconnected() {
                players
                    .iter_mut()
                    .for_each(|(entity, _client_id, first_person)| {
                        println!("removing entity: {} for disconnect.", entity);
                        match first_person {
                            Some(_) => {
                                commands.entity(entity).remove::<ClientId>();
                            }
                            None => {
                                commands.entity(entity).despawn_recursive();
                            }
                        }
                    });

                commands.remove_resource::<MultiplayerMessageSender>();
                commands.remove_resource::<MultiplayerMessageReceiver>();
                commands.remove_resource::<RenetClient>();
                println!("disconnected.");
            }
        }
        None => {
            println!("do_finish_disconnect: Client is not disconnected in disconnecting state??");
        }
    }
}

pub fn do_multiplayer_server(mut r_client: ResMut<RenetClient>) {
    if let Err(e) = r_client.server_transact() {
        println!("Error transacting with server. {}", e);
    }
}

pub fn set_connected(mut next_multiplayer: ResMut<NextState<MultiplayerState>>) {
    next_multiplayer.set(MultiplayerState::Connected);
}

pub fn set_disconnected(mut next_multiplayer: ResMut<NextState<MultiplayerState>>) {
    next_multiplayer.set(MultiplayerState::Disconnected);
}

pub struct ClientPlugin;
impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            do_multiplayer_server
                .run_if(in_state(MultiplayerState::Connected).and(resource_exists::<RenetClient>)),
        );
        app.add_systems(
            OnEnter(MultiplayerState::Disconnecting),
            do_multiplayer_disconnect.run_if(resource_exists::<RenetClient>),
        );
        app.add_systems(
            Update,
            do_finish_disconnect.run_if(
                in_state(MultiplayerState::Disconnecting).and(resource_exists::<RenetClient>),
            ),
        );
        app.add_systems(OnEnter(MultiplayerState::Connecting), set_connected);

        app.add_systems(
            Update,
            set_disconnected.run_if(resource_removed::<RenetClient>),
        );
    }
}

#[cfg(test)]
mod test {

    use crate::client::MultiplayerMessage;
    use crate::server::*;
    use bevy::prelude::*;
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

    #[test]
    fn test_multiplayermessage_connect() {
        let mess = MultiplayerMessage::Connect {
            client_id: 77u64,
            location: Vec3::new(1.1f32, 2.2f32, 3.3f32),
            direction: Vec3::new(4.4f32, 5.5f32, 6.6f32),
            name: "ikky".to_string(),
        };
        let buf = mess.get_buf().unwrap();
        println!("buf:{:?}", buf);
        let connect: MultiplayerMessage = MultiplayerMessage::get(&buf).unwrap().into();
        match connect {
            MultiplayerMessage::Connect {
                client_id,
                location,
                direction,
                name,
            } => {
                assert_eq!(client_id, 77);
                assert_eq!(location, Vec3::new(1.1f32, 2.2f32, 3.3f32));
                assert_eq!(direction, Vec3::new(4.4f32, 5.5f32, 6.6f32));
                assert_eq!(name, "ikky".to_string());
            }
            _ => panic!("test_multiplayermessage_connect fail!"),
        };
    }

    #[test]
    fn test_multiplayermessage_disconnect() {
        let mess = MultiplayerMessage::Disconnect { client_id: 77u64 };
        let buf = mess.get_buf().unwrap();
        println!("buf:{:?}", buf);
        let disconnect: MultiplayerMessage = MultiplayerMessage::get(&buf).unwrap().into();
        match disconnect {
            MultiplayerMessage::Disconnect { client_id } => {
                assert_eq!(client_id, 77);
            }
            _ => panic!("test_multiplayermessage_connect fail!"),
        }
    }
    #[test]
    fn test_multiplayermessage_move() {
        let mess = MultiplayerMessage::Move {
            client_id: 78u64,
            location: Vec3::new(1., 2., 3.),
        };
        let buf = mess.get_buf().unwrap();
        let move_msg: MultiplayerMessage = MultiplayerMessage::get(&buf).unwrap().into();
        match move_msg {
            MultiplayerMessage::Move {
                client_id,
                location,
            } => {
                assert_eq!(client_id, 78);
                assert_eq!(location.x, 1.);
                assert_eq!(location.y, 2.);
                assert_eq!(location.z, 3.);
            }
            _ => panic!("test_multiplayermessage_connect fail!"),
        }
    }

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
