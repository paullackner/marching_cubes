#import bevy_pbr::mesh_view_bind_group
#import bevy_pbr::mesh_struct

struct Vertex {
    [[location(0)]] position: vec3<f32>;    
    [[location(1)]] normal: vec3<f32>;    
};


struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
    [[location(1)]] world_position: vec4<f32>;
    [[location(2)]] world_normal: vec3<f32>;
};

fn inverse_transpose_3x3(in: mat3x3<f32>) -> mat3x3<f32> {
    let x = cross(in.y, in.z);
    let y = cross(in.z, in.x); 
    let z = cross(in.x, in.y);
    let det = dot(in.z, z);
    return mat3x3<f32>(
        x / det,
        y / det,
        z / det
    );
}

fn skin_normals(
    model: mat4x4<f32>,
    normal: vec3<f32>,
) -> vec3<f32> {
    return inverse_transpose_3x3(mat3x3<f32>(
        model[0].xyz,
        model[1].xyz,
        model[2].xyz
    )) * normal;
}

[[group(2), binding(0)]]
var<uniform> mesh: Mesh;

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    var world_position = mesh.model * vec4<f32>(vertex.position, 1.0);

    let height: f32 = (sin(world_position.y / 50.0)+1.0) * 0.5;

    var out: VertexOutput;
    out.clip_position = view.view_proj * world_position;
    out.color = vec4<f32>(0.5, height , 1.0 - height, 1.0);
    out.world_normal = skin_normals(mesh.model, vertex.normal);
    out.position = world_position;
    
    return out;
}

[[stage(fragment)]]
fn fragment(input: VertexOutput) -> [[location(0)]] vec4<f32> {
    let norm: vec3<f32> = normalize(input.world_normal);
    let lightdir = normalize(vec3<f32>(100.0, 50.0, -500.0) - input.world_position.xyz);
    let diff = max(dot(norm, lightdir), 0.0);

    return vec4<f32>(input.color.xyz * (diff + 0.1), 1.0);
}