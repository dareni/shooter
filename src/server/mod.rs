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
    let mut received_messages = vec![];
    let mut last_updated = Instant::now();
    let mut buffer = [0u8; NETCODE_MAX_PACKET_BYTES];
    let mut usernames: HashMap<u64, String> = HashMap::new();
    let mut last_ping = Instant::now();
    loop {
        server.update(Instant::now() - last_updated);
        received_messages.clear();

        loop {
            match udp_socket.recv_from(&mut buffer) {
                Ok((len, addr)) => {
                    // println!("Received decrypted message {:?} from {}.", &buffer[..len], addr);
                    let server_result = server.process_packet(addr, &mut buffer[..len]);
                    handle_server_result(
                        server_result,
                        &udp_socket,
                        &mut received_messages,
                        &mut usernames,
                    );
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => panic!("Socket error: {}", e),
            };
        }

        for text in received_messages.iter() {
            for client_id in server.clients_id().iter() {
                let (addr, payload) = server
                    .generate_payload_packet(*client_id, text.as_bytes())
                    .unwrap();
                udp_socket.send_to(payload, addr).unwrap();
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
                &mut received_messages,
                &mut usernames,
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
    received_messages: &mut Vec<String>,
    usernames: &mut HashMap<u64, String>,
) {
    match server_result {
        ServerResult::Payload { client_id, payload } => {
            let text = String::from_utf8(payload.to_vec()).unwrap();
            let username = usernames.get(&client_id).unwrap();
            println!(
                "Client {} ({}) sent message {:?}.",
                username, client_id, text
            );
            let text = format!("{}: {}", username, text);
            received_messages.push(text);
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
            usernames.insert(client_id, username.0);
            socket.send_to(payload, addr).unwrap();
        }
        ServerResult::ClientDisconnected {
            client_id,
            addr,
            payload,
        } => {
            println!("Client {} disconnected.", client_id);
            usernames.remove_entry(&client_id);
            if let Some(payload) = payload {
                socket.send_to(payload, addr).unwrap();
            }
        }
        ServerResult::None => {}
    }
}
