use bevy::prelude::*;
use renetcode::{ClientAuthentication, ConnectToken, NetcodeClient, NETCODE_MAX_PACKET_BYTES};
use std::{
    net::{SocketAddr, UdpSocket},
    time::Instant,
};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::server::*;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum Multiplayer {
    #[default]
    Disconnected,
    Disconnecting,
    Connected,
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
            //let (addr, payload) = client.generate_payload_packet(text.as_bytes()).unwrap();
            //   udp_socket.send_to(payload, addr).unwrap();
            //} else {
            //   println!("Client is not yet connected");
            //}
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
        app.add_systems(OnEnter(Multiplayer::Disconnecting), do_multiplayer_disconnect);

    }

}

#[cfg(test)]
mod  test {

use renetcode::{ClientAuthentication, ConnectToken, NetcodeClient, NETCODE_MAX_PACKET_BYTES};
use std::{
    net::{SocketAddr, UdpSocket},
    time::Instant,
};
use std::{
    sync::mpsc::{
        self,
        Receiver, TryRecvError
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use crate::server::*;


pub fn client_main(user_name: String) {
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
                      let (addr, payload) = client.generate_payload_packet(text.as_bytes()).unwrap();
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

  //#[test]
  pub fn test_client_connect() {
      client_main("ikky".to_string());
  }
}
