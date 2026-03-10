use std::collections::HashMap;
use wgpu::util::DeviceExt;

use crate::voxel::block::Block;
use crate::voxel::chunk::{Chunk, CHUNK_SIZE, WORLD_HEIGHT};
use crate::voxel::mesher::VoxelMesher;
use super::terrain::TerrainGen;

pub struct ChunkMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

pub struct ChunkManager {
    terrain: TerrainGen,
    chunks: HashMap<(i32, i32), Chunk>,
    meshes: HashMap<(i32, i32), ChunkMesh>,
    render_distance: i32,
}

impl ChunkManager {
    pub fn new(render_distance: i32) -> Self {
        Self {
            terrain: TerrainGen::new(),
            chunks: HashMap::new(),
            meshes: HashMap::new(),
            render_distance,
        }
    }

    /// Get block at world coordinates. Returns Air for unloaded/out-of-bounds.
    pub fn get_block(&self, wx: i32, wy: i32, wz: i32) -> Block {
        if wy < 0 || wy >= WORLD_HEIGHT as i32 {
            return Block::Air;
        }
        let cx = wx.div_euclid(CHUNK_SIZE as i32);
        let cz = wz.div_euclid(CHUNK_SIZE as i32);
        let lx = wx.rem_euclid(CHUNK_SIZE as i32) as usize;
        let lz = wz.rem_euclid(CHUNK_SIZE as i32) as usize;

        self.chunks
            .get(&(cx, cz))
            .map(|c| c.get(lx, wy as usize, lz))
            .unwrap_or(Block::Air)
    }

    /// Check if block at world coordinates is solid.
    pub fn is_solid(&self, wx: i32, wy: i32, wz: i32) -> bool {
        self.get_block(wx, wy, wz).is_solid()
    }

    /// Generate initial chunks around origin.
    pub fn generate_initial(&mut self, device: &wgpu::Device) {
        let rd = self.render_distance;
        for cz in -rd..rd {
            for cx in -rd..rd {
                self.load_chunk(cx, cz, device);
            }
        }
    }

    /// Stream chunks around player position.
    pub fn update(&mut self, player_cx: i32, player_cz: i32, device: &wgpu::Device) {
        let rd = self.render_distance;

        // Load missing chunks within render distance
        for cz in (player_cz - rd)..(player_cz + rd) {
            for cx in (player_cx - rd)..(player_cx + rd) {
                if !self.chunks.contains_key(&(cx, cz)) {
                    self.load_chunk(cx, cz, device);
                }
            }
        }

        // Unload distant chunks (margin to avoid thrashing)
        let unload_dist = rd + 2;
        let to_remove: Vec<(i32, i32)> = self
            .chunks
            .keys()
            .filter(|(cx, cz)| {
                (cx - player_cx).abs() >= unload_dist
                    || (cz - player_cz).abs() >= unload_dist
            })
            .copied()
            .collect();

        for key in to_remove {
            self.chunks.remove(&key);
            self.meshes.remove(&key);
        }
    }

    fn load_chunk(&mut self, cx: i32, cz: i32, device: &wgpu::Device) {
        let mut chunk = Chunk::new(cx, cz);
        self.terrain.generate(&mut chunk);

        if let Some((vertices, indices)) = VoxelMesher::build(&chunk) {
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("chunk_vertex"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("chunk_index"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            self.meshes.insert(
                (cx, cz),
                ChunkMesh {
                    vertex_buffer,
                    index_buffer,
                    num_indices: indices.len() as u32,
                },
            );
        }

        self.chunks.insert((cx, cz), chunk);
    }

    /// Iterate all loaded meshes for rendering.
    pub fn meshes(&self) -> impl Iterator<Item = &ChunkMesh> {
        self.meshes.values()
    }
}
