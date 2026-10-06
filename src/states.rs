use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    MainMenu,
    Settings,
    WorldCreate,
    InGame,
    Paused,
}

/// Параметры, выбранные в меню создания мира.
#[derive(Resource)]
pub struct WorldConfig {
    pub seed: u64,
    pub world_type: crate::worldgen::WorldType,
    pub seed_input: String,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            world_type: crate::worldgen::WorldType::Normal,
            seed_input: String::new(),
        }
    }
}

/// Сохраняет созданный мир, чтобы при выходе из игры можно было его очистить.
#[derive(Resource, Default)]
pub struct ActiveWorld {
    pub gen: Option<crate::worldgen::WorldGen>,
}