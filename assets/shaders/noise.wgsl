struct Params {
    freq: f32;
    amp: f32;
    lacunarity: f32;
    gain: f32;
    octaves: u32;
};

struct Position {
    pos: vec4<f32>;
};

struct Values {
    data : [[stride(4)]] array<f32>;
};


// [[group(0), binding(0)]]
// var<uniform> params: Params;

[[group(0), binding(0)]]
var<uniform> pos: Position;

[[group(0), binding(1)]]
var<storage, read_write> values: Values;

let one: vec3<f32> = vec3<f32>(1.0, 1.0, 1.0);
let zero: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);

fn to_index(pos: vec3<u32>) -> i32 {
    return i32(pos.x << 0u | pos.y << 10u | pos.z << 5u);
}

fn mod289(x: vec4<f32>) -> vec4<f32>{return x - floor(x * (1.0 / 289.0)) * 289.0;}
fn perm(x: vec4<f32>) -> vec4<f32>{return mod289(((x * 34.0) + 1.0) * x);}

fn noise(pos: vec3<f32>) -> f32{
    let a: vec3<f32> = floor(pos);
    var d : vec3<f32> = fract(pos);
    d = d * d * (3.0 - 2.0 * d);

    let b : vec4<f32>= a.xxyy + vec4<f32>(0.0, 1.0, 0.0, 1.0);
    let k1: vec4<f32> = perm(b.xyxy);
    let k2: vec4<f32> = perm(k1.xyxy + b.zzww);

    let c : vec4<f32>= k2 + a.zzzz;
    let k3: vec4<f32> = perm(c);
    let k4: vec4<f32> = perm(c + 1.0);

    let o1: vec4<f32> = fract(k3 * (1.0 / 41.0));
    let o2: vec4<f32> = fract(k4 * (1.0 / 41.0));

    let o3: vec4<f32> = o2 * d.z + o1 * (1.0 - d.z);
    let o4: vec2<f32> = o3.yw * d.x + o3.xz * (1.0 - d.x);

    return o4.y * d.y + o4.x * (1.0 - d.y);
}

[[stage(compute), workgroup_size(8, 8, 8)]]
fn main([[builtin(global_invocation_id)]] id: vec3<u32>) {
    let pos: vec3<i32> = vec3<i32>(pos.pos.xyz) + vec3<i32>(id);
    
    var amp = 100.0;
    var freq = 0.002;

    var density: f32 = f32(-pos.y);

    for (var i = 0; i < 10; i = i+1) {
        density = density + noise(vec3<f32>(pos) * freq) * amp;
        amp = amp * 0.5;
        freq = freq * 2.0;
    }    

    values.data[to_index(id)] = density;
}
