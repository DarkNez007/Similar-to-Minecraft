use crate::blocks::{BlockType, Face, TextureKind};
use crate::worldgen::WorldGen;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use std::collections::HashMap;

pub const CHUNK_SIZE: i32 = 16;
pub const CHUNK_HEIGHT: i32 = 128;

/// Данные чанка: 16×128×16 блоков + заготовки для деревьев.
pub struct ChunkData {
    pub pos: IVec2,
    pub blocks: Vec<BlockType>,
    pub trees: Vec<IVec3>, // мировые позиции корней деревьев в этом чанке
}

impl ChunkData {
    pub fn new(pos: IVec2) -> Self {
        Self {
            pos,
            blocks: vec![BlockType::Air; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_HEIGHT) as usize],
            trees: Vec::new(),
        }
    }

    fn idx(x: i32, y: i32, z: i32) -> usize {
        ((y * CHUNK_SIZE + z) * CHUNK_SIZE + x) as usize
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockType {
        if x < 0 || x >= CHUNK_SIZE || z < 0 || z >= CHUNK_SIZE || y < 0 || y >= CHUNK_HEIGHT {
            return BlockType::Air;
        }
        self.blocks[Self::idx(x, y, z)]
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, b: BlockType) {
        if x < 0 || x >= CHUNK_SIZE || z < 0 || z >= CHUNK_SIZE || y < 0 || y >= CHUNK_HEIGHT {
            return;
        }
        self.blocks[Self::idx(x, y, z)] = b;
    }

    pub fn origin(&self) -> IVec3 {
        IVec3::new(self.pos.x * CHUNK_SIZE, 0, self.pos.y * CHUNK_SIZE)
    }
}

/// Сгенерировать данные чанка: рельеф + деревья.
pub fn generate_chunk(pos: IVec2, gen: &WorldGen) -> ChunkData {
    let mut chunk = ChunkData::new(pos);
    let origin = chunk.origin();

    // 1. Рельеф
    for lx in 0..CHUNK_SIZE {
        for lz in 0..CHUNK_SIZE {
            let wx = origin.x + lx;
            let wz = origin.z + lz;
            for y in 0..CHUNK_HEIGHT {
                let b = gen.terrain_block(wx, y, wz);
                chunk.set(lx, y, lz, b);
            }
        }
    }

    // 2. Собираем позиции деревьев, чьи корни попадают в этот чанк
    for lx in 0..CHUNK_SIZE {
        for lz in 0..CHUNK_SIZE {
            let wx = origin.x + lx;
            let wz = origin.z + lz;
            if gen.tree_at(wx, wz) {
                let h = gen.surface_height(wx, wz);
                chunk.trees.push(IVec3::new(wx, h, wz));
            }
        }
    }

    chunk
}

/// Построить меши для чанка. Возвращает по одному мешу на каждый тип
/// использованной текстуры.
pub fn build_chunk_meshes(
    chunk: &ChunkData,
    world_lookup: &dyn Fn(i32, i32, i32) -> BlockType,
) -> Vec<(TextureKind, Mesh)> {
    // Группируем грани по текстуре
    let mut groups: HashMap<TextureKind, Vec<(Face, IVec3)>> = HashMap::new();
    let origin = chunk.origin();

    for lx in 0..CHUNK_SIZE {
        for ly in 0..CHUNK_HEIGHT {
            for lz in 0..CHUNK_SIZE {
                let block = chunk.get(lx, ly, lz);
                if block == BlockType::Air { continue; }

                let wx = origin.x + lx;
                let wy = ly;
                let wz = origin.z + lz;

                for face in Face::ALL {
                    let (dx, dy, dz) = face.offset();
                    let nx = wx + dx;
                    let ny = wy + dy;
                    let nz = wz + dz;

                    // Сосед: если это внутренняя координата — из чанка, иначе из внешнего мира
                    let neighbor = if nx >= origin.x && nx < origin.x + CHUNK_SIZE
                        && nz >= origin.z && nz < origin.z + CHUNK_SIZE
                        && ny >= 0 && ny < CHUNK_HEIGHT
                    {
                        chunk.get(nx - origin.x, ny, nz - origin.z)
                    } else {
                        world_lookup(nx, ny, nz)
                    };

                    if !face_visible(block, neighbor) { continue; }

                    if let Some(tex) = block.face_texture(face) {
                        groups.entry(tex).or_default().push((face, IVec3::new(lx, ly, lz)));
                    }
                }
            }
        }
    }

    groups.into_iter().map(|(tex, faces)| (tex, build_mesh(&faces))).collect()
}

fn face_visible(block: BlockType, neighbor: BlockType) -> bool {
    if neighbor == BlockType::Air { return true; }
    if neighbor == block { return false; }
    neighbor.is_transparent()
}

fn build_mesh(faces: &[(Face, IVec3)]) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(faces.len() * 4);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(faces.len() * 4);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(faces.len() * 4);
    let mut indices: Vec<u32> = Vec::with_capacity(faces.len() * 6);

    for (face, block_pos) in faces {
        let base = positions.len() as u32;
        let corners = face.corners();
        let normal = face.normal();
        let uvs_f = face.uvs();
        let bx = block_pos.x as f32;
        let by = block_pos.y as f32;
        let bz = block_pos.z as f32;

        for i in 0..4 {
            positions.push([corners[i][0] + bx, corners[i][1] + by, corners[i][2] + bz]);
            normals.push(normal);
            uvs.push(uvs_f[i]);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Компонент-маркер для корневого узла чанка.
#[derive(Component)]
pub struct ChunkEntity {
    #[allow(dead_code)]
    pub pos: IVec2,
}

/// Хелпер: получить блок в мире по мировым координатам через
/// функцию-замыкание, включая деревья.
pub fn world_lookup_block(
    gen: &WorldGen,
    chunks: &std::collections::HashMap<IVec2, ChunkData>,
    trees_global: &[IVec3],
    x: i32, y: i32, z: i32,
) -> BlockType {
    // Сначала проверяем деревья
    for root in trees_global {
        if let Some(b) = tree_block_at(*root, x, y, z) {
            return b;
        }
    }

    // Потом terrain из уже сгенерированного чанка, если есть
    let cx = x.div_euclid(CHUNK_SIZE);
    let cz = z.div_euclid(CHUNK_SIZE);
    if let Some(c) = chunks.get(&IVec2::new(cx, cz)) {
        let lx = x.rem_euclid(CHUNK_SIZE);
        let lz = z.rem_euclid(CHUNK_SIZE);
        let b = c.get(lx, y, lz);
        if b != BlockType::Air { return b; }
    }

    // Иначе — чистая генерация
    gen.terrain_block(x, y, z)
}

/// Проверяет, попадает ли мировая точка (x,y,z) внутрь дерева с корнем root.
/// Возвращает тип блока, если да.
pub fn tree_block_at(root: IVec3, x: i32, y: i32, z: i32) -> Option<BlockType> {
    let rx = root.x;
    let ry = root.y;
    let rz = root.z;
    let lx = x - rx;
    let ly = y - ry;
    let lz = z - rz;

    // Ствол: 5 блоков вверх
    if lx == 0 && lz == 0 && ly >= 0 && ly <= 4 {
        return Some(BlockType::OakLog);
    }

    // Листва: два слоя плюс крест сверху
    // Слой 1 и 2: y = 3..4, радиус 2
    if ly >= 3 && ly <= 4 {
        if lx.abs() <= 2 && lz.abs() <= 2 {
            // Углы убираем
            if lx.abs() == 2 && lz.abs() == 2 && ly == 3 { return None; }
            return Some(BlockType::OakLeaves);
        }
    }
    // Слой 3: y = 5, радиус 1
    if ly == 5 && lx.abs() <= 1 && lz.abs() <= 1 {
        return Some(BlockType::OakLeaves);
    }
    // Верхушка: y = 6, центр
    if ly == 6 && lx == 0 && lz == 0 {
        return Some(BlockType::OakLeaves);
    }

    None
}