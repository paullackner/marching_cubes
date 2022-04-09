use bevy::{prelude::*, math::Vec4Swizzles, render::{render_resource::PrimitiveTopology, mesh::{VertexAttributeValues, Indices}}};
use rand::prelude::*;
use std::fmt;

use super::marching_cubes_tables::{TRI_TABLE, CORNER_INDEX_AFROM_EDGE, CORNER_INDEX_BFROM_EDGE};

pub const AXIS_SIZE: usize = 16;
pub const BUFFER_SIZE: usize = AXIS_SIZE * AXIS_SIZE * AXIS_SIZE;

// big brain bit masks and shifts
pub const Y_MASK: usize = 0b_1111_0000_0000;
pub const Z_MASK: usize = 0b_0000_1111_0000;
pub const X_MASK: usize = 0b_0000_0000_1111;

pub const Y_SHIFT: usize = 8;
pub const Z_SHIFT: usize = 4;
pub const X_SHIFT: usize = 0;

pub const ISO_LEVEL: u8 = 128;


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

pub fn gen_cube(start: IVec3) -> [IVec3; 8] {
    [
        /*0*/start,
        /*1*/IVec3::new(start.x + 1, start.y, start.z),
        /*2*/IVec3::new(start.x + 1, start.y, start.z + 1),
        /*3*/IVec3::new(start.x, start.y, start.z + 1),
        /*4*/IVec3::new(start.x, start.y + 1, start.z),
        /*5*/IVec3::new(start.x + 1, start.y + 1, start.z),
        /*6*/IVec3::new(start.x + 1, start.y + 1, start.z + 1),
        /*7*/IVec3::new(start.x, start.y + 1, start.z + 1),
    ]
}

fn interpolate_verts(v1: IVec4, v2: IVec4) -> Vec3{
    let t = (ISO_LEVEL as i32 - v1.w) as f32 / (v2.w - v1.w) as f32;
    println!("{v1}{v2}");
    v1.xyz().as_vec3() + t * (v2.xyz() - v1.xyz()).as_vec3()
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Chunk {
    points: [u8; BUFFER_SIZE],
    dirty: bool,
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new_empty()
    }
}


impl Chunk {
    pub fn new_empty() -> Self {
        Self {points: [0; BUFFER_SIZE], dirty: false}
    }

    pub fn get_cube(self, pos: IVec3) -> [IVec4; 8] {
        // if pos.max_element() > 14 {
        //     return [IVec4::ZERO; 8];
        // }
        
        
        let cube = gen_cube(pos);
        cube.map(|x| { IVec4::new(x.x, x.y, x.z, self.points[to_index(x)] as i32) })
    
    }

    pub fn set_point(mut self, pos: IVec3, value: u8) {
        self.points[to_index(pos)] = value;
        self.dirty = true;
    }
}

#[derive(Bundle)]
pub struct ChunkBundle {
    pub chunk: Chunk,

    #[bundle]
    pub pbr: PbrBundle,
}

// #[derive(Debug)]
// pub struct Triangle {
//     pub a: Vec3,
//     pub b: Vec3,
//     pub c: Vec3,
// }

// impl fmt::Display for Triangle {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.write_fmt(format_args!("({}, {}, {})", self.a, self.b, self.c))
//     }
// }

// // impl Into<VertexAttributeValues> for  Triangle{
// //     fn into(self) -> VertexAttributeValues {
        
// //     }
// // }


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
            if from_index(i).max_element() >= 15 {continue;}
            let points = chunk.get_cube(from_index(i));
            

            let mut index = 0;
            
            if points[0].w as u8 > ISO_LEVEL {index |= 1}
            if points[1].w as u8 > ISO_LEVEL {index |= 2}
            if points[2].w as u8 > ISO_LEVEL {index |= 4}
            if points[3].w as u8 > ISO_LEVEL {index |= 8}
            if points[4].w as u8 > ISO_LEVEL {index |= 16}
            if points[5].w as u8 > ISO_LEVEL {index |= 32}
            if points[6].w as u8 > ISO_LEVEL {index |= 64}
            if points[7].w as u8 > ISO_LEVEL {index |= 128}
            
            if index == 0 { continue; }
            // print!("{}/{i}={}~{}: ",from_index(i),chunk.points[i],to_index(from_index(i)));
            // println!("{:?}-{:#10b}", points, index);
            
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

                // print!("[a: {}-{}], [b: {}-{}], [c: {}-{}] ->", points[a0 as usize], points[b0 as usize], points[a1 as usize], points[b1 as usize], points[a2 as usize], points[b2 as usize]);
                vertices.push(interpolate_verts(points[a0 as usize], points[b0 as usize]).into());
                vertices.push(interpolate_verts(points[a1 as usize], points[b1 as usize]).into());
                vertices.push(interpolate_verts(points[a2 as usize], points[b2 as usize]).into());
            }

        }


        let length = vertices.len() as u32;
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        let  indices = (0..length).collect::<Vec<u32>>();
        let mut normals: Vec<[f32; 3]> = Vec::new();
        let uvs: Vec<[f32; 2]> = vec![[0.0, 0.0]; length as usize];

        for chunk in indices.chunks(3) {
            let a = Vec3::from(vertices[chunk[0] as usize]);
            let b = Vec3::from(vertices[chunk[1] as usize]);
            let c = Vec3::from(vertices[chunk[2] as usize]);

            // println!("({:?}{:?}{:?})",a,b,c);

            let normal = (a - b).cross(a - c);
            normals.push(normal.into());
            normals.push(normal.into());
            normals.push(normal.into());
        }


        // println!("{} - {:?}", length, indices);


        mesh.set_indices(Some(Indices::U32(indices)));
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        

        
        *meshes.get_mut(mesh_handle.id).unwrap() = mesh;
        chunk.dirty = false;
    }
}

fn set_points_system(
    mut query: Query<&mut Chunk>,
    key: Res<Input<KeyCode>>,
) {
    let mut rng = thread_rng();

    if key.just_pressed(KeyCode::G){
        for mut chunk in query.iter_mut() {
            for i in 0..BUFFER_SIZE-1 {
                if from_index(i).y < rng.gen_range(3..5) {
                    chunk.points[i] = rng.gen_range(130..200);
                }
                chunk.dirty = true;
            }
        }
    }
}