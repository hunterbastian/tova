pub mod flowers;
pub mod grass;
pub mod logs;
pub mod rocks;
pub mod ruins;
pub mod tree;

use std::collections::HashMap;
use glam::Vec3;
use wgpu::util::DeviceExt;

use crate::renderer::Vertex;
use crate::voxel::chunk::CHUNK_SIZE;
use flowers::generate_flower;
use grass::generate_grass;
use logs::generate_fallen_log;
use rocks::{generate_boulder, generate_rock_cluster, generate_rock_spire, generate_standing_stone, generate_rubble, generate_giant_spire};
use ruins::{generate_wall_fragment, generate_column, generate_arch_fragment};
use tree::generate_tree;

/// Deterministic hash for prop placement.
fn hash_pos(x: i32, z: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(374761393)
        .wrapping_add((z as u32).wrapping_mul(668265263));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^ (h >> 16)
}

pub struct PropMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

pub struct PropManager {
    meshes: HashMap<(i32, i32), PropMesh>,
}

impl PropManager {
    pub fn new() -> Self {
        Self {
            meshes: HashMap::new(),
        }
    }

    /// Generate prop meshes for a chunk given its terrain height samples.
    pub fn generate_chunk(
        &mut self,
        cx: i32,
        cz: i32,
        heights: &[(usize, usize, f32, bool)], // (lx, lz, surface_y, is_grass)
        device: &wgpu::Device,
    ) {
        let mut all_verts: Vec<Vertex> = Vec::new();
        let mut all_idxs: Vec<u32> = Vec::new();

        let base_x = cx * CHUNK_SIZE as i32;
        let base_z = cz * CHUNK_SIZE as i32;
        let sea_level = crate::voxel::chunk::SEA_LEVEL;

        for &(lx, lz, surface_y, is_grass) in heights {
            let wx = base_x + lx as i32;
            let wz = base_z + lz as i32;
            let sy = surface_y as usize;
            let h = hash_pos(wx, wz);

            // ─── Ruins — rare ancient fragments ──────────────────
            let h_ruin = hash_pos(wx.wrapping_add(5000), wz.wrapping_add(5000));

            // Arch fragments (rarest)
            if h_ruin % 800 == 0 && sy > sea_level + 5 {
                let base = Vec3::new(wx as f32 + 0.5, surface_y, wz as f32 + 0.5);
                let (v, i) = generate_arch_fragment(base, h_ruin);
                append_mesh(&mut all_verts, &mut all_idxs, &v, &i);
                continue;
            }

            // Wall fragments
            if h_ruin % 500 == 0 && sy > sea_level + 5 {
                let base = Vec3::new(wx as f32 + 0.5, surface_y, wz as f32 + 0.5);
                let (v, i) = generate_wall_fragment(base, h_ruin);
                append_mesh(&mut all_verts, &mut all_idxs, &v, &i);
                continue;
            }

            // Columns
            if h_ruin % 400 == 1 && sy > sea_level + 3 {
                let base = Vec3::new(wx as f32 + 0.5, surface_y, wz as f32 + 0.5);
                let (v, i) = generate_column(base, h_ruin);
                append_mesh(&mut all_verts, &mut all_idxs, &v, &i);
                continue;
            }

            // ─── Trees ─────────────────────────────────────────
            // Grove Hills: denser trees in the grove valley (near center)
            let grove_dx = wx as f64 - 64.0;
            let grove_dz = wz as f64;
            let grove_dist = (grove_dx * grove_dx + grove_dz * grove_dz).sqrt();
            let tree_threshold = if grove_dist < 20.0 {
                3  // Ancient heart of the forest — Fangorn density
            } else if grove_dist < 45.0 {
                4  // Dense ancient forest in the grove
            } else if grove_dist < 75.0 {
                8  // More trees on the surrounding hills
            } else {
                14 // Scattered trees in the mountains
            };

            if is_grass && h % tree_threshold == 0 && sy > sea_level + 3 && sy <= sea_level + 100 {
                let base = Vec3::new(wx as f32 + 0.5, surface_y, wz as f32 + 0.5);
                let altitude_t = ((sy - sea_level - 5) as f32 / 80.0).clamp(0.0, 1.0);
                let (v, i) = generate_tree(base, h, altitude_t);
                append_mesh(&mut all_verts, &mut all_idxs, &v, &i);
                continue;
            }

            // ─── Giant spires — towering formations in the north ──
            let h2 = hash_pos(wx.wrapping_add(1000), wz.wrapping_add(1000));
            if wz < -60 && h2 % 80 == 0 && sy > sea_level + 5 {
                let base = Vec3::new(wx as f32 + 0.5, surface_y, wz as f32 + 0.5);
                let (v, i) = generate_giant_spire(base, h2);
                append_mesh(&mut all_verts, &mut all_idxs, &v, &i);
                continue;
            }

            // ─── Rock spires — jagged stone columns on higher terrain ──
            if h2 % 200 == 0 && sy > sea_level + 15 {
                let base = Vec3::new(wx as f32 + 0.5, surface_y, wz as f32 + 0.5);
                let (v, i) = generate_rock_spire(base, h2);
                append_mesh(&mut all_verts, &mut all_idxs, &v, &i);
                continue;
            }

            // ─── Standing stones — ancient monoliths, flatter areas ──
            let h3 = hash_pos(wx.wrapping_add(2000), wz.wrapping_add(2000));
            if h3 % 350 == 0 && sy > sea_level + 3 && sy < sea_level + 50 {
                let base = Vec3::new(wx as f32 + 0.5, surface_y, wz as f32 + 0.5);
                let (v, i) = generate_standing_stone(base, h3);
                append_mesh(&mut all_verts, &mut all_idxs, &v, &i);
                continue;
            }

            // ─── Boulders — more frequent for dark fantasy ──────
            if h2 % 70 == 0 && sy > sea_level + 2 {
                let base = Vec3::new(wx as f32 + 0.5, surface_y, wz as f32 + 0.5);
                let (v, i) = generate_boulder(base, h2);
                append_mesh(&mut all_verts, &mut all_idxs, &v, &i);
                continue;
            }

            // ─── Small rock clusters — denser ────────────────────
            if h2 % 25 == 1 && sy > sea_level + 1 {
                let ox = ((h2 >> 4) % 60) as f32 * 0.01 + 0.2;
                let oz = ((h2 >> 8) % 60) as f32 * 0.01 + 0.2;
                let base = Vec3::new(wx as f32 + ox, surface_y, wz as f32 + oz);
                let (v, i) = generate_rock_cluster(base, h2);
                append_mesh(&mut all_verts, &mut all_idxs, &v, &i);
                continue;
            }

            // ─── Rubble — scattered stone fragments ──────────────
            let h4 = hash_pos(wx.wrapping_add(3000), wz.wrapping_add(3000));
            if h4 % 45 == 0 && sy > sea_level + 5 {
                let ox = ((h4 >> 4) % 60) as f32 * 0.01 + 0.2;
                let oz = ((h4 >> 8) % 60) as f32 * 0.01 + 0.2;
                let base = Vec3::new(wx as f32 + ox, surface_y, wz as f32 + oz);
                let (v, i) = generate_rubble(base, h4);
                append_mesh(&mut all_verts, &mut all_idxs, &v, &i);
                continue;
            }

            // ─── Fallen logs — decaying wood ──────────────────
            let h_log = hash_pos(wx.wrapping_add(4000), wz.wrapping_add(4000));
            if is_grass && h_log % 60 == 0 && sy > sea_level + 3 {
                let base = Vec3::new(wx as f32 + 0.5, surface_y, wz as f32 + 0.5);
                let (v, i) = generate_fallen_log(base, h_log);
                append_mesh(&mut all_verts, &mut all_idxs, &v, &i);
                continue;
            }

            if !is_grass || sy <= sea_level + 3 {
                continue;
            }

            // ─── Grass tufts — dense meadow coverage ────────────
            let gh = hash_pos(wz, wx);
            if gh % 3 == 0 {
                let ox = ((gh >> 4) % 80) as f32 * 0.01 + 0.1;
                let oz = ((gh >> 8) % 80) as f32 * 0.01 + 0.1;
                let base = Vec3::new(wx as f32 + ox, surface_y, wz as f32 + oz);
                let (v, i) = generate_grass(base, gh);
                append_mesh(&mut all_verts, &mut all_idxs, &v, &i);
            }

            // ─── Wildflowers — sparse color pops ──────────────
            let fh = hash_pos(wx.wrapping_add(500), wz.wrapping_add(500));
            if fh % 25 == 0 {
                let ox = ((fh >> 4) % 80) as f32 * 0.01 + 0.1;
                let oz = ((fh >> 8) % 80) as f32 * 0.01 + 0.1;
                let base = Vec3::new(wx as f32 + ox, surface_y, wz as f32 + oz);
                let (v, i) = generate_flower(base, fh);
                append_mesh(&mut all_verts, &mut all_idxs, &v, &i);
            }
        }

        if all_idxs.is_empty() {
            self.meshes.remove(&(cx, cz));
            return;
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("prop_vertex"),
            contents: bytemuck::cast_slice(&all_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("prop_index"),
            contents: bytemuck::cast_slice(&all_idxs),
            usage: wgpu::BufferUsages::INDEX,
        });

        self.meshes.insert((cx, cz), PropMesh {
            vertex_buffer,
            index_buffer,
            num_indices: all_idxs.len() as u32,
        });
    }

    /// Remove props for a chunk.
    pub fn remove_chunk(&mut self, cx: i32, cz: i32) {
        self.meshes.remove(&(cx, cz));
    }

    /// Iterate all prop meshes for rendering.
    pub fn meshes(&self) -> impl Iterator<Item = &PropMesh> {
        self.meshes.values()
    }
}

fn append_mesh(
    all_verts: &mut Vec<Vertex>,
    all_idxs: &mut Vec<u32>,
    new_verts: &[Vertex],
    new_idxs: &[u32],
) {
    let offset = all_verts.len() as u32;
    all_verts.extend_from_slice(new_verts);
    for idx in new_idxs {
        all_idxs.push(idx + offset);
    }
}
