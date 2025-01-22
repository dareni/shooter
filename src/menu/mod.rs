use bevy::prelude::*;
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_egui::{egui::menu, EguiContexts, EguiSet, EguiStartupSet};
use egui::containers::panel::TopBottomPanel;
use egui::{pos2, Color32, Visuals};
use regex::Regex;
use std::sync::Mutex;
use std::time::Duration;

use crate::client::*;
use crate::config::*;
use crate::input_n_state::*;
use crate::server::Server;
use crate::ActiveCamera;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_menu);
        app.add_systems(Startup, low_fps);
        app.add_systems(
            Update,
            (
                spawn_main_menu.run_if(in_state(AppState::MainMenu)),
                spawn_config_window.run_if(in_state(MenuItem::Config)),
                spawn_player_window.run_if(in_state(MenuItem::Players)),
                spawn_server_window.run_if(in_state(MenuItem::Servers)),
            )
                .after(EguiStartupSet::InitContexts)
                .after(EguiSet::InitContexts),
        );

        app.add_systems(OnEnter(AppState::Game), high_fps);
        app.add_systems(OnExit(AppState::Game), low_fps);
        app.add_systems(OnEnter(MenuItem::ActivateCamera), activate_camera);

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
                if ui.button("Toggle Camera").clicked() {
                    next_item.set(MenuItem::ActivateCamera);
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

pub fn save_config(app_params: &AppParams) -> Result<(), String> {
    let mut copy: AppParams = app_params.dup();
    copy.changed = false;
    match do_write_config(&copy) {
        Ok(_) => Ok(()),
        Err(e) => {
            println!("Failed to write config file. {}", e);
            Err(e)
        }
    }
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
    mut app_params: ResMut<AppParams>,
    mut commands: Commands,
) {
    bevy_egui::egui::Window::new("Game Servers")
        .collapsible(false)
        .default_pos(pos2(30.0, 50.0))
        .show(contexts.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Server Selection:");
            });
            let visuals = ui.visuals_mut();
            visuals.dark_mode = true;
            use egui_extras::{Column, TableBuilder};
            let mut table = TableBuilder::new(ui).columns(Column::remainder(), 2);
            table = table.sense(egui::Sense::click());
            table
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("Server Name");
                    });
                    header.col(|ui| {
                        ui.strong("Server Connection");
                    });
                })
                .body(|mut body| {
                    let row_height = 18.0;
                    let mut row_index: i8 = 0;
                    let mut idx: i8 = app_params.last_server_index;
                    for game_server in (app_params.server_list.as_ref() as &Vec<Server>).into_iter()
                    {
                        if app_params.last_server_index == 0
                            || app_params.last_server_index * -1 != row_index
                        {
                            body.row(row_height, |mut row| {
                                if row_index == app_params.last_server_index {
                                    row.set_selected(true);
                                }
                                row.col(|ui| {
                                    ui.label(&game_server.name);
                                });
                                row.col(|ui| {
                                    ui.label(&game_server.url);
                                });
                                if row.response().clicked() {
                                    idx = row_index;
                                    println!("selected");
                                }
                            });
                        }
                        row_index = row_index + 1;
                    }
                    if !app_params.last_server_index < 0 {
                        app_params.set_last_server_index(idx);
                    }
                });
            ui.separator();
            ui.label("Server Configuration:");
            ui.horizontal(|ui| {
                if app_params.last_server_index >= 0 {
                    if ui.button("Add").clicked() {
                        app_params.server_list.push(Server {
                            name: "".to_string(),
                            url: "".to_string(),
                        });
                        let mut add_idx = app_params.server_list.len() as i8 - 1;
                        add_idx *= -1;
                        app_params.last_server_index = add_idx;
                    };
                }
                if app_params.last_server_index != 0 {
                    if app_params.last_server_index > 0 {
                        if ui.button("Edit").clicked() {
                            app_params.last_server_index *= -1;
                        };
                    }
                    if app_params.last_server_index < 0 {
                        if ui.button("Save").clicked() {
                            app_params.last_server_index *= -1;
                            if let Ok(_) = save_config(&app_params) {
                                app_params.changed = false;
                            };
                        };
                    }
                    if app_params.server_list.len() > 1 {
                        if ui.button("Delete").clicked() {
                            if app_params.last_server_index < 0 {
                                app_params.last_server_index *= -1;
                            }
                            let rm_indx = app_params.last_server_index as usize;
                            app_params.server_list.remove(rm_indx);
                            app_params.last_server_index -= 1;
                        };
                    }
                }
            });

            if app_params.last_server_index < 0 {
                ui.horizontal(|ui| {
                    ui.label("Edit Server:");
                });
                let idx = app_params.last_server_index * -1;
                let server = &mut app_params.server_list[idx as usize];
                ui.horizontal(|ui| {
                    let width = ui.available_width() / 2.0 - 5.0;
                    ui.scope(|ui| {
                        ui.set_max_width(width);
                        ui.label("name:");
                        ui.text_edit_singleline(&mut server.name);
                    });
                    ui.scope(|ui| {
                        ui.set_max_width(width);
                        ui.label("url:");
                        ui.text_edit_singleline(&mut server.url);
                    });
                });
            }
            ui.separator();

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

pub fn activate_camera(
    mut next_menu_item: ResMut<NextState<MenuItem>>,
    mut cameras: Query<(Entity, &mut Camera, Option<&ActiveCamera>)>,
    mut commands: Commands,
) {
    next_menu_item.set(MenuItem::None);
    cameras
        .iter_mut()
        .for_each(|(entityid, mut camera, activecamera)| match activecamera {
            Some(_) => {
                camera.is_active = false;
                commands.entity(entityid).remove::<ActiveCamera>();
            }
            None => {
                camera.is_active = true;
                commands.entity(entityid).insert(ActiveCamera {});
            }
        });
}

pub fn setup_menu(mut commands: Commands, mut contexts: EguiContexts) {
    let con = contexts.ctx_mut();
    con.set_theme(egui::Theme::Light);
    let mut vis = Visuals::light();
    vis.extreme_bg_color = Color32::LIGHT_GRAY;
    con.set_visuals_of(egui::Theme::Light, vis);
    commands.insert_resource(DevParam { on: false });
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

fn high_fps(mut winit: ResMut<WinitSettings>) {
    winit.focused_mode = UpdateMode::reactive(Duration::from_secs_f32(1.0 / 60.0));
    winit.unfocused_mode = UpdateMode::reactive(Duration::from_secs_f32(1.0 / 1.0));
}

fn low_fps(mut winit: ResMut<WinitSettings>) {
    winit.focused_mode = UpdateMode::reactive(Duration::from_secs_f32(1.0 / 1.0));
    winit.unfocused_mode = UpdateMode::reactive(Duration::from_secs_f32(1.0 / 1.0));
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
