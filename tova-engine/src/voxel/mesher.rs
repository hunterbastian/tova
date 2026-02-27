use super::block::{Block, BLOCK_COLORS};
use super::chunk::{Chunk, CHUNK_SIZE, SEA_LEVEL, WORLD_HEIGHT};
use crate::renderer::Vertex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FaceCell {
    block: Block,
    x: usize,
    y: usize,
    z: usize,
}

#[derive(Clone, Copy, Debug)]
enum FaceDir {
    Px,
    Nx,
    Py,
    Ny,
    Pz,
    Nz,
}

impl FaceDir {
    const ALL: [Self; 6] = [Self::Px, Self::Nx, Self::Py, Self::Ny, Self::Pz, Self::Nz];

    fn normal(self) -> [f32; 3] {
        match self {
            Self::Px => [1.0, 0.0, 0.0],
            Self::Nx => [-1.0, 0.0, 0.0],
            Self::Py => [0.0, 1.0, 0.0],
            Self::Ny => [0.0, -1.0, 0.0],
            Self::Pz => [0.0, 0.0, 1.0],
            Self::Nz => [0.0, 0.0, -1.0],
        }
    }

    fn offset(self) -> [i32; 3] {
        match self {
            Self::Px => [1, 0, 0],
            Self::Nx => [-1, 0, 0],
            Self::Py => [0, 1, 0],
            Self::Ny => [0, -1, 0],
            Self::Pz => [0, 0, 1],
            Self::Nz => [0, 0, -1],
        }
    }

    fn shade(self) -> f32 {
        match self {
            Self::Px | Self::Nx => 0.6,
            Self::Py => 1.0,
            Self::Ny => 0.5,
            Self::Pz | Self::Nz => 0.8,
        }
    }

    fn slice_count(self) -> usize {
        match self {
            Self::Px | Self::Nx | Self::Pz | Self::Nz => CHUNK_SIZE,
            Self::Py | Self::Ny => WORLD_HEIGHT,
        }
    }

    fn u_count(self) -> usize {
        match self {
            Self::Px | Self::Nx => WORLD_HEIGHT,
            Self::Py | Self::Ny | Self::Pz | Self::Nz => CHUNK_SIZE,
        }
    }

    fn v_count(self) -> usize {
        match self {
            Self::Px | Self::Nx | Self::Py | Self::Ny => CHUNK_SIZE,
            Self::Pz | Self::Nz => WORLD_HEIGHT,
        }
    }

    fn block_coords(self, slice: usize, u: usize, v: usize) -> (usize, usize, usize) {
        match self {
            Self::Px | Self::Nx => (slice, u, v),
            Self::Py | Self::Ny => (u, slice, v),
            Self::Pz | Self::Nz => (u, v, slice),
        }
    }
}

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
    /// Meshing using an external block lookup for cross-chunk culling.
    pub fn build_with_lookup<F>(
        chunk: &Chunk,
        mut sample_block: F,
    ) -> Option<(Vec<Vertex>, Vec<u32>)>
    where
        F: FnMut(i32, i32, i32) -> Block,
    {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let base_x = chunk.cx * CHUNK_SIZE as i32;
        let base_z = chunk.cz * CHUNK_SIZE as i32;

        for face in FaceDir::ALL {
            let slice_count = face.slice_count();
            let u_count = face.u_count();
            let v_count = face.v_count();
            let cell_count = u_count * v_count;
            let mut mask = vec![None; cell_count];
            let mut visited = vec![false; cell_count];

            for slice in 0..slice_count {
                mask.fill(None);
                visited.fill(false);

                for u in 0..u_count {
                    for v in 0..v_count {
                        let idx = u * v_count + v;
                        let (lx, ly, lz) = face.block_coords(slice, u, v);
                        let block = chunk.get(lx, ly, lz);
                        if block == Block::Air {
                            continue;
                        }

                        let wx = base_x + lx as i32;
                        let wy = ly as i32;
                        let wz = base_z + lz as i32;
                        let [ox, oy, oz] = face.offset();
                        let neighbor = sample_block(wx + ox, wy + oy, wz + oz);
                        if should_render_face(block, neighbor) {
                            mask[idx] = Some(FaceCell {
                                block,
                                x: lx,
                                y: ly,
                                z: lz,
                            });
                        }
                    }
                }

                for u in 0..u_count {
                    for v in 0..v_count {
                        let idx = u * v_count + v;
                        if visited[idx] {
                            continue;
                        }
                        let Some(cell) = mask[idx] else {
                            continue;
                        };

                        let mut width = 1;
                        while v + width < v_count {
                            let next_idx = u * v_count + (v + width);
                            if visited[next_idx] {
                                break;
                            }
                            match mask[next_idx] {
                                Some(other) if other.block == cell.block => width += 1,
                                _ => break,
                            }
                        }

                        let mut height = 1;
                        'grow_height: while u + height < u_count {
                            for dv in 0..width {
                                let next_idx = (u + height) * v_count + (v + dv);
                                if visited[next_idx] {
                                    break 'grow_height;
                                }
                                match mask[next_idx] {
                                    Some(other) if other.block == cell.block => {}
                                    _ => break 'grow_height,
                                }
                            }
                            height += 1;
                        }

                        for du in 0..height {
                            for dv in 0..width {
                                visited[(u + du) * v_count + (v + dv)] = true;
                            }
                        }

                        emit_merged_face(
                            &mut vertices,
                            &mut indices,
                            face,
                            cell,
                            width as f32,
                            height as f32,
                        );
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

fn emit_merged_face(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    face: FaceDir,
    cell: FaceCell,
    width: f32,
    height: f32,
) {
    let x = cell.x as f32;
    let y = cell.y as f32;
    let z = cell.z as f32;

    let positions = match face {
        FaceDir::Px => [
            [x + 1.0, y, z],
            [x + 1.0, y + height, z],
            [x + 1.0, y + height, z + width],
            [x + 1.0, y, z + width],
        ],
        FaceDir::Nx => [
            [x, y, z + width],
            [x, y + height, z + width],
            [x, y + height, z],
            [x, y, z],
        ],
        FaceDir::Py => [
            [x, y + 1.0, z + width],
            [x + height, y + 1.0, z + width],
            [x + height, y + 1.0, z],
            [x, y + 1.0, z],
        ],
        FaceDir::Ny => [
            [x, y, z],
            [x + height, y, z],
            [x + height, y, z + width],
            [x, y, z + width],
        ],
        FaceDir::Pz => [
            [x + height, y, z + 1.0],
            [x + height, y + width, z + 1.0],
            [x, y + width, z + 1.0],
            [x, y, z + 1.0],
        ],
        FaceDir::Nz => [
            [x, y, z],
            [x, y + width, z],
            [x + height, y + width, z],
            [x + height, y, z],
        ],
    };

    push_quad(
        vertices,
        indices,
        positions,
        face.normal(),
        cell.block,
        face.shade(),
    );
}

fn push_quad(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    positions: [[f32; 3]; 4],
    normal: [f32; 3],
    block: Block,
    face_shade: f32,
) {
    let base_idx = vertices.len() as u32;
    for position in positions {
        vertices.push(Vertex {
            position,
            color: compute_face_color(block, face_shade, position[1]),
            normal,
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

fn compute_face_color(block: Block, face_shade: f32, y: f32) -> [f32; 3] {
    let altitude = (0.88 + (y - SEA_LEVEL as f32) * 0.004).clamp(0.7, 1.15);
    let shade = face_shade * altitude;
    let base = BLOCK_COLORS[block as usize];
    [
        (base[0] * shade).min(1.0),
        (base[1] * shade).min(1.0),
        (base[2] * shade).min(1.0),
    ]
}
