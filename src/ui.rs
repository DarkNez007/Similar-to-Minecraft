use crate::states::{AppState, WorldConfig};
use crate::worldgen::WorldType;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use crate::world::LoadedChunks;
use crate::states::ActiveWorld;
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldConfig>()
            .add_systems(OnEnter(AppState::MainMenu), spawn_main_menu)
            .add_systems(OnExit(AppState::MainMenu), despawn_all::<MainMenuRoot>)
            .add_systems(OnEnter(AppState::Settings), spawn_settings)
            .add_systems(OnExit(AppState::Settings), despawn_all::<SettingsRoot>)
            .add_systems(OnEnter(AppState::WorldCreate), spawn_world_create)
            .add_systems(OnExit(AppState::WorldCreate), despawn_all::<WorldCreateRoot>)
            .add_systems(
                Update,
                (
                    main_menu_buttons.run_if(in_state(AppState::MainMenu)),
                    settings_buttons.run_if(in_state(AppState::Settings)),
                    (world_create_buttons, seed_input)
                        .chain()
                        .run_if(in_state(AppState::WorldCreate)),
                ),
            )
            .add_systems(OnEnter(AppState::Paused), spawn_pause_menu)
            .add_systems(OnExit(AppState::Paused), despawn_all::<PauseMenuRoot>)
            .add_systems(Update, pause_buttons.run_if(in_state(AppState::Paused)));
    }
}

#[derive(Component)] struct MainMenuRoot;
#[derive(Component)] struct SettingsRoot;
#[derive(Component)] struct WorldCreateRoot;
#[derive(Component)] struct SeedLabel;
#[derive(Component)] struct WorldTypeLabel;

#[derive(Component, Clone, Copy)]
enum MainMenuButton { SinglePlayer, Settings, Quit }

#[derive(Component, Clone, Copy)]
enum SettingsButton { Back }

#[derive(Component)] struct PauseMenuRoot;
#[derive(Component, Clone, Copy)] enum PauseButton { Resume, MainMenu }

#[derive(Component, Clone, Copy)]
enum CreateButton { Normal, Flat, Create, Back, RandomSeed }

fn random_seed() -> u64 {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap();
    t.as_secs() ^ (t.subsec_nanos() as u64).wrapping_mul(0x9E3779B97F4A7C15)
}

fn despawn_all<T: Component>(mut commands: Commands, q: Query<Entity, With<T>>) {
    for e in q.iter() {
        commands.entity(e).despawn_recursive();
    }
}

// --- Новые хелперы под Bevy 0.15 ---

/// Корневой узел на весь экран
fn fullscreen_node() -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(12.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.08, 0.08, 0.12)),
    )
}

/// Кнопка (только корневой узел, без текста)
fn button_node() -> impl Bundle {
    (
        Button,
        Node {
            width: Val::Px(260.0),
            height: Val::Px(48.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.25, 0.25, 0.30)),
    )
}

/// Текстовый дочерний элемент кнопки
fn button_text(text: &str) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font_size: 22.0,
            ..default()
        },
        TextColor(Color::WHITE),
    )
}

/// Одиночный текстовый узел
fn label(text: &str) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
    )
}

// --- Основное меню ---

fn spawn_main_menu(mut commands: Commands) {
    commands
        .spawn((fullscreen_node(), MainMenuRoot))
        .with_children(|p| {
            p.spawn(label("MC-Rust"));

            p.spawn(button_node())
                .insert(MainMenuButton::SinglePlayer)
                .with_children(|b| { b.spawn(button_text("Single Player")); });

            p.spawn(button_node())
                .insert(MainMenuButton::Settings)
                .with_children(|b| { b.spawn(button_text("Settings")); });

            p.spawn(button_node())
                .insert(MainMenuButton::Quit)
                .with_children(|b| { b.spawn(button_text("Quit")); });
        });
}

fn main_menu_buttons(
    q: Query<(&Interaction, &MainMenuButton), Changed<Interaction>>,
    mut next: ResMut<NextState<AppState>>,
    mut exit: EventWriter<bevy::app::AppExit>,
) {
    for (i, b) in q.iter() {
        if *i != Interaction::Pressed { continue; }
        match b {
            MainMenuButton::SinglePlayer => next.set(AppState::WorldCreate),
            MainMenuButton::Settings => next.set(AppState::Settings),
            MainMenuButton::Quit => { exit.send(bevy::app::AppExit::Success); }
        }
    }
}

// --- Настройки ---

fn spawn_settings(mut commands: Commands) {
    commands
        .spawn((fullscreen_node(), SettingsRoot))
        .with_children(|p| {
            p.spawn(label("Settings"));
            p.spawn(label("(nothing here yet)"));

            p.spawn(button_node())
                .insert(SettingsButton::Back)
                .with_children(|b| { b.spawn(button_text("Back")); });
        });
}

fn settings_buttons(
    q: Query<(&Interaction, &SettingsButton), Changed<Interaction>>,
    mut next: ResMut<NextState<AppState>>,
) {
    for (i, _) in q.iter() {
        if *i == Interaction::Pressed {
            next.set(AppState::MainMenu);
        }
    }
}

// --- Создание мира ---

fn spawn_world_create(mut commands: Commands, config: Res<WorldConfig>) {
    let seed_text = if config.seed_input.is_empty() {
        "Seed: (random)".to_string()
    } else {
        format!("Seed: {}", config.seed_input)
    };

    commands
        .spawn((fullscreen_node(), WorldCreateRoot))
        .with_children(|p| {
            p.spawn(label("Create World"));

            p.spawn((
                Text::new(seed_text),
                TextFont { font_size: 22.0, ..default() },
                TextColor(Color::WHITE),
                SeedLabel,
            ));

            p.spawn(button_node())
                .insert(CreateButton::RandomSeed)
                .with_children(|b| { b.spawn(button_text("Random seed")); });

            p.spawn((
                Text::new(format!("World type: {:?}", config.world_type)),
                TextFont { font_size: 22.0, ..default() },
                TextColor(Color::WHITE),
                WorldTypeLabel,
            ));

            p.spawn(button_node())
                .insert(CreateButton::Normal)
                .with_children(|b| { b.spawn(button_text("Normal")); });

            p.spawn(button_node())
                .insert(CreateButton::Flat)
                .with_children(|b| { b.spawn(button_text("Flat")); });

            p.spawn(button_node())
                .insert(CreateButton::Create)
                .with_children(|b| { b.spawn(button_text("Create")); });

            p.spawn(button_node())
                .insert(CreateButton::Back)
                .with_children(|b| { b.spawn(button_text("Back")); });
        });
}

fn world_create_buttons(
    q: Query<(&Interaction, &CreateButton), Changed<Interaction>>,
    mut next: ResMut<NextState<AppState>>,
    mut config: ResMut<WorldConfig>,
    mut seed_label: Query<&mut Text, (With<SeedLabel>, Without<WorldTypeLabel>)>,
    mut type_label: Query<&mut Text, (With<WorldTypeLabel>, Without<SeedLabel>)>,
) {
    for (i, b) in q.iter() {
        if *i != Interaction::Pressed { continue; }
        match b {
            CreateButton::Normal => {
                config.world_type = WorldType::Normal;
                if let Ok(mut t) = type_label.get_single_mut() {
                    *t = Text::new(format!("World type: {:?}", config.world_type));
                }
            }
            CreateButton::Flat => {
                config.world_type = WorldType::Flat;
                if let Ok(mut t) = type_label.get_single_mut() {
                    *t = Text::new(format!("World type: {:?}", config.world_type));
                }
            }
            CreateButton::RandomSeed => {
                config.seed = random_seed();
                config.seed_input = config.seed.to_string();
                if let Ok(mut t) = seed_label.get_single_mut() {
                    *t = Text::new(format!("Seed: {}", config.seed_input));
                }
            }
            CreateButton::Create => {
                if config.seed_input.is_empty() {
                    config.seed = random_seed();
                } else {
                    config.seed = config.seed_input.parse().unwrap_or_else(|_| random_seed());
                }
                next.set(AppState::InGame);
            }
            CreateButton::Back => next.set(AppState::MainMenu),
        }
    }
}

fn seed_input(
    mut events: EventReader<KeyboardInput>,
    mut config: ResMut<WorldConfig>,
    mut seed_label: Query<&mut Text, (With<SeedLabel>, Without<WorldTypeLabel>)>,
    mut next: ResMut<NextState<AppState>>,
) {
    let mut changed = false;

    for ev in events.read() {
        if !ev.state.is_pressed() { continue; }
        match ev.key_code {
            KeyCode::Backspace => { config.seed_input.pop(); changed = true; }
            KeyCode::Escape => { config.seed_input.clear(); changed = true; }
            KeyCode::Enter => {
                if config.seed_input.is_empty() {
                    config.seed = random_seed();
                } else {
                    config.seed = config.seed_input.parse().unwrap_or_else(|_| random_seed());
                }
                next.set(AppState::InGame);
                return;
            }
            KeyCode::Digit0 | KeyCode::Numpad0 => { config.seed_input.push('0'); changed = true; }
            KeyCode::Digit1 | KeyCode::Numpad1 => { config.seed_input.push('1'); changed = true; }
            KeyCode::Digit2 | KeyCode::Numpad2 => { config.seed_input.push('2'); changed = true; }
            KeyCode::Digit3 | KeyCode::Numpad3 => { config.seed_input.push('3'); changed = true; }
            KeyCode::Digit4 | KeyCode::Numpad4 => { config.seed_input.push('4'); changed = true; }
            KeyCode::Digit5 | KeyCode::Numpad5 => { config.seed_input.push('5'); changed = true; }
            KeyCode::Digit6 | KeyCode::Numpad6 => { config.seed_input.push('6'); changed = true; }
            KeyCode::Digit7 | KeyCode::Numpad7 => { config.seed_input.push('7'); changed = true; }
            KeyCode::Digit8 | KeyCode::Numpad8 => { config.seed_input.push('8'); changed = true; }
            KeyCode::Digit9 | KeyCode::Numpad9 => { config.seed_input.push('9'); changed = true; }
            _ => {}
        }
    }
    if changed {
        let text = if config.seed_input.is_empty() {
            "Seed: (random)".to_string()
        } else {
            format!("Seed: {}", config.seed_input)
        };
        if let Ok(mut t) = seed_label.get_single_mut() {
            *t = Text::new(text);
        }
    }
}

fn spawn_pause_menu(mut commands: Commands) {
    commands
        .spawn((fullscreen_node(), PauseMenuRoot))
        .with_children(|p| {
            p.spawn(label("Paused"));

            p.spawn(button_node())
                .insert(PauseButton::Resume)
                .with_children(|b| { b.spawn(button_text("Resume")); });

            p.spawn(button_node())
                .insert(PauseButton::MainMenu)
                .with_children(|b| { b.spawn(button_text("Main Menu")); });
        });
}

fn pause_buttons(
    q: Query<(&Interaction, &PauseButton), Changed<Interaction>>,
    mut next: ResMut<NextState<AppState>>,
    mut commands: Commands,
    chunks: Option<Res<LoadedChunks>>,
    mut active: ResMut<ActiveWorld>,
    players: Query<Entity, With<crate::player::Player>>,
    lights: Query<Entity, With<DirectionalLight>>,
) {
    for (i, b) in q.iter() {
        if *i != Interaction::Pressed { continue; }
        match b {
            PauseButton::Resume => next.set(AppState::InGame),
            PauseButton::MainMenu => {
                // Полная очистка мира перед возвратом в главное меню
                crate::world::cleanup_world(
                    &mut commands,
                    chunks.as_deref(),
                    Some(&mut active),
                    &players,
                    &lights,
                );
                next.set(AppState::MainMenu);
            }
        }
    }
}