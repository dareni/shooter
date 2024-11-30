use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

#[derive(serde::Deserialize, serde::Serialize, Resource)]
pub struct AppParamsInput {
    pub player_name: String,
    pub window_size_x: String,
    pub window_size_y: String,
}
impl AppParamsInput {
    pub fn new(app_params: &AppParams) -> AppParamsInput {
        AppParamsInput{
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

#[derive(serde::Deserialize, serde::Serialize, Resource)]
pub struct AppParams {
    pub player_name: String,
    pub window_size: Vec2,
}
impl AppParams {
    pub fn dup(&self) -> AppParams {
        AppParams {
            player_name: self.player_name.clone(),
            window_size: self.window_size.clone(),
        }
    }
    pub fn default() -> AppParams {
        AppParams {
            player_name: "player1".to_string(),
            window_size: Vec2::new(640.0, 480.0),
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
        app.add_systems(
            Update,
            game_keys
                .run_if(in_state(AppState::Game))
                .run_if(resource_changed::<ButtonInput<KeyCode>>),
        );
        app.add_systems(OnEnter(AppState::GameOver), app_exit);
        app.add_plugins(WorldInspectorPlugin::default().run_if(do_world_inspector()));
    }
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
