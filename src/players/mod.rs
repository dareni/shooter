use crate::client::*;
use crate::input_n_state::*;
use crate::*;
use bevy::prelude::*;

#[derive(Event)]
pub struct PlayerMovementEvent(pub Movement);

#[derive(Event)]
pub struct PlayerRotateEvent(pub Vec2);

#[derive(serde::Deserialize, serde::Serialize, Resource)]
pub struct MouseRotation(pub Vec2);

#[derive(Debug)]
pub enum Movement {
    Forward,
    Back,
    Left,
    Right,
}

const MOUSE_SENSITIVITY: f32 = 0.001;

pub struct PlayersPlugin;
impl Plugin for PlayersPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PlayerMovementEvent>();
        app.add_event::<PlayerRotateEvent>();
        app.insert_resource(MouseRotation(Vec2::ZERO));
        app.add_systems(
            Update,
            update_world_from_server_messages.run_if(resource_exists::<MultiplayerMessageReceiver>),
        );
        app.add_systems(Update, keyboard_move_cmd);
        app.add_systems(Update, mouse_move_cmd);
        app.add_systems(
            OnEnter(MultiplayerState::Connected),
            connect_first_person.run_if(resource_exists::<MultiplayerMessageReceiver>),
        );
    }
}

fn mouse_move_cmd(
    mut player_rotate: EventReader<PlayerRotateEvent>,
    mut mouse_rotation: ResMut<MouseRotation>,
    mut camera: Query<&mut Transform, With<ActiveCamera>>,
) {
    let mut transform = camera.get_single_mut().unwrap();
    for rotation in player_rotate.read() {
        let PlayerRotateEvent(delta) = rotation;

        mouse_rotation.0.x -= delta.x * MOUSE_SENSITIVITY;
        mouse_rotation.0.y -= delta.y * MOUSE_SENSITIVITY;

        let x_quat = Quat::from_axis_angle(Vec3::new(0., 1., 0.), mouse_rotation.0.x);

        let y_quat = Quat::from_axis_angle(Vec3::new(1., 0., 0.), mouse_rotation.0.y);

        transform.rotation = x_quat * y_quat;
    }
}

fn keyboard_move_cmd(
    mut player_movement: EventReader<PlayerMovementEvent>,
    mut camera: Query<&mut Transform, With<ActiveCamera>>,
) {
    let mut transform = camera.get_single_mut().unwrap();

    for mv in player_movement.read() {
        println!("{:?}", mv.0);
        match mv {
            PlayerMovementEvent(Movement::Forward) => {
                let forward: Dir3 = transform.forward();
                transform.translation += *forward;
            }
            PlayerMovementEvent(Movement::Back) => {
                let back: Dir3 = transform.back();
                //transform.rotate
                transform.translation += *back;
            }
            PlayerMovementEvent(Movement::Left) => {
                let left: Dir3 = transform.left();
                transform.translation += *left;
            }
            PlayerMovementEvent(Movement::Right) => {
                let right: Dir3 = transform.right();
                transform.translation += *right;
            } //_  => {}
        }
    }
}

pub fn connect_first_person(
    mut commands: Commands,
    fp_entity_query: Query<Entity, With<FirstPerson>>,
    r_client: ResMut<RenetClient>,
) {
    let cid = r_client.get_client_id();
    match fp_entity_query.get_single() {
        Ok(entity_id) => {
            commands.entity(entity_id).insert(ClientId { id: cid });
            println!("Added clientid to FirstPerson: {}", cid);
        }
        Err(e) => {
            eprintln!("Error adding clientid to FirstPerson: {}", e);
        }
    }
}

pub fn update_world_from_server_messages(
    receiver: ResMut<MultiplayerMessageReceiver>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut players: Query<(Entity, &ClientId, &mut Transform, Option<&FirstPerson>)>,
) {
    for message in receiver.receiver.lock().expect("").try_iter() {
        match message {
            MultiplayerMessage::Connect {
                client_id,
                pos,
                direction: _,
                name,
            } => {
                let mut is_spawned = false;
                players.iter_mut().for_each(|(_entity, cid, mut transform, first_person)| {
                      if client_id == cid.id  {
                          is_spawned = true;
                          match first_person {
                              Some(_) => {
                                  transform.translation = pos;
                                  println!("Littleman connected and positioned.");
                              }
                              None => {
                                  eprintln!("Player is already spawned? //
                                  Should only get a new player and it's id should not exist as an entity??.");
                              }
                          }
                      }
                  });
                println!("received connect message for {}", name);
                if is_spawned == false {
                    println!("spawn player {}", name);
                    commands.spawn((
                        Name::new(name),
                        Transform::from_translation(pos),
                        ClientId { id: client_id },
                        SceneRoot(
                            asset_server.load(GltfAssetLabel::Scene(0).from_asset("littleman.glb")),
                        ),
                    ));
                }
            }
            MultiplayerMessage::Disconnect { client_id } => {
                players.iter_mut().for_each(|(entity, cid, _, first_person)| {
                    if client_id == cid.id  {
                        match first_person {
                            Some(_) => {
                                eprintln!("Disconnect is not propagated by the payload layer?? cid:{}", client_id);
                            }
                            None => {
                                //remove the disconnected player.
                                println!("disconnect player cid:{}", client_id);
                                commands.entity(entity).despawn_recursive();
                            }
                        }
                    }
                });
            }
            MultiplayerMessage::None => {
                println!("Received Multiplayer::None?");
            }
        }
    }
}
