use super::block::{Block, BLOCK_COLORS};
use super::chunk::{Chunk, CHUNK_SIZE, SEA_LEVEL, WORLD_HEIGHT};
use crate::renderer::Vertex;

/// Face definitions: normal, neighbor offset, 4 corner positions.
/// Shade values per face match Minecraft's directional lighting:
/// Top=1.0, Bottom=0.5, North/South=0.8, East/West=0.6
struct FaceDef {
    normal: [f32; 3],
    offset: [i32; 3],
    corners: [[i32; 3]; 4],
    shade: f32,
    /// For each corner, the 3 neighbors to check for ambient occlusion.
    /// Each is relative to the face's base block position.
    ao_neighbors: [[[i32; 3]; 3]; 4],
}

const FACES: [FaceDef; 6] = [
    // +X (East) — shade 0.6
    FaceDef {
        normal: [1.0, 0.0, 0.0],
        offset: [1, 0, 0],
        corners: [[1, 0, 0], [1, 1, 0], [1, 1, 1], [1, 0, 1]],
        shade: 0.6,
        ao_neighbors: [
            [[1, -1, 0], [1, 0, -1], [1, -1, -1]], // corner 0: bottom-back
            [[1, 1, 0],  [1, 0, -1], [1, 1, -1]],  // corner 1: top-back
            [[1, 1, 0],  [1, 0, 1],  [1, 1, 1]],   // corner 2: top-front
            [[1, -1, 0], [1, 0, 1],  [1, -1, 1]],  // corner 3: bottom-front
        ],
    },
    // -X (West) — shade 0.6
    FaceDef {
        normal: [-1.0, 0.0, 0.0],
        offset: [-1, 0, 0],
        corners: [[0, 0, 1], [0, 1, 1], [0, 1, 0], [0, 0, 0]],
        shade: 0.6,
        ao_neighbors: [
            [[-1, -1, 0], [-1, 0, 1],  [-1, -1, 1]],
            [[-1, 1, 0],  [-1, 0, 1],  [-1, 1, 1]],
            [[-1, 1, 0],  [-1, 0, -1], [-1, 1, -1]],
            [[-1, -1, 0], [-1, 0, -1], [-1, -1, -1]],
        ],
    },
    // +Y (Top) — shade 1.0
    FaceDef {
        normal: [0.0, 1.0, 0.0],
        offset: [0, 1, 0],
        corners: [[0, 1, 1], [1, 1, 1], [1, 1, 0], [0, 1, 0]],
        shade: 1.0,
        ao_neighbors: [
            [[0, 1, 1],  [-1, 1, 0], [-1, 1, 1]],
            [[0, 1, 1],  [1, 1, 0],  [1, 1, 1]],
            [[0, 1, -1], [1, 1, 0],  [1, 1, -1]],
            [[0, 1, -1], [-1, 1, 0], [-1, 1, -1]],
        ],
    },
    // -Y (Bottom) — shade 0.5
    FaceDef {
        normal: [0.0, -1.0, 0.0],
        offset: [0, -1, 0],
        corners: [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]],
        shade: 0.5,
        ao_neighbors: [
            [[0, -1, -1], [-1, -1, 0], [-1, -1, -1]],
            [[0, -1, -1], [1, -1, 0],  [1, -1, -1]],
            [[0, -1, 1],  [1, -1, 0],  [1, -1, 1]],
            [[0, -1, 1],  [-1, -1, 0], [-1, -1, 1]],
        ],
    },
    // +Z (South) — shade 0.8
    FaceDef {
        normal: [0.0, 0.0, 1.0],
        offset: [0, 0, 1],
        corners: [[1, 0, 1], [1, 1, 1], [0, 1, 1], [0, 0, 1]],
        shade: 0.8,
        ao_neighbors: [
            [[1, 0, 1],  [0, -1, 1], [1, -1, 1]],
            [[1, 0, 1],  [0, 1, 1],  [1, 1, 1]],
            [[-1, 0, 1], [0, 1, 1],  [-1, 1, 1]],
            [[-1, 0, 1], [0, -1, 1], [-1, -1, 1]],
        ],
    },
    // -Z (North) — shade 0.8
    FaceDef {
        normal: [0.0, 0.0, -1.0],
        offset: [0, 0, -1],
        corners: [[0, 0, 0], [0, 1, 0], [1, 1, 0], [1, 0, 0]],
        shade: 0.8,
        ao_neighbors: [
            [[-1, 0, -1], [0, -1, -1], [-1, -1, -1]],
            [[-1, 0, -1], [0, 1, -1],  [-1, 1, -1]],
            [[1, 0, -1],  [0, 1, -1],  [1, 1, -1]],
            [[1, 0, -1],  [0, -1, -1], [1, -1, -1]],
        ],
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

/// Compute ambient occlusion for a corner (0-3 scale).
/// 0 = fully occluded (darkest), 3 = fully open (brightest).
fn compute_ao(side1: bool, side2: bool, corner: bool) -> u8 {
    if side1 && side2 {
        return 0;
    }
    3 - (side1 as u8 + side2 as u8 + corner as u8)
}

/// Map AO value (0-3) to a brightness multiplier.
fn ao_brightness(ao: u8) -> f32 {
    match ao {
        0 => 0.5,
        1 => 0.65,
        2 => 0.8,
        _ => 1.0,
    }
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

                    // Altitude tint
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

                        // Compute AO for each of the 4 corners
                        let mut ao_values = [3u8; 4];
                        for (ci, ao_nbrs) in face.ao_neighbors.iter().enumerate() {
                            let s1 = is_solid_at(chunk, wx + ao_nbrs[0][0], y as i32 + ao_nbrs[0][1], wz + ao_nbrs[0][2]);
                            let s2 = is_solid_at(chunk, wx + ao_nbrs[1][0], y as i32 + ao_nbrs[1][1], wz + ao_nbrs[1][2]);
                            let cr = is_solid_at(chunk, wx + ao_nbrs[2][0], y as i32 + ao_nbrs[2][1], wz + ao_nbrs[2][2]);
                            ao_values[ci] = compute_ao(s1, s2, cr);
                        }

                        let base_idx = vertices.len() as u32;

                        for (ci, corner) in face.corners.iter().enumerate() {
                            let ao = ao_brightness(ao_values[ci]);
                            let shade = face.shade * altitude * ao;

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

                        // Flip quad diagonal if AO is anisotropic — prevents ugly diagonal artifacts
                        if ao_values[0] + ao_values[2] > ao_values[1] + ao_values[3] {
                            indices.extend_from_slice(&[
                                base_idx,
                                base_idx + 1,
                                base_idx + 2,
                                base_idx,
                                base_idx + 2,
                                base_idx + 3,
                            ]);
                        } else {
                            indices.extend_from_slice(&[
                                base_idx + 1,
                                base_idx + 2,
                                base_idx + 3,
                                base_idx + 1,
                                base_idx + 3,
                                base_idx,
                            ]);
                        }
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

/// Check if block at world coords is solid (for AO).
fn is_solid_at(chunk: &Chunk, wx: i32, wy: i32, wz: i32) -> bool {
    let block = get_block_world(chunk, wx, wy, wz);
    block.is_solid()
}

/// Get block at world coordinates. For blocks outside this chunk, treat as air.
fn get_block_world(chunk: &Chunk, wx: i32, wy: i32, wz: i32) -> Block {
    if wy < 0 || wy >= WORLD_HEIGHT as i32 {
        return Block::Air;
    }

    let lx = wx - chunk.cx * CHUNK_SIZE as i32;
    let lz = wz - chunk.cz * CHUNK_SIZE as i32;

    if lx < 0 || lx >= CHUNK_SIZE as i32 || lz < 0 || lz >= CHUNK_SIZE as i32 {
        return Block::Air;
    }

    chunk.get(lx as usize, wy as usize, lz as usize)
}
