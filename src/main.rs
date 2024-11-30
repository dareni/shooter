use bevy::input::keyboard::KeyboardInput;
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use bevy::render::camera::Viewport;
use bevy::render::mesh::VertexAttributeValues;
use std::f32::consts::PI;

use crate::input_n_state::InputNStatePlugin;
use crate::menu::MenuPlugin;
use crate::server::server_main;
use crate::client::ClientPlugin;

mod client;
mod config;
mod input_n_state;
mod menu;
mod server;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let exec_type = &args[1];
        match exec_type.as_str() {
            "server" => {
                println!("Starting server...");
                server_main();
            }
            "config" => {
            }
            _ => {}
        }
    } else {
        let mut app:App = App::new();
            app.add_plugins(DefaultPlugins);
            app.add_plugins(InputNStatePlugin);
            app.add_plugins(MenuPlugin);
            app.add_plugins(ClientPlugin);
            app.add_systems(Startup, setup);
            app.add_systems(Update, (move_cube, rotate_on_timer));
            app.run();
    }
}

#[derive(Component)]
struct Cube;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(100.0, 100.0)),
        material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
        ..default()
    });
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
        .spawn(PbrBundle {
            mesh: meshes.add(colorful_cube),
            // This is the default color, but note that vertex colors are
            // multiplied by the base color, so you'll likely want this to be
            // white if using vertex colors.
            material: materials.add(Color::srgb(1., 1., 1.)),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        })
        .insert(Cube);

    // Light
    //commands.spawn(PointLightBundle {
    //    point_light: PointLight {
    //        intensity: 100_000_000.0,
    //        shadows_enabled: true,
    //        ..default()
    //    },
    //    transform: Transform::from_xyz(4.0, 15.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    //    ..default()
    //});

    // ambient light
    //commands.insert_resource(AmbientLight {
    //    color: WHITE.into(),
    //    brightness: 200.02,
    //});
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
        // The default cascade config is designed to handle large scenes.
        // As this example has a much smaller world, we can tighten the shadow
        // bounds for better visual quality.
        cascade_shadow_config: CascadeShadowConfigBuilder {
            first_cascade_far_bound: 4.0,
            maximum_distance: 100.0,
            ..default()
        }
        .into(),
        ..default()
    });

    // Camera
    commands.spawn(Camera3dBundle {
        //transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        transform: Transform::from_xyz(50.0, 15.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
        camera: Camera {
            order: 1,
            viewport: Some(Viewport {
                physical_size: UVec2 { x: 200, y: 200 },
                physical_position: UVec2 { x: 000, y: 000 },
                ..default()
            }),
            ..default()
        },
        ..default()
    });

    commands.spawn(SceneBundle {
        transform: Transform::from_xyz(-1.0, 4.0, 0.0),
        scene: asset_server.load(GltfAssetLabel::Scene(0).from_asset("littleman.glb")),
        ..default()
    });
}

fn move_cube(
    mut player_query: Query<&mut Transform, With<Cube>>,
    mut char_input_events: EventReader<KeyboardInput>,
) {
    let mut offset = Vec3::ZERO;
    for event in char_input_events.read() {
        if event.state.is_pressed() {
            match event.key_code {
                KeyCode::KeyW => offset.z += 0.1,
                KeyCode::KeyS => offset.z -= 0.1,
                KeyCode::KeyA => offset.x -= 0.1,
                KeyCode::KeyD => offset.x += 0.1,
                KeyCode::KeyQ => offset.y += 0.1,
                KeyCode::KeyE => offset.y -= 0.1,
                _ => {}
            }
        }
    }
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
        transform.rotate(Quat::from_rotation_y(time.delta_seconds()));
    }
}
