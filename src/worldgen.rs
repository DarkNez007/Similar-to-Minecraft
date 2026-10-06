use crate::blocks::BlockType;
use noise::{NoiseFn, Perlin};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldType {
    Normal,
    Flat,
}

pub const WORLD_BASE_HEIGHT: i32 = 64;

pub struct WorldGen {
    perlin: Perlin,
    perlin_detail: Perlin,
    world_type: WorldType,
    seed: u64,
}

impl WorldGen {
    pub fn new(seed: u64, world_type: WorldType) -> Self {
        Self {
            perlin: Perlin::new(seed as u32),
            perlin_detail: Perlin::new((seed.wrapping_mul(6364136223846793005) as u32).wrapping_add(1)),
            world_type,
            seed,
        }
    }

    #[allow(dead_code)]
    pub fn seed(&self) -> u64 { self.seed }
    #[allow(dead_code)]
    pub fn world_type(&self) -> WorldType { self.world_type }

    /// Высота поверхности в данной колонке (x, z — мировые координаты).
    pub fn surface_height(&self, x: i32, z: i32) -> i32 {
        match self.world_type {
            WorldType::Flat => WORLD_BASE_HEIGHT,
            WorldType::Normal => {
                let base_scale = 0.012;
                let detail_scale = 0.06;
                let h1 = self.perlin.get([x as f64 * base_scale, z as f64 * base_scale]);
                let h2 = self.perlin_detail.get([x as f64 * detail_scale, z as f64 * detail_scale]);
                let combined = h1 * 0.75 + h2 * 0.25;
                WORLD_BASE_HEIGHT + (combined * 18.0) as i32
            }
        }
    }

    /// Тип блока в мировых координатах без учёта деревьев.
    pub fn terrain_block(&self, x: i32, y: i32, z: i32) -> BlockType {
        let h = self.surface_height(x, z);
        if y > h {
            BlockType::Air
        } else if y == h {
            BlockType::Grass
        } else if y >= h - 3 {
            BlockType::Stone
        } else {
            BlockType::Stone
        }
    }

    /// Детерминированный хеш по колонке. Возвращает true, если здесь должно
    /// вырасти дерево.
    pub fn tree_at(&self, x: i32, z: i32) -> bool {
        if self.world_type == WorldType::Flat {
            // На плоском мире деревьев нет
            return false;
        }
        let mut h = self.seed.wrapping_add(x as u64).wrapping_mul(0x9E3779B97F4A7C15);
        h ^= (z as u64).wrapping_mul(0xBF58476D1CE4E5B9);
        h = h.rotate_left(31);
        // Примерно 1 дерево на ~120 колонок
        (h % 120) == 0
    }
}