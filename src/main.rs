#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod blocks;
mod chunk;
mod player;
mod states;
mod ui;
mod world;
mod worldgen;

use bevy::prelude::*;
use bevy::render::settings::{Backends, WgpuSettings};
use bevy::render::RenderPlugin;
use iyes_perf_ui::prelude::*;

use blocks::TextureSet;
use states::{ActiveWorld, AppState};

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "MC-Rust".into(),
                        resolution: (1280.0_f32, 720.0_f32).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: WgpuSettings {
                        backends: Some(Backends::VULKAN),
                        ..default()
                    }
                    .into(),
                    ..default()
                }),
        )
        // Диагностика — обязательна для iyes_perf_ui
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugins(bevy::diagnostic::EntityCountDiagnosticsPlugin::default())
        // Сам оверлей
        .add_plugins(PerfUiPlugin)
        .init_state::<AppState>()
        .init_resource::<TextureSet>()
        .init_resource::<ActiveWorld>()
        .add_plugins((ui::UiPlugin, world::WorldPlugin, player::PlayerPlugin))
        .add_systems(Startup, (spawn_ui_camera, spawn_perf_ui))
        .run();
}

fn spawn_ui_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            ..default()
        },
    ));
}

fn spawn_perf_ui(mut commands: Commands) {
    commands.spawn(PerfUiDefaultEntries::default());
}