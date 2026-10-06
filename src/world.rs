use crate::blocks::TextureSet;
use crate::chunk::{
    build_chunk_meshes, generate_chunk, world_lookup_block, ChunkData, ChunkEntity,
};
use crate::states::{ActiveWorld, AppState, WorldConfig};
use crate::worldgen::WorldGen;
use bevy::prelude::*;
use std::collections::HashMap;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
        OnEnter(AppState::InGame),
        (cleanup_before_setup, setup_world).chain(),
        )
            .add_systems(OnExit(AppState::InGame), teardown_world);
    }
}

/// Ресурс, хранящий сгенерированные чанки и их корневые сущности.
#[derive(Resource, Default)]
pub struct LoadedChunks {
    pub data: HashMap<IVec2, ChunkData>,
    pub entities: HashMap<IVec2, Entity>,
    pub all_trees: Vec<IVec3>,
}

const VIEW_RADIUS: i32 = 3; // радиус отображения в чанках

fn setup_world(
    mut commands: Commands,
    config: Res<WorldConfig>,
    mut active: ResMut<ActiveWorld>,
    texture_set: Res<TextureSet>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let seed = if config.seed_input.is_empty() {
        config.seed
    } else {
        config.seed_input.parse::<u64>().unwrap_or(config.seed)
    };

    let gen = WorldGen::new(seed, config.world_type);
    let gen_clone_for_lookup = WorldGen::new(seed, config.world_type);

    // Вычисляем высоту спавна ДО перемещения gen
    let spawn_height = gen.surface_height(0, 0) + 1; // ноги на поверхности

    // Сохраняем генератор в ресурс (перемещение — теперь безопасно)
    active.gen = Some(gen);

    // Свет
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.7, 0.5, 0.0)),
    ));
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 400.0,
    });

    // Генерируем чанки вокруг спавна
    let mut loaded = LoadedChunks::default();

    // Первый проход: сгенерировать данные и собрать все деревья
    let mut chunk_datas: HashMap<IVec2, ChunkData> = HashMap::new();
    for cx in -VIEW_RADIUS..=VIEW_RADIUS {
        for cz in -VIEW_RADIUS..=VIEW_RADIUS {
            let pos = IVec2::new(cx, cz);
            let data = generate_chunk(pos, &gen_clone_for_lookup);
            loaded.all_trees.extend(data.trees.iter().copied());
            chunk_datas.insert(pos, data);
        }
    }

    // Второй проход: построить меши
    for (pos, data) in &chunk_datas {
        let trees = loaded.all_trees.clone();
        let chunk_datas_ref = &chunk_datas;
        let gen_ref = &gen_clone_for_lookup;

        let lookup = move |x: i32, y: i32, z: i32| -> crate::blocks::BlockType {
            world_lookup_block(gen_ref, chunk_datas_ref, &trees, x, y, z)
        };

        let meshes_with_tex = build_chunk_meshes(data, &lookup);

        let mut root = commands.spawn((
            Transform::default(),
            Visibility::default(),
            ChunkEntity { pos: *pos },
        ));

        let root_id = root.id();

        root.with_children(|parent| {
            for (tex, mesh) in meshes_with_tex {
                let mat = texture_set.materials.get(&tex).cloned()
                    .unwrap_or_else(|| panic!("no material for {:?}", tex));
                parent.spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(mat),
                    Transform::from_translation(data.origin().as_vec3()),
                ));
            }
        });

        loaded.entities.insert(*pos, root_id);
    }

    loaded.data = chunk_datas;
    commands.insert_resource(loaded);

    // Игрок
    commands.spawn((
        crate::player::Player,
        crate::player::Velocity(Vec3::ZERO),
        crate::player::FlightState {
            is_flying: false,
            last_space_press: None,
            double_press_window: 0.3,
            flying_up: false,
            flying_down: false,
        },
        Camera3d::default(),
        Transform::from_xyz(0.5, spawn_height as f32 + crate::player::EYE_HEIGHT, 0.5),
    ));
}
fn teardown_world(
    mut commands: Commands,
    chunks: Option<Res<LoadedChunks>>,
    mut active: ResMut<ActiveWorld>,
    players: Query<Entity, With<crate::player::Player>>,
    lights: Query<Entity, With<DirectionalLight>>,
) {
    cleanup_world(
        &mut commands,
        chunks.as_deref(),
        Some(&mut active),
        &players,
        &lights,
    );
}

/// Полная очистка мира: чанки, игрок, свет, ресурсы.
pub fn cleanup_world(
    commands: &mut Commands,
    chunks: Option<&LoadedChunks>,
    active: Option<&mut ActiveWorld>,
    players: &Query<Entity, With<crate::player::Player>>,
    lights: &Query<Entity, With<DirectionalLight>>,
) {
    if let Some(chunks) = chunks {
        for (_, entity) in chunks.entities.iter() {
            commands.entity(*entity).despawn_recursive();
        }
    }
    for e in players.iter() {
        commands.entity(e).despawn_recursive();
    }
    for e in lights.iter() {
        commands.entity(e).despawn_recursive();
    }
    commands.remove_resource::<LoadedChunks>();
    if let Some(active) = active {
        active.gen = None;
    }
}

fn cleanup_before_setup(
    mut commands: Commands,
    chunks: Option<Res<LoadedChunks>>,
    mut active: ResMut<ActiveWorld>,
    players: Query<Entity, With<crate::player::Player>>,
    lights: Query<Entity, With<DirectionalLight>>,
) {
    cleanup_world(
        &mut commands,
        chunks.as_deref(),
        Some(&mut active),
        &players,
        &lights,
    );
}