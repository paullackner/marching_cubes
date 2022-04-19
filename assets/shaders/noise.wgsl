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

let one: vec4<f32> = vec4<f32>(1.0, 1.0, 1.0, 1.0);
let zero: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);

fn to_index(pos: vec3<u32>) -> i32 {
    return i32(pos.x << 0u | pos.y << 10u | pos.z << 5u);
}

fn permute(x: vec4<f32>) -> vec4<f32> {
    var temp: vec4<f32> = 289.0 * one;
    return modf(((x*34.0) + one) * x, &temp);
}

fn taylorInvSqrt(r: vec4<f32>) -> vec4<f32> {
    return 1.79284291400159 * one - 0.85373472095314 * r;
}

fn noise(v: vec3<f32>) -> f32{
    let C = vec2<f32>(1.0/6.0, 1.0/3.0);
    let D = vec4<f32>(0.0, 0.5, 1.0, 2.0);

    // First corner
    //TODO: use the splat operations when available
    let vCy = dot(v, C.yyy);
    var i: vec3<f32> = floor(v + vec3<f32>(vCy, vCy, vCy));
    let iCx = dot(i, C.xxx);
    let x0 = v - i + vec3<f32>(iCx, iCx, iCx);

    // Other corners
    let g = step(x0.yzx, x0.xyz);
    let l = (vec3<f32>(1.0, 1.0, 1.0) - g).zxy;
    let i1 = min(g, l);
    let i2 = max(g, l);

    //   x0 = x0 - 0.0 + 0.0 * C.xxx;
    //   x1 = x0 - i1  + 1.0 * C.xxx;
    //   x2 = x0 - i2  + 2.0 * C.xxx;
    //   x3 = x0 - 1.0 + 3.0 * C.xxx;
    let x1 = x0 - i1 + C.xxx;
    let x2 = x0 - i2 + C.yyy; // 2.0*C.x = 1/3 = C.y
    let x3 = x0 - D.yyy; // -1.0+3.0*C.x = -0.5 = -D.y

    // Permutations
    var temp: vec3<f32> = 289.0 * one.xyz;
    i = modf(i, &temp);
    let p = permute(
        permute(
            permute(i.zzzz + vec4<f32>(0.0, i1.z, i2.z, 1.0))
            + i.yyyy + vec4<f32>(0.0, i1.y, i2.y, 1.0))
        + i.xxxx + vec4<f32>(0.0, i1.x, i2.x, 1.0));

    // Gradients: 7x7 points over a square, mapped onto an octahedron.
    // The ring size 17*17 = 289 is close to a multiple of 49 (49*6 = 294)
    let n_ = 0.142857142857;// 1.0/7.0
    let ns = n_ * D.wyz - D.xzx;

    let j = p - 49.0 * floor(p * ns.z * ns.z);//  mod(p,7*7)

    let x_ = floor(j * ns.z);
    let y_ = floor(j - 7.0 * x_);// mod(j,N)

    var x: vec4<f32> = x_ *ns.x + ns.yyyy;
    var y: vec4<f32> = y_ *ns.x + ns.yyyy;
    let h = one - abs(x) - abs(y);

    let b0 = vec4<f32>(x.xy, y.xy);
    let b1 = vec4<f32>(x.zw, y.zw);

    //vec4 s0 = vec4(lessThan(b0,0.0))*2.0 - one;
    //vec4 s1 = vec4(lessThan(b1,0.0))*2.0 - one;
    let s0 = floor(b0)*2.0 + one;
    let s1 = floor(b1)*2.0 + one;
    let sh = -step(h, 0.0 * one);

    let a0 = b0.xzyw + s0.xzyw*sh.xxyy;
    let a1 = b1.xzyw + s1.xzyw*sh.zzww;

    var p0: vec3<f32> = vec3<f32>(a0.xy, h.x);
    var p1: vec3<f32> = vec3<f32>(a0.zw, h.y);
    var p2: vec3<f32> = vec3<f32>(a1.xy, h.z);
    var p3: vec3<f32> = vec3<f32>(a1.zw, h.w);

    //Normalise gradients
    let norm = taylorInvSqrt(vec4<f32>(dot(p0, p0), dot(p1, p1), dot(p2, p2), dot(p3, p3)));
    p0 = p0 * norm.x;
    p1 = p1 * norm.y;
    p2 = p2 * norm.z;
    p3 = p3 * norm.w;

    // Mix final noise value
    var m: vec4<f32> = max(0.6 * one - vec4<f32>(dot(x0, x0), dot(x1, x1), dot(x2, x2), dot(x3, x3)), 0.0 * one);
    m = m * m;
    return 9.0 * dot(m*m, vec4<f32>(dot(p0, x0), dot(p1, x1), dot(p2, x2), dot(p3, x3)));
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
