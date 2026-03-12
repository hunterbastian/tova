use std::collections::HashMap;
use wgpu::util::DeviceExt;

use crate::renderer::vertex::Vertex;
use crate::voxel::block::{Block, BLOCK_COLORS};
use crate::voxel::chunk::SEA_LEVEL;
use super::terrain::TerrainGen;

/// Each LOD level covers a ring of tiles beyond normal render distance.
/// step = how many world-blocks per vertex in the heightmap grid.
struct LodLevel {
    step: usize,
    tile_blocks: usize, // world blocks per tile side
    grid_size: usize,   // vertices per tile side
}

const LOD_LEVELS: [LodLevel; 3] = [
    LodLevel { step: 4, tile_blocks: 64, grid_size: 17 },   // near: 4-block res, 64-block tiles
    LodLevel { step: 8, tile_blocks: 128, grid_size: 17 },  // mid: 8-block res, 128-block tiles
    LodLevel { step: 16, tile_blocks: 256, grid_size: 17 }, // far: 16-block res, 256-block tiles
];

/// Rings define which LOD level renders at what distance (in chunks from player).
/// Ring 0: normal render distance (handled by ChunkManager)
/// Ring 1: LOD 0 (near) — from render_distance out to 2x
/// Ring 2: LOD 1 (mid) — from 2x to 4x
/// Ring 3: LOD 2 (far) — from 4x to 8x
const LOD_RING_MULTIPLIERS: [(f32, f32); 3] = [
    (1.0, 2.0),   // near LOD: 1x to 2x render distance
    (2.0, 4.0),   // mid LOD: 2x to 4x render distance
    (4.0, 8.0),   // far LOD: 4x to 8x render distance
];

pub struct LodMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

pub struct LodTerrain {
    /// Meshes keyed by (lod_level, tile_x, tile_z)
    meshes: HashMap<(usize, i32, i32), LodMesh>,
    render_distance: i32,
}

impl LodTerrain {
    pub fn new(render_distance: i32) -> Self {
        Self {
            meshes: HashMap::new(),
            render_distance,
        }
    }

    /// Update LOD tiles around the player. Generates new tiles, removes distant ones.
    pub fn update(
        &mut self,
        player_cx: i32,
        player_cz: i32,
        terrain: &TerrainGen,
        device: &wgpu::Device,
    ) {
        for (lod_idx, lod_level) in LOD_LEVELS.iter().enumerate() {
            let (ring_start, ring_end) = LOD_RING_MULTIPLIERS[lod_idx];

            // Convert chunk-based render distance to tile coordinates
            let tile_blocks = lod_level.tile_blocks as i32;
            let player_tx = (player_cx * 16).div_euclid(tile_blocks);
            let player_tz = (player_cz * 16).div_euclid(tile_blocks);

            // Range of tiles to keep loaded (in tile coords)
            let rd_blocks = self.render_distance as f32 * 16.0;
            let min_dist_tiles = (rd_blocks * ring_start / tile_blocks as f32).ceil() as i32;
            let max_dist_tiles = (rd_blocks * ring_end / tile_blocks as f32).ceil() as i32;

            // Load tiles in the ring
            for tz in (player_tz - max_dist_tiles)..=(player_tz + max_dist_tiles) {
                for tx in (player_tx - max_dist_tiles)..=(player_tx + max_dist_tiles) {
                    let dx = (tx - player_tx).abs();
                    let dz = (tz - player_tz).abs();
                    let dist = dx.max(dz);

                    if dist < min_dist_tiles || dist > max_dist_tiles {
                        continue;
                    }

                    let key = (lod_idx, tx, tz);
                    if self.meshes.contains_key(&key) {
                        continue;
                    }

                    // Generate this LOD tile
                    if let Some(mesh) = build_lod_tile(terrain, lod_level, tx, tz, device) {
                        self.meshes.insert(key, mesh);
                    }
                }
            }

            // Unload tiles too far away
            let unload_dist = max_dist_tiles + 2;
            let to_remove: Vec<(usize, i32, i32)> = self
                .meshes
                .keys()
                .filter(|(l, tx, tz)| {
                    *l == lod_idx
                        && ((tx - player_tx).abs() > unload_dist
                            || (tz - player_tz).abs() > unload_dist)
                })
                .copied()
                .collect();

            for key in to_remove {
                self.meshes.remove(&key);
            }
        }
    }

    /// Iterate all LOD meshes for rendering.
    pub fn meshes(&self) -> impl Iterator<Item = &LodMesh> {
        self.meshes.values()
    }
}

/// Build a single LOD tile mesh from heightmap samples.
fn build_lod_tile(
    terrain: &TerrainGen,
    level: &LodLevel,
    tile_x: i32,
    tile_z: i32,
    device: &wgpu::Device,
) -> Option<LodMesh> {
    let tile_blocks = level.tile_blocks as i32;
    let base_wx = tile_x * tile_blocks;
    let base_wz = tile_z * tile_blocks;
    let step = level.step as i32;
    let grid = level.grid_size;

    let mut vertices = Vec::with_capacity(grid * grid);
    let mut heights = Vec::with_capacity(grid * grid);

    for gz in 0..grid {
        for gx in 0..grid {
            let wx = base_wx + gx as i32 * step;
            let wz = base_wz + gz as i32 * step;

            let (raw_h, biome_val) = terrain.sample_raw_pub(wx as f64, wz as f64);
            let h = (raw_h as usize).clamp(1, 255);
            heights.push(h);

            // Pick color based on biome/altitude (simplified surface_block logic)
            let color = lod_surface_color(h, biome_val);

            vertices.push(Vertex {
                position: [wx as f32, h as f32, wz as f32],
                color,
                normal: [0.0, 1.0, 0.0], // placeholder, computed below
            });
        }
    }

    // Compute normals from neighboring heights
    for gz in 0..grid {
        for gx in 0..grid {
            let idx = gz * grid + gx;
            let h_c = heights[idx] as f32;

            let h_l = if gx > 0 { heights[idx - 1] as f32 } else { h_c };
            let h_r = if gx < grid - 1 { heights[idx + 1] as f32 } else { h_c };
            let h_u = if gz > 0 { heights[idx - grid] as f32 } else { h_c };
            let h_d = if gz < grid - 1 { heights[idx + grid] as f32 } else { h_c };

            let step_f = step as f32;
            // Central difference normal
            let nx = (h_l - h_r) / (2.0 * step_f);
            let nz = (h_u - h_d) / (2.0 * step_f);
            let len = (nx * nx + 1.0 + nz * nz).sqrt();

            vertices[idx].normal = [nx / len, 1.0 / len, nz / len];
        }
    }

    // Build index buffer (triangle grid)
    let cells = grid - 1;
    let mut indices = Vec::with_capacity(cells * cells * 6);
    for gz in 0..cells {
        for gx in 0..cells {
            let tl = (gz * grid + gx) as u32;
            let tr = tl + 1;
            let bl = tl + grid as u32;
            let br = bl + 1;
            indices.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
        }
    }

    if vertices.is_empty() {
        return None;
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("lod_vertex"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("lod_index"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    Some(LodMesh {
        vertex_buffer,
        index_buffer,
        num_indices: indices.len() as u32,
    })
}

/// Simplified surface color for LOD tiles — no per-block detail, just biome + altitude.
fn lod_surface_color(h: usize, biome_val: f64) -> [f32; 3] {
    let altitude = h as f64 - SEA_LEVEL as f64;

    // Below sea level — show water color
    if h <= SEA_LEVEL {
        return BLOCK_COLORS[Block::Water as usize];
    }

    // Beach
    if altitude <= 1.5 {
        return BLOCK_COLORS[Block::Sand as usize];
    }

    // High mountain peaks — rock
    if biome_val > 0.3 && altitude > 70.0 {
        return BLOCK_COLORS[Block::Stone as usize];
    }

    // High highland — mix grass/rock
    if biome_val > 0.3 && altitude > 50.0 {
        let t = ((altitude - 50.0) / 20.0).min(1.0) as f32;
        let grass = BLOCK_COLORS[Block::Grass as usize];
        let stone = BLOCK_COLORS[Block::Stone as usize];
        return [
            grass[0] + (stone[0] - grass[0]) * t,
            grass[1] + (stone[1] - grass[1]) * t,
            grass[2] + (stone[2] - grass[2]) * t,
        ];
    }

    // Default: grass
    BLOCK_COLORS[Block::Grass as usize]
}
