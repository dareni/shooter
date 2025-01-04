use bevy::input::keyboard::KeyboardInput;
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use bevy::render::camera::Viewport;
use bevy::render::mesh::VertexAttributeValues;
use bevy_egui::EguiPlugin;
use std::f32::consts::PI;

use crate::client::ClientPlugin;
use crate::input_n_state::{InputNStatePlugin, AppParams, initialise_app};
use crate::menu::MenuPlugin;
use crate::players::PlayersPlugin;
use crate::server::server_main;

mod client;
mod config;
mod input_n_state;
mod menu;
mod players;
mod server;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let exec_type = &args[1];
        match exec_type.as_str() {
            "--server" => {
                println!("Starting server...");
                server_main();
                return;
            }
            "--help" => {
                println!("--config [alternate filename]");
                return;
            }

            _ => {}
        }
    }
    let mut app: App = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(EguiPlugin);
    app.add_plugins(InputNStatePlugin);
    app.add_plugins(MenuPlugin);
    app.add_plugins(ClientPlugin);
    app.add_plugins(PlayersPlugin);
    app.add_systems(Startup, setup.after(initialise_app));
    app.add_systems(Update, (move_cube, rotate_on_timer));
    app.run();
}

#[derive(Component)]
struct Cube;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    app_params: Res<AppParams>,
    asset_server: Res<AssetServer>,
) {
    // plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(100.0, 100.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));

    // cube
    // Assign vertex colors based on vertex positions
    let mut colorful_cube = Mesh::from(Cuboid::default());
    if let Some(VertexAttributeValues::Float32x3(positions)) =
        colorful_cube.attribute(Mesh::ATTRIBUTE_POSITION)
    {
        let colors: Vec<[f32; 4]> = positions
            .iter()
            .map(|[r, g, b]| [(1. - *r) / 2., (1. - *g) / 2., (1. - *b) / 2., 1.])
            .collect();
        colorful_cube.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
    commands
        .spawn((
            Mesh3d(meshes.add(colorful_cube)),
            MeshMaterial3d(materials.add(Color::srgb(1., 1., 1.))),
            Transform::from_xyz(0.0, 0.5, 0.0),
        ))
        .insert(Cube);

    // Light
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
        CascadeShadowConfigBuilder {
            minimum_distance: 1.0,
            first_cascade_far_bound: 4.0,
            maximum_distance: 100.0,
            ..default()
        }
        .build(),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 1,
            viewport: Some(Viewport {
                physical_size: UVec2 {
                    x: app_params.window_size.x as u32,
                    y: app_params.window_size.y as u32
                },
                physical_position: UVec2 { x: 000, y: 000 },
                ..default()
            }),
            ..default()
        },
        Transform::from_xyz(50.0, 15.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Transform::from_xyz(-2.0, 4.0, 0.0),
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("littleman.glb"))),
        FirstPerson {},
    ));
}

#[derive(Component)]
pub struct FirstPerson {}

#[derive(Component)]
pub struct ClientId {
    id: u64,
}

fn move_cube(
    mut player_query: Query<&mut Transform, With<Cube>>,
    _char_input_events: EventReader<KeyboardInput>,
) {
    let offset = Vec3::ZERO;
   // for event in char_input_events.read() {
   //     if event.state.is_pressed() {
   //         match event.key_code {
   //             KeyCode::KeyW => offset.z += 0.1,
   //             KeyCode::KeyS => offset.z -= 0.1,
   //             KeyCode::KeyA => offset.x -= 0.1,
   //             KeyCode::KeyD => offset.x += 0.1,
   //             KeyCode::KeyQ => offset.y += 0.1,
   //             KeyCode::KeyE => offset.y -= 0.1,
   //             _ => {}
   //         }
   //     }
   // }
    // Don't bother running the rest of the function if there's no offset
    if offset == Vec3::ZERO {
        return;
    }

    // Move the player
    if let Ok(mut player) = player_query.get_single_mut() {
        player.translation += offset;
    }
}

fn rotate_on_timer(time: Res<Time>, mut query: Query<&mut Transform, With<Cube>>) {
    for mut transform in query.iter_mut() {
        transform.rotate(Quat::from_rotation_y(time.delta_secs()));
    }
}
