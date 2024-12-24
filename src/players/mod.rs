use crate::client::*;
use crate::input_n_state::*;
use crate::*;
use bevy::prelude::*;

pub struct PlayersPlugin;
impl Plugin for PlayersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_world_from_server_messages.run_if(resource_exists::<MultiplayerMessageReceiver>),
        );
        app.add_systems(
            OnEnter(MultiplayerState::Connected),
            connect_first_person.run_if(resource_exists::<MultiplayerMessageReceiver>),
        );
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
                direction,
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
