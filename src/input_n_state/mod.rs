use crate::config::do_read_config;
use crate::config::get_file;
use bevy::prelude::*;
use bevy_inspector_egui::prelude::*;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_inspector_egui::InspectorOptions;

#[derive(serde::Deserialize, serde::Serialize, Resource)]
pub struct AppParamsInput {
    pub player_name: String,
    pub window_size_x: String,
    pub window_size_y: String,
}
impl AppParamsInput {
    pub fn new(app_params: &AppParams) -> AppParamsInput {
        AppParamsInput {
            player_name: app_params.player_name.clone(),
            window_size_x: app_params.window_size.x.clone().to_string(),
            window_size_y: app_params.window_size.y.clone().to_string(),
        }
    }

    pub fn from(&mut self, app_params: &AppParams) {
        self.player_name = app_params.player_name.clone();
        self.window_size_x = app_params.window_size.x.clone().to_string();
        self.window_size_y = app_params.window_size.y.clone().to_string();
    }

    pub fn to(&self, app_params: &mut AppParams) {
        app_params.player_name = self.player_name.clone();
        app_params.window_size = Vec2::new(
            self.window_size_x.parse::<f32>().unwrap(),
            self.window_size_y.parse::<f32>().unwrap(),
        );
    }
}

#[derive(serde::Deserialize, serde::Serialize, Resource, Reflect, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
pub struct AppParams {
    pub player_name: String,
    pub window_size: Vec2,
    pub config_file: Option<String>,
    pub changed: bool,
}
impl AppParams {
    pub fn dup(&self) -> AppParams {
        AppParams {
            player_name: self.player_name.clone(),
            window_size: self.window_size.clone(),
            config_file: self.config_file.clone(),
            changed: self.changed,
        }
    }
    pub fn default() -> AppParams {
        AppParams {
            player_name: "player1".to_string(),
            window_size: Vec2::new(640.0, 480.0),
            config_file: None,
            changed: true,
        }
    }
}

#[derive(Resource)]
pub struct DevParam {
    pub on: bool,
}

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,
    Game,
    GameOver,
}

#[derive(SubStates, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
#[source(AppState = AppState::MainMenu)]
pub enum MenuItem {
    Config,
    #[default]
    Players,
}

pub struct InputNStatePlugin;
impl Plugin for InputNStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>();
        app.add_sub_state::<MenuItem>();
        app.add_systems(Startup, initialise_app);
        app.add_systems(
            Update,
            game_keys
                .run_if(in_state(AppState::Game))
                .run_if(resource_changed::<ButtonInput<KeyCode>>),
        );
        app.add_systems(OnEnter(AppState::GameOver), app_exit);
        app.add_plugins(WorldInspectorPlugin::default().run_if(do_world_inspector()));
        app.add_plugins(
            ResourceInspectorPlugin::<AppParams>::default().run_if(do_world_inspector()),
        );
    }
}

fn initialise_app(
    mut commands: Commands,
    mut windows: Query<&mut Window>,
    mut next_item: ResMut<NextState<MenuItem>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    //Check for commandline config.
    let args: Vec<String> = std::env::args().collect();
    let config_file_path: Option<String> = {
        if args.len() > 2 {
            let parameter = &args[1];
            match parameter.as_str() {
                "--config" => {
                    //Check the file exists.
                    let file_result = get_file(&args[2].clone().into(), false);
                    match file_result {
                        Ok(_) => Some(args[2].clone().into()),
                        Err(e) => {
                            println!("--config path arg error: {}", e);
                            None
                        }
                    }
                }
                _ => None,
            }
        } else {
            None
        }
    };

    //config file is Some for the command line config option.
    let params = match do_read_config(config_file_path.clone()) {
        Ok(param) => param,
        Err(e) => {
            println!("Failed to read configuration, using defaults. (input_n_state) {}", e);
            next_state.set(AppState::MainMenu);
            next_item.set(MenuItem::Config);
            let mut apps = AppParams::default();
            apps.config_file = config_file_path;
            apps.changed = true;
            apps
        }
    };
    let mut window = windows.single_mut();
    window
        .resolution
        .set(params.window_size.x, params.window_size.y);
    commands.insert_resource(params);
}

fn game_keys(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) || keyboard_input.just_pressed(KeyCode::KeyX) {
        next_state.set(AppState::GameOver);
    } else if keyboard_input.just_pressed(KeyCode::KeyM) {
        next_state.set(AppState::MainMenu);
    } else if keyboard_input.just_pressed(KeyCode::KeyG) {
        next_state.set(AppState::Game);
    }

    // Add for loop
    //for key in keys.get_pressed() {
    //    println!("{:?} is currently held down", key);
    //}
    //for key in keys.get_just_pressed() {
    //    println!("{:?} was pressed", key);
    //}
}

fn app_exit(mut app_exit_event_writer: EventWriter<AppExit>) {
    app_exit_event_writer.send(AppExit::Success);
}

fn do_world_inspector() -> impl Condition<()> {
    IntoSystem::into_system(|param: Res<DevParam>| {
        if param.on {
            return true;
        }
        false
    })
}
