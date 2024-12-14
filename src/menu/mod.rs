use bevy::prelude::*;
use bevy_egui::{egui::menu, EguiContexts};
use egui::containers::panel::TopBottomPanel;
use egui::pos2;
use regex::Regex;
use std::sync::Mutex;

use crate::client::*;
use crate::config::*;
use crate::input_n_state::*;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_menu);
        app.add_systems(Update, spawn_main_menu.run_if(in_state(AppState::MainMenu)));
        app.add_systems(
            Update,
            spawn_config_window.run_if(in_state(MenuItem::Config)),
        );
        app.add_systems(
            Update,
            spawn_player_window.run_if(in_state(MenuItem::Players)),
        );
        app.add_systems(
            Update,
            spawn_server_window.run_if(in_state(MenuItem::Servers)),
        );
        app.add_systems(
            OnEnter(MenuItem::Config),
            setup_config_window_params.before(initialise_config_window_params),
        );
        app.add_systems(OnExit(MenuItem::Config), finalise_config_window_params);
    }
}

pub fn spawn_main_menu(
    mut contexts: EguiContexts,
    mut next_item: ResMut<NextState<MenuItem>>,
    mut next_state: ResMut<NextState<AppState>>,
    multiplayer_state: Res<State<MultiplayerState>>,
    mut is_w_inspect: ResMut<DevParam>,
) {
    TopBottomPanel::top("menu_bar").show(contexts.ctx_mut(), |ui| {
        menu::bar(ui, |ui| {
            let open = ui.button("Config");
            if open.clicked() {
                next_item.set(MenuItem::Config);
                ui.close_menu();
            }
            if ui.button("Multiplayer").clicked() {
                match multiplayer_state.get() {
                    MultiplayerState::Connected => {
                        next_item.set(MenuItem::Players);
                    }
                    MultiplayerState::Disconnected => {
                        next_item.set(MenuItem::Servers);
                    }
                    _ => {
                        println!("{:?}", multiplayer_state.get())
                    }
                }
                ui.close_menu();
            }
            if ui.button("Resume Game").clicked() {
                next_state.set(AppState::Game);
                ui.close_menu();
            }
            if ui.button("Exit Game").clicked() {
                next_state.set(AppState::GameOver);
                ui.close_menu();
            }

            let _file_button = ui.menu_button("Options", |ui| {
                if ui.button("Dev").clicked() {
                    is_w_inspect.on = !is_w_inspect.on;
                    ui.close_menu();
                }
            });
        });
    });
}

fn only_numbers_mask(s: &mut String) {
    let re = Regex::new(r"[^0-9]+").expect("band expression");
    *s = re.replace_all(s, "").to_string();
}

fn validate_ok(app_params: &mut AppParamsInput) -> bool {
    if app_params.player_name.len() < 4 {
        println!("Player name must use more than 4 characters");
        return false;
    }
    only_numbers_mask(&mut app_params.window_size_x);
    only_numbers_mask(&mut app_params.window_size_y);
    if app_params.window_size_x.len() < 3 {
        println!("Screen with minimum 100.");
        return false;
    }
    if app_params.window_size_y.len() < 3 {
        println!("Screen with minimum 100.");
        return false;
    }
    true
}

pub fn spawn_config_window(
    mut contexts: EguiContexts,
    mut app_params: ResMut<AppParams>,
    mut app_params_input: ResMut<AppParamsInput>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    bevy_egui::egui::Window::new("shooter config")
        .collapsible(false)
        .default_pos(pos2(30.0, 50.0))
        .show(contexts.ctx_mut(), |ui| {
            //.char_limit(4)
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("name:");
                ui.text_edit_singleline(&mut app_params_input.player_name);
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("window dim (width, height):");
                let width = ui.available_size().x;
                let width = (width - 20.0) / 2.0;
                ui.scope(|ui| {
                    ui.set_max_width(width);
                    if ui
                        .text_edit_singleline(&mut app_params_input.window_size_x)
                        .changed()
                    {
                        only_numbers_mask(&mut app_params_input.window_size_x);
                    }
                });
                ui.scope(|ui| {
                    ui.set_max_width(width);
                    if ui
                        .text_edit_singleline(&mut app_params_input.window_size_y)
                        .changed()
                    {
                        only_numbers_mask(&mut app_params_input.window_size_y);
                    }
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    if validate_ok(&mut app_params_input) {
                        app_params_input.to(&mut app_params);
                        let mut copy: AppParams = app_params.dup();
                        copy.changed = false;
                        match do_write_config(&copy) {
                            Ok(_) => {
                                app_params.changed = false;
                                ui.close_menu();
                            }
                            Err(e) => println!("Failed to write config file. {}", e),
                        }
                    } else {
                        println!("Validation failed.");
                    }
                }

                if app_params.changed == false {
                    if ui.button("Close").clicked() {
                        if app_params.player_name.len() > 3 {
                            next_state.set(AppState::Game);
                            ui.close_menu();
                        } else {
                            println!("Player name is a minumum of 4 characters.");
                        }
                    }
                }
            });
        });
}

pub fn spawn_server_window(
    mut contexts: EguiContexts,
    mut next_multiplayer: ResMut<NextState<MultiplayerState>>,
    mut next_menu_item: ResMut<NextState<MenuItem>>,
    app_params: Res<AppParams>,
    mut commands: Commands,
) {
    bevy_egui::egui::Window::new("Game Servers")
        .collapsible(false)
        .default_pos(pos2(30.0, 50.0))
        .show(contexts.ctx_mut(), |ui| {
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Players:");
            });
            if app_params.player_name.len() > 3 {
                if ui.button("Connect").clicked() {
                    let mut r_client = RenetClient::new();
                    let (sender, rx) = r_client.connect(app_params.player_name.clone());
                    commands.insert_resource(r_client);
                    commands.insert_resource(MultiplayerMessageSender { sender });
                    let receiver = Mutex::new(rx);
                    commands.insert_resource(MultiplayerMessageReceiver { receiver });
                    next_multiplayer.set(MultiplayerState::Connecting);
                    next_menu_item.set(MenuItem::None);
                    ui.close_menu();
                };
            } else {
                println!("Configuration of 'Player Name' required before connecting.");
            }
        });
}

pub fn spawn_player_window(
    mut contexts: EguiContexts,
    mut next_multiplayer: ResMut<NextState<MultiplayerState>>,
    mut next_menu_item: ResMut<NextState<MenuItem>>,
    app_params: Res<AppParams>,
    mut r_client: ResMut<RenetClient>,
) {
    bevy_egui::egui::Window::new("Players Ingame")
        .collapsible(false)
        .default_pos(pos2(30.0, 50.0))
        .show(contexts.ctx_mut(), |ui| {
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Players:");
            });
            if app_params.player_name.len() > 3 {
                if !r_client.is_disconnected() {
                    if ui.button("Disconnect").clicked() {
                        next_multiplayer.set(MultiplayerState::Disconnecting);
                        next_menu_item.set(MenuItem::None);
                        ui.close_menu();
                    };
                }
            } else {
                println!("Configuration of 'Player Name' required before connecting.");
            }
        });
}

pub fn setup_menu(mut commands: Commands, mut contexts: EguiContexts) {
    let con = contexts.ctx_mut();
    con.set_theme(egui::Theme::Light);
    commands.insert_resource(DevParam { on: false });
    //commands.insert_resource(RenetClient::new());
}

pub fn setup_config_window_params(mut commands: Commands, app_params: Res<AppParams>) {
    commands.insert_resource(AppParamsInput::new(&app_params));
}

pub fn initialise_config_window_params(
    app_params: Res<AppParams>,
    mut app_params_input: ResMut<AppParamsInput>,
) {
    app_params_input.from(&app_params);
}

pub fn finalise_config_window_params(mut commands: Commands) {
    commands.remove_resource::<AppParamsInput>();
}

#[test]
fn test_numbers_mask() {
    let mut test_str = "12 34".to_string();
    only_numbers_mask(&mut test_str);
    assert!("1234" == test_str, "1test_str!={}", test_str);

    test_str = "12  34".to_string();
    only_numbers_mask(&mut test_str);
    assert!("1234" == test_str, "2test_str={}", test_str);

    test_str = "sdf!12 $ 34sf".to_string();
    only_numbers_mask(&mut test_str);
    assert!("1234" == test_str, "3test_str={}", test_str);
}
