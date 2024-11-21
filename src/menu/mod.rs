use bevy::prelude::*;
use bevy_egui::egui::menu;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui::containers::panel::TopBottomPanel;
use egui::pos2;

use crate::config::*;
use crate::input_n_state::*;


pub struct MenuPlugin;

impl Plugin for MenuPlugin {
  fn build(&self, app: &mut App) {
    app.add_plugins(EguiPlugin);
    app.add_systems(Startup, setup_menu);
    app.add_systems(Update, spawn_main_menu.run_if(in_state(AppState::MainMenu)));
    app.add_systems(Update, spawn_config_window.run_if(in_state(MenuItem::Config)));
    app.add_systems(Update, spawn_player_window.run_if(in_state(MenuItem::Players)));
  }
}

pub fn spawn_main_menu(
    mut contexts: EguiContexts,
    mut next_item: ResMut<NextState<MenuItem>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut is_w_inspect: ResMut<DevParam>,
) {
    TopBottomPanel::top("menu_bar").show(contexts.ctx_mut(), |ui| {
        menu::bar(ui, |ui| {
            let open = ui.button("Config");
            if open.clicked() {
                next_item.set(MenuItem::Config);
                ui.close_menu();
            }
            if ui.button("Players").clicked() {
                next_item.set(MenuItem::Players);
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

pub fn spawn_config_window(
    mut contexts: EguiContexts,
    mut app_params: ResMut<AppParams>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    bevy_egui::egui::Window::new("shooter config")
        .collapsible(false)
        .default_pos(pos2(30.0, 50.0))
        .show(contexts.ctx_mut(), |ui| {
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("name:");
                ui.text_edit_singleline(&mut app_params.player_name);
            });
            if ui.button("Ok").clicked() {
                let copy: AppParams = app_params.dup();
                match do_write_config(&copy) {
                    Ok(_) => (),
                    Err(e) => println!("Failed to write config file. {}", e),
                }
                next_state.set(AppState::Game);
                ui.close_menu();
            };
        });
}

pub fn spawn_player_window(
    mut contexts: EguiContexts,
    mut next_state: ResMut<NextState<AppState>>,
) {
    bevy_egui::egui::Window::new("players ingame").show(contexts.ctx_mut(), |ui| {
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Players:");
        });
        if ui.button("Ok").clicked() {
            next_state.set(AppState::Game);
            ui.close_menu();
        };
    });
}

pub fn setup_menu(mut commands: Commands, mut contexts: EguiContexts) {
    let con = contexts.ctx_mut();
    con.set_theme(egui::Theme::Light);

    let params = match do_read_config() {
        Ok(param) => param,
        Err(e) => {
            println!("Failed to read configuration, using defaults. {}", e);
            AppParams::default()
        }
    };
    commands.insert_resource(params);
    commands.insert_resource(DevParam { on: false });
}
