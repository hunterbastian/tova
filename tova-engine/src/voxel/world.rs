use std::collections::HashMap;

use glam::{IVec3, Vec3};
use web_time::{SystemTime, UNIX_EPOCH};

use super::block::Block;
use super::chunk::{
    Chunk, CHUNK_SIZE, SEA_LEVEL, SPAWN_SURFACE_Y, SPAWN_X, SPAWN_Z, WORLD_HEIGHT,
};

pub const DEFAULT_WORLD_RADIUS: i32 = 6;

const SPAWN_FLAT_RADIUS: f32 = 8.0;
const SPAWN_BLEND_RADIUS: f32 = 24.0;
const SPAWN_CLEAR_RADIUS: i32 = 11;
const FOREST_RADIUS: i32 = 18;
const CASTLE_HALF_WIDTH: i32 = 12;
const CASTLE_HALF_DEPTH: i32 = 10;

#[derive(Debug, Clone)]
pub struct WorldEdit {
    pub dirty_chunks: Vec<(i32, i32)>,
}

#[derive(Clone)]
pub struct VoxelWorld {
    chunks: HashMap<(i32, i32), Chunk>,
}

impl VoxelWorld {
    pub fn generate(radius: i32) -> Self {
        let seed = procedural_seed();
        Self::generate_with_seed(radius, seed)
    }

    pub fn generate_with_seed(radius: i32, seed: u64) -> Self {
        log::info!("Generating procedural world with landmarks, seed={seed:#x}");
        let mut chunks = HashMap::new();

        for cz in -radius..radius {
            for cx in -radius..radius {
                let mut chunk = Chunk::new(cx, cz);
                fill_chunk(&mut chunk, seed);
                chunks.insert((cx, cz), chunk);
            }
        }

        let mut world = Self { chunks };
        world.ensure_spawn_clearing();
        world.build_forest(seed);
        world.build_castle(seed);
        world
    }

    pub fn chunks(&self) -> &HashMap<(i32, i32), Chunk> {
        &self.chunks
    }

    pub fn sample_block(&self, block_pos: IVec3) -> Block {
        self.sample_xyz(block_pos.x, block_pos.y, block_pos.z)
    }

    pub fn sample_xyz(&self, wx: i32, wy: i32, wz: i32) -> Block {
        if wy < 0 {
            return Block::Stone;
        }
        if wy >= WORLD_HEIGHT as i32 {
            return Block::Air;
        }

        let (cx, cz, lx, lz) = block_address(wx, wz);
        self.chunks
            .get(&(cx, cz))
            .map(|chunk| chunk.get(lx, wy as usize, lz))
            .unwrap_or(Block::Air)
    }

    pub fn set_block(&mut self, block_pos: IVec3, block: Block) -> Option<WorldEdit> {
        if block_pos.y < 0 || block_pos.y >= WORLD_HEIGHT as i32 {
            return None;
        }

        let (cx, cz, lx, lz) = block_address(block_pos.x, block_pos.z);
        let chunk = self.chunks.get_mut(&(cx, cz))?;
        if chunk.get(lx, block_pos.y as usize, lz) == block {
            return None;
        }

        chunk.set(lx, block_pos.y as usize, lz, block);
        Some(WorldEdit {
            dirty_chunks: dirty_chunks_for_block(block_pos),
        })
    }

    pub fn block_coords(position: Vec3) -> IVec3 {
        position.floor().as_ivec3()
    }

    fn ensure_spawn_clearing(&mut self) {
        for wz in -SPAWN_CLEAR_RADIUS..=SPAWN_CLEAR_RADIUS {
            for wx in -SPAWN_CLEAR_RADIUS..=SPAWN_CLEAR_RADIUS {
                if spawn_distance(wx, wz) > SPAWN_CLEAR_RADIUS as f32 {
                    continue;
                }
                self.flatten_column(SPAWN_X + wx, SPAWN_Z + wz, SPAWN_SURFACE_Y, Block::Grass);
                self.clear_column(SPAWN_X + wx, SPAWN_Z + wz, SPAWN_SURFACE_Y + 1);
            }
        }
    }

    fn build_forest(&mut self, seed: u64) {
        let (center_x, center_z) = forest_center(seed);

        for wz in center_z - FOREST_RADIUS..=center_z + FOREST_RADIUS {
            for wx in center_x - FOREST_RADIUS..=center_x + FOREST_RADIUS {
                let dx = wx - center_x;
                let dz = wz - center_z;
                if dx * dx + dz * dz > FOREST_RADIUS * FOREST_RADIUS {
                    continue;
                }

                let terrain_y = surface_height(wx, wz, seed);
                if terrain_y <= SEA_LEVEL + 1 || terrain_y >= 58 {
                    continue;
                }

                self.ensure_surface(wx, wz, terrain_y, Block::Grass);
            }
        }

        for &(ox, oz) in &[
            (0, 0),
            (-5, -2),
            (4, -3),
            (-3, 4),
            (6, 5),
            (-7, 3),
            (2, 8),
            (8, -1),
        ] {
            let wx = center_x + ox;
            let wz = center_z + oz;
            let terrain_y = surface_height(wx, wz, seed);
            if terrain_y <= SEA_LEVEL + 1 || terrain_y >= 58 {
                continue;
            }
            self.ensure_surface(wx, wz, terrain_y, Block::Grass);
            self.place_tree(wx, terrain_y, wz, seed ^ ((wx as u64) << 32) ^ wz as u64);
        }

        for wz in center_z - FOREST_RADIUS..=center_z + FOREST_RADIUS {
            for wx in center_x - FOREST_RADIUS..=center_x + FOREST_RADIUS {
                let dx = wx - center_x;
                let dz = wz - center_z;
                if dx * dx + dz * dz > FOREST_RADIUS * FOREST_RADIUS {
                    continue;
                }

                let terrain_y = surface_height(wx, wz, seed);
                if terrain_y <= SEA_LEVEL + 1 || terrain_y >= 58 {
                    continue;
                }

                let noise = value_noise(wx as f32, wz as f32, 0.22, seed ^ 0x1db4_12c9_f216_5a5d);
                let grove = value_noise(wx as f32, wz as f32, 0.08, seed ^ 0x7f4a_7c15_9e37_79b9);
                let spaced_grid = ((wx + wz).rem_euclid(4) == 0) || ((wx - wz).rem_euclid(5) == 0);

                if noise > 0.22 && grove > -0.18 && spaced_grid {
                    self.place_tree(wx, terrain_y, wz, seed ^ ((wx as u64) << 32) ^ wz as u64);
                }
            }
        }
    }

    fn build_castle(&mut self, seed: u64) {
        let (center_x, center_z) = castle_center(seed);
        let platform_y = surface_height(center_x, center_z, seed).max(SPAWN_SURFACE_Y + 4);
        let plateau_half_width = CASTLE_HALF_WIDTH + 4;
        let plateau_half_depth = CASTLE_HALF_DEPTH + 4;

        for wz in center_z - plateau_half_depth..=center_z + plateau_half_depth {
            for wx in center_x - plateau_half_width..=center_x + plateau_half_width {
                let dx = (wx - center_x).abs();
                let dz = (wz - center_z).abs();
                let top = if dx <= CASTLE_HALF_WIDTH && dz <= CASTLE_HALF_DEPTH {
                    Block::Cobble
                } else {
                    Block::Grass
                };
                self.flatten_column(wx, wz, platform_y, top);
                self.clear_column(wx, wz, platform_y + 1);
            }
        }

        let wall_height = 6usize;
        let tower_height = wall_height + 3;

        for wz in center_z - CASTLE_HALF_DEPTH..=center_z + CASTLE_HALF_DEPTH {
            for wx in center_x - CASTLE_HALF_WIDTH..=center_x + CASTLE_HALF_WIDTH {
                let dx = (wx - center_x).abs();
                let dz = (wz - center_z).abs();
                let on_wall = dx == CASTLE_HALF_WIDTH || dz == CASTLE_HALF_DEPTH;
                let gate = wx == center_x + CASTLE_HALF_WIDTH
                    && (wz - center_z).abs() <= 1;
                if on_wall && !gate {
                    for y in platform_y + 1..=platform_y + wall_height {
                        let block = if y == platform_y + wall_height {
                            Block::Cobble
                        } else {
                            Block::Stone
                        };
                        self.set_block_raw(wx, y, wz, block);
                    }
                }
            }
        }

        for &(tower_x, tower_z) in &[
            (center_x - CASTLE_HALF_WIDTH, center_z - CASTLE_HALF_DEPTH),
            (center_x - CASTLE_HALF_WIDTH, center_z + CASTLE_HALF_DEPTH),
            (center_x + CASTLE_HALF_WIDTH, center_z - CASTLE_HALF_DEPTH),
            (center_x + CASTLE_HALF_WIDTH, center_z + CASTLE_HALF_DEPTH),
        ] {
            self.build_tower(tower_x, platform_y, tower_z, tower_height);
        }

        for wz in center_z - 3..=center_z + 3 {
            for wx in center_x - 3..=center_x + 3 {
                for y in platform_y + 1..=platform_y + wall_height + 5 {
                    let shell = (wx - center_x).abs() == 3 || (wz - center_z).abs() == 3;
                    if shell {
                        let block = if y == platform_y + wall_height + 5 {
                            Block::Cobble
                        } else {
                            Block::Stone
                        };
                        self.set_block_raw(wx, y, wz, block);
                    } else {
                        self.set_block_raw(wx, y, wz, Block::Air);
                    }
                }
            }
        }
    }

    fn build_tower(&mut self, tower_x: i32, platform_y: usize, tower_z: i32, tower_height: usize) {
        for wz in tower_z - 1..=tower_z + 1 {
            for wx in tower_x - 1..=tower_x + 1 {
                for y in platform_y + 1..=platform_y + tower_height {
                    let block = if y == platform_y + tower_height {
                        Block::Cobble
                    } else {
                        Block::Stone
                    };
                    self.set_block_raw(wx, y, wz, block);
                }
            }
        }
    }

    fn place_tree(&mut self, wx: i32, terrain_y: usize, wz: i32, seed: u64) {
        let height = 4 + (seed % 3) as usize;
        for y in terrain_y + 1..=terrain_y + height {
            self.set_block_if_open(wx, y, wz, Block::Wood);
        }

        let canopy_center = terrain_y + height;
        for y in canopy_center - 1..=canopy_center + 2 {
            let layer = y as i32 - canopy_center as i32;
            let radius: i32 = match layer {
                -1 => 2,
                0 => 2,
                1 => 2,
                _ => 1,
            };
            for dz in -radius..=radius {
                for dx in -radius..=radius {
                    if dx.abs() == radius && dz.abs() == radius && y != canopy_center + 2 {
                        continue;
                    }
                    let leaf_x = wx + dx;
                    let leaf_z = wz + dz;
                    let leaf_y = y;
                    if leaf_x == wx && leaf_z == wz && leaf_y <= terrain_y + height {
                        continue;
                    }
                    self.set_block_if_open(leaf_x, leaf_y, leaf_z, Block::Leaf);
                }
            }
        }
    }

    fn flatten_column(&mut self, wx: i32, wz: i32, target_y: usize, top_block: Block) {
        for y in 0..=target_y {
            let block = if y == target_y {
                top_block
            } else if y + 3 >= target_y {
                if top_block == Block::Sand {
                    Block::Sand
                } else {
                    Block::Dirt
                }
            } else {
                Block::Stone
            };
            self.set_block_raw(wx, y, wz, block);
        }
    }

    fn ensure_surface(&mut self, wx: i32, wz: i32, surface_y: usize, block: Block) {
        self.set_block_raw(wx, surface_y, wz, block);
        for y in surface_y + 1..=surface_y + 2 {
            self.set_block_raw(wx, y, wz, Block::Air);
        }
    }

    fn clear_column(&mut self, wx: i32, wz: i32, from_y: usize) {
        for y in from_y..WORLD_HEIGHT {
            self.set_block_raw(wx, y, wz, Block::Air);
        }
    }

    fn set_block_if_open(&mut self, wx: i32, wy: usize, wz: i32, block: Block) {
        let current = self.sample_xyz(wx, wy as i32, wz);
        if current.is_replaceable() {
            self.set_block_raw(wx, wy, wz, block);
        }
    }

    fn set_block_raw(&mut self, wx: i32, wy: usize, wz: i32, block: Block) {
        if wy >= WORLD_HEIGHT {
            return;
        }
        let (cx, cz, lx, lz) = block_address(wx, wz);
        let Some(chunk) = self.chunks.get_mut(&(cx, cz)) else {
            return;
        };
        chunk.set(lx, wy, lz, block);
    }
}

fn fill_chunk(chunk: &mut Chunk, seed: u64) {
    let base_x = chunk.cx * CHUNK_SIZE as i32;
    let base_z = chunk.cz * CHUNK_SIZE as i32;

    for lz in 0..CHUNK_SIZE {
        for lx in 0..CHUNK_SIZE {
            let wx = base_x + lx as i32;
            let wz = base_z + lz as i32;
            let surface = surface_height(wx, wz, seed);
            let top = top_block(surface, wx, wz, seed);
            let filler = filler_block(top);

            for y in 0..=surface {
                let block = if y == surface {
                    top
                } else if y + 3 >= surface {
                    filler
                } else {
                    Block::Stone
                };
                chunk.set(lx, y, lz, block);
            }

            for y in surface + 1..WORLD_HEIGHT {
                let block = if y <= SEA_LEVEL {
                    Block::Water
                } else {
                    Block::Air
                };
                chunk.set(lx, y, lz, block);
            }
        }
    }
}

fn surface_height(wx: i32, wz: i32, seed: u64) -> usize {
    let wxf = wx as f32;
    let wzf = wz as f32;
    let continental = value_noise(wxf, wzf, 0.0065, seed ^ 0x9e37_79b9_7f4a_7c15);
    let hills = value_noise(wxf, wzf, 0.0180, seed ^ 0xc2b2_ae3d_27d4_eb4f);
    let detail = value_noise(wxf, wzf, 0.0520, seed ^ 0x94d0_49bb_1331_11eb);
    let ridge = 1.0 - value_noise(wxf, wzf, 0.0130, seed ^ 0xff51_afd7_ed55_8ccd).abs();
    let basin = value_noise(wxf, wzf, 0.0037, seed ^ 0xc4ce_b9fe_1a85_ec53);

    let raw_height =
        26.0 + continental * 14.0 + hills * 8.0 + detail * 2.4 + ridge * 7.5 + basin * 5.0;
    let distance = spawn_distance(wx, wz);
    let height = if distance <= SPAWN_FLAT_RADIUS {
        SPAWN_SURFACE_Y as f32
    } else if distance >= SPAWN_BLEND_RADIUS {
        raw_height
    } else {
        let t = smoothstep01(
            (distance - SPAWN_FLAT_RADIUS) / (SPAWN_BLEND_RADIUS - SPAWN_FLAT_RADIUS),
        );
        lerp(SPAWN_SURFACE_Y as f32, raw_height, t)
    };

    height.round().clamp(8.0, (WORLD_HEIGHT - 2) as f32) as usize
}

fn top_block(surface: usize, wx: i32, wz: i32, seed: u64) -> Block {
    if spawn_distance(wx, wz) <= SPAWN_BLEND_RADIUS * 0.55 {
        return Block::Grass;
    }

    if surface <= SEA_LEVEL + 1 {
        return Block::Sand;
    }

    let moisture = value_noise(wx as f32, wz as f32, 0.021, seed ^ 0x27d4_eb2f_1656_67c5);
    if surface >= 62 || (surface >= 52 && moisture < -0.14) {
        Block::Stone
    } else {
        Block::Grass
    }
}

fn filler_block(top: Block) -> Block {
    match top {
        Block::Sand => Block::Sand,
        Block::Stone => Block::Stone,
        _ => Block::Dirt,
    }
}

fn forest_center(seed: u64) -> (i32, i32) {
    let x = 42 + seed_offset(seed ^ 0x5bf0_3635, 10);
    let z = 38 + seed_offset(seed ^ 0xa54f_f53a, 10);
    (x, z)
}

fn castle_center(seed: u64) -> (i32, i32) {
    let x = -58 + seed_offset(seed ^ 0x510e_527f, 8);
    let z = -34 + seed_offset(seed ^ 0x9b05_688c, 8);
    (x, z)
}

fn seed_offset(seed: u64, max_abs: i32) -> i32 {
    let span = (max_abs * 2 + 1) as u64;
    (seed % span) as i32 - max_abs
}

fn spawn_distance(wx: i32, wz: i32) -> f32 {
    let dx = (wx - SPAWN_X) as f32;
    let dz = (wz - SPAWN_Z) as f32;
    (dx * dx + dz * dz).sqrt()
}

fn block_address(wx: i32, wz: i32) -> (i32, i32, usize, usize) {
    let chunk_size = CHUNK_SIZE as i32;
    let cx = wx.div_euclid(chunk_size);
    let cz = wz.div_euclid(chunk_size);
    let lx = wx.rem_euclid(chunk_size) as usize;
    let lz = wz.rem_euclid(chunk_size) as usize;
    (cx, cz, lx, lz)
}

fn dirty_chunks_for_block(block_pos: IVec3) -> Vec<(i32, i32)> {
    let chunk_size = CHUNK_SIZE as i32;
    let cx = block_pos.x.div_euclid(chunk_size);
    let cz = block_pos.z.div_euclid(chunk_size);
    let lx = block_pos.x.rem_euclid(chunk_size);
    let lz = block_pos.z.rem_euclid(chunk_size);

    let mut dirty = vec![(cx, cz)];
    if lx == 0 {
        dirty.push((cx - 1, cz));
    }
    if lx == chunk_size - 1 {
        dirty.push((cx + 1, cz));
    }
    if lz == 0 {
        dirty.push((cx, cz - 1));
    }
    if lz == chunk_size - 1 {
        dirty.push((cx, cz + 1));
    }
    dirty.sort_unstable();
    dirty.dedup();
    dirty
}

fn procedural_seed() -> u64 {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let nanos = elapsed.as_nanos() as u64;
    nanos ^ nanos.rotate_left(17) ^ 0xa076_1d64_78bd_642f
}

fn value_noise(x: f32, z: f32, frequency: f32, seed: u64) -> f32 {
    let scaled_x = x * frequency;
    let scaled_z = z * frequency;
    let x0 = scaled_x.floor() as i32;
    let z0 = scaled_z.floor() as i32;
    let tx = smoothstep01(scaled_x.fract());
    let tz = smoothstep01(scaled_z.fract());

    let n00 = lattice_value(x0, z0, seed);
    let n10 = lattice_value(x0 + 1, z0, seed);
    let n01 = lattice_value(x0, z0 + 1, seed);
    let n11 = lattice_value(x0 + 1, z0 + 1, seed);

    let nx0 = lerp(n00, n10, tx);
    let nx1 = lerp(n01, n11, tx);
    lerp(nx0, nx1, tz)
}

fn lattice_value(x: i32, z: i32, seed: u64) -> f32 {
    let mixed = seed
        ^ ((x as u64).wrapping_mul(0x9e37_79b1_85eb_ca87))
        ^ ((z as u64)
            .wrapping_mul(0xc2b2_ae3d_27d4_eb4f)
            .rotate_left(32));
    let mut hash = mixed;
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    hash ^= hash >> 33;

    (hash as f64 / u64::MAX as f64) as f32 * 2.0 - 1.0
}

fn smoothstep01(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use glam::{IVec3, Vec3};

    use super::{
        castle_center, forest_center, surface_height, Block, VoxelWorld, CASTLE_HALF_DEPTH,
        CASTLE_HALF_WIDTH,
    };
    use crate::voxel::chunk::{SPAWN_SURFACE_Y, SPAWN_X, SPAWN_Z};

    const TEST_SEED: u64 = 0xfeed_cafe_d15c_a11e;

    #[test]
    fn block_coords_floor_world_positions() {
        let coords = VoxelWorld::block_coords(Vec3::new(2.9, 48.1, -3.2));
        assert_eq!(coords, IVec3::new(2, 48, -4));
    }

    #[test]
    fn setting_out_of_bounds_block_is_rejected() {
        let mut world = VoxelWorld::generate_with_seed(1, TEST_SEED);
        assert!(world.set_block(IVec3::new(0, -1, 0), Block::Dirt).is_none());
    }

    #[test]
    fn spawn_plateau_exists_at_origin() {
        let world = VoxelWorld::generate_with_seed(1, TEST_SEED);
        assert_eq!(world.sample_xyz(SPAWN_X, SPAWN_SURFACE_Y as i32, SPAWN_Z), Block::Grass);
        assert_eq!(
            world.sample_xyz(SPAWN_X, SPAWN_SURFACE_Y as i32 + 1, SPAWN_Z),
            Block::Air
        );
    }

    #[test]
    fn spawn_area_is_flat() {
        let center = surface_height(0, 0, TEST_SEED);
        assert_eq!(center, surface_height(4, 0, TEST_SEED));
        assert_eq!(center, surface_height(-3, 5, TEST_SEED));
    }

    #[test]
    fn terrain_varies_away_from_spawn() {
        let samples = [
            surface_height(48, -32, TEST_SEED),
            surface_height(64, 64, TEST_SEED),
            surface_height(-80, 24, TEST_SEED),
            surface_height(112, -48, TEST_SEED),
        ];
        let min = samples.into_iter().min().unwrap_or_default();
        let max = samples.into_iter().max().unwrap_or_default();
        assert!(max.saturating_sub(min) >= 8);
    }

    #[test]
    fn forest_landmark_contains_trees() {
        let world = VoxelWorld::generate_with_seed(6, TEST_SEED);
        let (center_x, center_z) = forest_center(TEST_SEED);

        let mut saw_wood = false;
        let mut saw_leaf = false;
        for z in center_z - 8..=center_z + 8 {
            for x in center_x - 8..=center_x + 8 {
                for y in SPAWN_SURFACE_Y as i32..(SPAWN_SURFACE_Y as i32 + 20) {
                    let block = world.sample_xyz(x, y, z);
                    saw_wood |= block == Block::Wood;
                    saw_leaf |= block == Block::Leaf;
                }
            }
        }

        assert!(saw_wood && saw_leaf);
    }

    #[test]
    fn castle_landmark_contains_stonework() {
        let world = VoxelWorld::generate_with_seed(6, TEST_SEED);
        let (center_x, center_z) = castle_center(TEST_SEED);
        let platform_y = surface_height(center_x, center_z, TEST_SEED).max(SPAWN_SURFACE_Y + 4);

        assert_eq!(world.sample_xyz(center_x, (platform_y + 3) as i32, center_z), Block::Air);
        assert_eq!(
            world.sample_xyz(
                center_x - CASTLE_HALF_WIDTH,
                (platform_y + 3) as i32,
                center_z - CASTLE_HALF_DEPTH,
            ),
            Block::Stone
        );
        assert_eq!(
            world.sample_xyz(
                center_x + CASTLE_HALF_WIDTH,
                (platform_y + 3) as i32,
                center_z + CASTLE_HALF_DEPTH,
            ),
            Block::Stone
        );
    }

    #[test]
    fn boundary_edits_mark_neighbor_chunks_dirty() {
        let mut world = VoxelWorld::generate_with_seed(2, TEST_SEED);
        let y = surface_height(16, 0, TEST_SEED) as i32;
        let edit = world
            .set_block(IVec3::new(16, y, 0), Block::Air)
            .expect("expected edit");
        assert!(edit.dirty_chunks.contains(&(1, 0)));
        assert!(edit.dirty_chunks.contains(&(0, 0)));
    }
}
