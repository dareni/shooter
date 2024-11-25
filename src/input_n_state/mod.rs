use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

#[derive(serde::Deserialize, serde::Serialize, Resource)]
pub struct AppParams {
    pub player_name: String,
}
impl AppParams {
    pub fn dup(&self) -> AppParams {
        AppParams {
            player_name: self.player_name.clone(),
        }
    }
    pub fn default() -> AppParams {
        AppParams {
            player_name: "player1".to_string(),
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
