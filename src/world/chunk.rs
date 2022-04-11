use std::thread;
use bevy::{prelude::*, math::Vec4Swizzles, render::{render_resource::PrimitiveTopology, mesh::{VertexAttributeValues, Indices}}};
use bevy::tasks::AsyncComputeTaskPool;
use rand::prelude::*;
use opensimplex_noise_rs::OpenSimplexNoise;

use super::marching_cubes_tables::{TRI_TABLE, CORNER_INDEX_AFROM_EDGE, CORNER_INDEX_BFROM_EDGE};

pub const AXIS_SIZE: usize = 32;
pub const BUFFER_SIZE: usize = AXIS_SIZE * AXIS_SIZE * AXIS_SIZE;

// big brain bit masks and shifts
pub const Y_MASK: usize = 0b_0111_1100_0000_0000;
pub const Z_MASK: usize = 0b_0000_0011_1110_0000;
pub const X_MASK: usize = 0b_0000_0000_0001_1111;

pub const Y_SHIFT: usize = 10;
pub const Z_SHIFT: usize = 5;
pub const X_SHIFT: usize = 0;

pub const ISO_LEVEL: f32 = 0.0;


pub fn to_index(local: IVec3) -> usize {
    (local.x << X_SHIFT | local.y << Y_SHIFT | local.z << Z_SHIFT) as usize
}

fn from_index(index: usize) -> IVec3 {
    IVec3::new(
        ((index & X_MASK) >> X_SHIFT) as i32,
        ((index & Y_MASK) >> Y_SHIFT) as i32,
        ((index & Z_MASK) >> Z_SHIFT) as i32,
    )
}
//         e6
//     7-------6
//    /|      /|
// e7/ |e11e5/ |e10
//  4--|e4--5  |
//e8|  3--e2|--2
//  | /e3 e9| /e1
//  0-------1
//      e0

pub fn gen_cube(start: Vec3) -> [Vec3; 8] {
    [
        /*0*/start,
        /*1*/Vec3::new(start.x + 1.0, start.y, start.z),
        /*2*/Vec3::new(start.x + 1.0, start.y, start.z + 1.0),
        /*3*/Vec3::new(start.x, start.y, start.z + 1.0),
        /*4*/Vec3::new(start.x, start.y + 1.0, start.z),
        /*5*/Vec3::new(start.x + 1.0, start.y + 1.0, start.z),
        /*6*/Vec3::new(start.x + 1.0, start.y + 1.0, start.z + 1.0),
        /*7*/Vec3::new(start.x, start.y + 1.0, start.z + 1.0),
    ]
}

fn interpolate_verts(v1: Vec4, v2: Vec4) -> Vec3{
    let t = (ISO_LEVEL - v1.w) / (v2.w - v1.w);
    v1.xyz() + t * (v2.xyz() - v1.xyz())
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Chunk {
    points: [f32; BUFFER_SIZE],
    dirty: bool,
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new_empty()
    }
}


impl Chunk {
    pub fn new_empty() -> Self {
        Self {points: [-1.0; BUFFER_SIZE], dirty: false}
    }

    pub fn new_filled() -> Self{


        Self {points: [-1.0; BUFFER_SIZE], dirty: true}
    }

    pub fn get_cube(self, pos: Vec3) -> [Vec4; 8] {
        // if pos.max_element() > 14 {
        //     return [IVec4::ZERO; 8];
        // }
        
        
        let cube = gen_cube(pos);
        cube.map(|x| { Vec4::new(x.x, x.y, x.z, self.points[to_index(x.as_ivec3())]) })
    
    }

    pub fn set_point(mut self, pos: Vec3, value: f32) {
        self.points[to_index(pos.as_ivec3())] = value;
        self.dirty = true;
    }
}

#[derive(Bundle)]
pub struct ChunkBundle {
    pub chunk: Chunk,

    #[bundle]
    pub pbr: PbrBundle,
}

#[derive(Debug)]
pub struct Triangle {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
}

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_system(march_cubes_system)
            .add_system(set_points_system);
    }
}

fn march_cubes_system(
    mut query: Query<(&Handle<Mesh>, &mut Chunk)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (mesh_handle, mut chunk) in query.iter_mut() {
        if !chunk.dirty {continue;}
        let mut vertices: Vec<[f32; 3]> = Vec::new();

        for i in 0..BUFFER_SIZE-1 {
                if from_index(i).max_element() >= AXIS_SIZE as i32 -1 {continue;}
                let points = chunk.get_cube(from_index(i).as_vec3());
                let mut triangles: Vec<Triangle> = Vec::with_capacity(4);


                let mut index = 0;

                if points[0].w >= ISO_LEVEL {index |= 1}
                if points[1].w >= ISO_LEVEL {index |= 2}
                if points[2].w >= ISO_LEVEL {index |= 4}
                if points[3].w >= ISO_LEVEL {index |= 8}
                if points[4].w >= ISO_LEVEL {index |= 16}
                if points[5].w >= ISO_LEVEL {index |= 32}
                if points[6].w >= ISO_LEVEL {index |= 64}
                if points[7].w >= ISO_LEVEL {index |= 128}

                if index == 0 { continue; }

                for i in (0..15).step_by(3) {

                    if TRI_TABLE[index][i] == -1 {
                        break;
                    }

                    let a0 = CORNER_INDEX_AFROM_EDGE[TRI_TABLE[index][i] as usize];
                    let b0 = CORNER_INDEX_BFROM_EDGE[TRI_TABLE[index][i] as usize];

                    let a1 = CORNER_INDEX_AFROM_EDGE[TRI_TABLE[index][i+1] as usize];
                    let b1 = CORNER_INDEX_BFROM_EDGE[TRI_TABLE[index][i+1] as usize];

                    let a2 = CORNER_INDEX_AFROM_EDGE[TRI_TABLE[index][i+2] as usize];
                    let b2 = CORNER_INDEX_BFROM_EDGE[TRI_TABLE[index][i+2] as usize];
                    let triangle = Triangle{
                        a: interpolate_verts(points[a0 as usize], points[b0 as usize]),
                        b: interpolate_verts(points[a1 as usize], points[b1 as usize]),
                        c: interpolate_verts(points[a2 as usize], points[b2 as usize]),
                    };
                    triangles.push(triangle);

                    // vertices.push(interpolate_verts(points[a0 as usize], points[b0 as usize]).into());
                    // vertices.push(interpolate_verts(points[a1 as usize], points[b1 as usize]).into());
                    // vertices.push(interpolate_verts(points[a2 as usize], points[b2 as usize]).into());
                }

                triangles.iter().for_each(|x| {
                    vertices.append(&mut vec![
                        x.a.to_array(),
                        x.b.to_array(),
                        x.c.to_array()
                    ])
                });
        }
        let length = vertices.len() as u32;
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        let  indices = (0..length).collect::<Vec<u32>>();
        let uvs: Vec<[f32; 2]> = vec![[0.0, 0.0]; length as usize];


        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.compute_flat_normals();
        mesh.set_indices(Some(Indices::U32(indices)));
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        

        
        *meshes.get_mut(mesh_handle.id).unwrap() = mesh;
        chunk.dirty = false;
        // println!("Mesh generated");
    }
}

fn set_points_system(
    mut query: Query<(&mut Chunk, &Transform)>,
    key: Res<Input<KeyCode>>,
) {
    if !key.just_pressed(KeyCode::G){
        return;
    }
    let simplex = OpenSimplexNoise::new(Some(69420));
    for (mut chunk, transform) in query.iter_mut() {

        for i in 0..BUFFER_SIZE-1 {
            chunk.points[i] = calc_iso(transform.translation + from_index(i).as_vec3(), &simplex);
        }
        chunk.dirty = true;   
    }

    println!("density calculated")
}

fn calc_iso(ws: Vec3, simplex: &OpenSimplexNoise) -> f32{
    let mut density = -ws.y;

    let mut freq = 0.005;
    let mut amplitude = 15.0;
    for _ in 0..=8 {
        density += (simplex.eval_3d(ws.x as f64 * freq, ws.y as f64 * freq, ws.z as f64 * freq) as f32 + 1.0) * amplitude;
        freq *= 2.0;
        amplitude *= 0.5;
    }
    density
}