use bevy::prelude::*;
use std::collections::HashMap;

/// Идентификатор типа блока. Хранится в массиве чанка.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockType {
    Air,
    Grass,
    Stone,
    OakLog,
    OakLeaves,
}

impl BlockType {
    pub fn is_solid(&self) -> bool {
        !matches!(self, BlockType::Air | BlockType::OakLeaves)
    }

    pub fn is_transparent(&self) -> bool {
        matches!(self, BlockType::Air | BlockType::OakLeaves)
    }

    /// Какая текстура нужна для конкретной грани блока.
    pub fn face_texture(&self, face: Face) -> Option<TextureKind> {
    use self::Face::*;
        match self {
            BlockType::Air => None,
            BlockType::Grass => match face {
                Top => Some(TextureKind::GrassTop),
                Bottom => Some(TextureKind::Dirt),
                North | South | East | West => Some(TextureKind::GrassSide),
            },
            BlockType::Stone => Some(TextureKind::Stone),
            BlockType::OakLog => match face {
                Top | Bottom => Some(TextureKind::OakLogTop),
                _ => Some(TextureKind::OakLogSide),
            },
            BlockType::OakLeaves => Some(TextureKind::OakLeaves),
        }
    }
}

/// Шесть граней куба. Порядок важен для генерации вершин.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Top, Bottom, North, South, East, West,
}

impl Face {
    pub const ALL: [Face; 6] = [Face::Top, Face::Bottom, Face::North, Face::South, Face::East, Face::West];

    /// Смещение соседнего блока в мировых координатах.
    pub fn offset(&self) -> (i32, i32, i32) {
        match self {
            Face::Top => (0, 1, 0),
            Face::Bottom => (0, -1, 0),
            Face::North => (0, 0, -1),
            Face::South => (0, 0, 1),
            Face::East => (1, 0, 0),
            Face::West => (-1, 0, 0),
        }
    }

    /// 4 вершины грани в локальных координатах блока (x, y, z от 0 до 1),
    /// перечисленные против часовой стрелки, если смотреть снаружи.
    pub fn corners(&self) -> [[f32; 3]; 4] {
        match self {
            Face::Top    => [[0.0, 1.0, 0.0], [0.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 0.0]],
            Face::Bottom => [[0.0, 0.0, 1.0], [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0]],
            Face::North  => [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
            Face::South  => [[1.0, 0.0, 1.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0], [0.0, 0.0, 1.0]],
            Face::East   => [[1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0], [1.0, 0.0, 1.0]],
            Face::West   => [[0.0, 0.0, 1.0], [0.0, 1.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]],
        }
    }

    pub fn normal(&self) -> [f32; 3] {
        let (x, y, z) = self.offset();
        [x as f32, y as f32, z as f32]
    }

    /// UV-координаты для 4 вершин грани.
    /// Порядок соответствует порядку вершин в corners().
    pub fn uvs(&self) -> [[f32; 2]; 4] {
        match self {
            Face::Top    => [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
            Face::Bottom => [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
            Face::North  => [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
            Face::South  => [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
            Face::East   => [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
            Face::West   => [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
        }
    }
}

/// Типы текстур (одна текстура = один PNG).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureKind {
    GrassTop,
    GrassSide,
    Dirt,
    Stone,
    OakLogTop,
    OakLogSide,
    OakLeaves,
}

impl TextureKind {
    pub const ALL: [TextureKind; 7] = [
        TextureKind::GrassTop, TextureKind::GrassSide, TextureKind::Dirt,
        TextureKind::Stone, TextureKind::OakLogTop, TextureKind::OakLogSide,
        TextureKind::OakLeaves,
    ];

    pub fn path(&self) -> &'static str {
        match self {
            TextureKind::GrassTop => "textures/grass_top.png",
            TextureKind::GrassSide => "textures/grass_side.png",
            TextureKind::Dirt => "textures/dirt.png",
            TextureKind::Stone => "textures/stone.png",
            TextureKind::OakLogTop => "textures/oak_log_top.png",
            TextureKind::OakLogSide => "textures/oak_log_side.png",
            TextureKind::OakLeaves => "textures/oak_leaves.png",
        }
    }
}

/// Ресурс со всеми материалами, сгруппированными по типу текстуры.
#[derive(Resource)]
pub struct TextureSet {
    pub materials: HashMap<TextureKind, Handle<StandardMaterial>>,
}

impl FromWorld for TextureSet {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>().clone();
        let mut materials = HashMap::new();

        for kind in TextureKind::ALL {
            let texture: Handle<Image> = asset_server.load(kind.path());
            let material = world
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial {
                    base_color_texture: Some(texture),
                    perceptual_roughness: 1.0,
                    reflectance: 0.0,
                    // Важно для листвы/прозрачности:
                    alpha_mode: if matches!(kind, TextureKind::OakLeaves) {
                        AlphaMode::Mask(0.5)
                    } else {
                        AlphaMode::Opaque
                    },
                    // Отключаем отсечение задних граней для листвы? Нет - оставим.
                    cull_mode: Some(bevy::render::render_resource::Face::Back),
                    ..default()
                });
            materials.insert(kind, material);
        }

        TextureSet { materials }
    }
}