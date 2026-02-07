use super::block::{Block, BLOCK_COLORS};
use super::chunk::{Chunk, CHUNK_SIZE, SEA_LEVEL, WORLD_HEIGHT};
use crate::renderer::Vertex;

/// Face definitions: normal, neighbor offset, 4 corner positions, shade factor.
struct FaceDef {
    normal: [f32; 3],
    offset: [i32; 3],
    corners: [[i32; 3]; 4],
    shade: f32,
}

const FACES: [FaceDef; 6] = [
    // +X
    FaceDef {
        normal: [1.0, 0.0, 0.0],
        offset: [1, 0, 0],
        corners: [[1, 0, 0], [1, 1, 0], [1, 1, 1], [1, 0, 1]],
        shade: 0.84,
    },
    // -X
    FaceDef {
        normal: [-1.0, 0.0, 0.0],
        offset: [-1, 0, 0],
        corners: [[0, 0, 1], [0, 1, 1], [0, 1, 0], [0, 0, 0]],
        shade: 0.72,
    },
    // +Y (top)
    FaceDef {
        normal: [0.0, 1.0, 0.0],
        offset: [0, 1, 0],
        corners: [[0, 1, 1], [1, 1, 1], [1, 1, 0], [0, 1, 0]],
        shade: 1.0,
    },
    // -Y (bottom)
    FaceDef {
        normal: [0.0, -1.0, 0.0],
        offset: [0, -1, 0],
        corners: [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]],
        shade: 0.55,
    },
    // +Z
    FaceDef {
        normal: [0.0, 0.0, 1.0],
        offset: [0, 0, 1],
        corners: [[1, 0, 1], [1, 1, 1], [0, 1, 1], [0, 0, 1]],
        shade: 0.8,
    },
    // -Z
    FaceDef {
        normal: [0.0, 0.0, -1.0],
        offset: [0, 0, -1],
        corners: [[0, 0, 0], [0, 1, 0], [1, 1, 0], [1, 0, 0]],
        shade: 0.75,
    },
];

fn should_render_face(current: Block, neighbor: Block) -> bool {
    if current == Block::Air {
        return false;
    }
    if current == Block::Water {
        return neighbor == Block::Air;
    }
    neighbor == Block::Air || neighbor == Block::Water
}

pub struct VoxelMesher;

impl VoxelMesher {
    /// Build vertex + index buffers for a chunk. Returns None if empty.
    pub fn build(chunk: &Chunk) -> Option<(Vec<Vertex>, Vec<u32>)> {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let base_x = chunk.cx * CHUNK_SIZE as i32;
        let base_z = chunk.cz * CHUNK_SIZE as i32;

        for y in 0..WORLD_HEIGHT {
            for lz in 0..CHUNK_SIZE {
                for lx in 0..CHUNK_SIZE {
                    let block = chunk.get(lx, y, lz);
                    if block == Block::Air {
                        continue;
                    }

                    let wx = base_x + lx as i32;
                    let wz = base_z + lz as i32;

                    // Altitude tint (matches original JS)
                    let altitude = (0.88 + (y as f32 - SEA_LEVEL as f32) * 0.004).clamp(0.7, 1.15);
                    let base_color = BLOCK_COLORS[block as usize];

                    for face in &FACES {
                        let nx = wx + face.offset[0];
                        let ny = y as i32 + face.offset[1];
                        let nz = wz + face.offset[2];

                        let neighbor = get_block_world(chunk, nx, ny, nz);
                        if !should_render_face(block, neighbor) {
                            continue;
                        }

                        let base_idx = vertices.len() as u32;
                        let shade = face.shade * altitude;

                        for corner in &face.corners {
                            vertices.push(Vertex {
                                position: [
                                    (wx + corner[0]) as f32,
                                    (y as i32 + corner[1]) as f32,
                                    (wz + corner[2]) as f32,
                                ],
                                color: [
                                    (base_color[0] * shade).min(1.0),
                                    (base_color[1] * shade).min(1.0),
                                    (base_color[2] * shade).min(1.0),
                                ],
                                normal: face.normal,
                            });
                        }

                        indices.extend_from_slice(&[
                            base_idx,
                            base_idx + 1,
                            base_idx + 2,
                            base_idx,
                            base_idx + 2,
                            base_idx + 3,
                        ]);
                    }
                }
            }
        }

        if indices.is_empty() {
            None
        } else {
            Some((vertices, indices))
        }
    }
}

/// Get block at world coordinates. For blocks outside this chunk, treat as air.
fn get_block_world(chunk: &Chunk, wx: i32, wy: i32, wz: i32) -> Block {
    if wy < 0 || wy >= WORLD_HEIGHT as i32 {
        return Block::Air;
    }

    let lx = wx - chunk.cx * CHUNK_SIZE as i32;
    let lz = wz - chunk.cz * CHUNK_SIZE as i32;

    if lx < 0 || lx >= CHUNK_SIZE as i32 || lz < 0 || lz >= CHUNK_SIZE as i32 {
        return Block::Air; // Cross-chunk: treat as air for now
    }

    chunk.get(lx as usize, wy as usize, lz as usize)
}
