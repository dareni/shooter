use crate::client::*;
use bevy::prelude::*;

pub struct PlayersPlugin;
impl Plugin for PlayersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_world_from_server_messages.run_if(resource_exists::<MultiplayerMessageReceiver>),
        );
    }
}

pub fn update_world_from_server_messages(
    receiver: ResMut<MultiplayerMessageReceiver>,
    mut commands: Commands,
) {
    for message in receiver.receiver.lock().expect("").try_iter() {
        match message {
            MultiplayerMessage::Connect { .. } => {}
            MultiplayerMessage::Disconnect => {
                commands.remove_resource::<RenetClient>();
                commands.remove_resource::<MultiplayerMessageSender>();
                commands.remove_resource::<MultiplayerMessageReceiver>();
                println!("Client disconnected.");
            }
            MultiplayerMessage::None => {}
        }
    }
}
