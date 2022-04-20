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


fn hash(p: vec2<f32>) -> vec2<f32> // replace this by something better
{
    let p2 = vec2<f32>( dot(p,vec2<f32>(127.1,311.7)), dot(p,vec2<f32>(269.5,183.3)) );
    return -1.0 + 2.0*fract(sin(p2)*43758.5453123);
}

fn simplex2d(p: vec2<f32>) -> f32
{
    let K1 = 0.366025404; // (sqrt(3)-1)/2;
    let K2 = 0.211324865; // (3-sqrt(3))/6;
    let i = floor( p + (p.x+p.y)*K1 );
    let a = p - i + (i.x+i.y)*K2;
    let o = step(a.yx,a.xy);
    let b = a - o + K2;
    let c = a - 1.0 + 2.0*K2;
    let h = max( 0.5-vec3<f32>(dot(a,a), dot(b,b), dot(c,c) ), vec3<f32>(0.) );
    let n = h*h*h*h*vec3<f32>( dot(a,hash(i+0.0)), dot(b,hash(i+o)), dot(c,hash(i+1.0)));
    return dot( n, vec3<f32>(70.0) );
}


// Simplex Noise 3D: https://www.shadertoy.com/view/XsX3zB

fn random3(c: vec3<f32>) -> vec3<f32>
{
    var j = 4096.0*sin(dot(c,vec3<f32>(17.0, 59.4, 15.0)));
    var r = vec3<f32>(0.);
    r.z = fract(512.0*j);
    j = j * .125;
    r.x = fract(512.0*j);
    j = j * .125;
    r.y = fract(512.0*j);
    return r - 0.5;
}

let F3 = 0.3333333;
let G3 = 0.1666667;

fn simplex3d(p: vec3<f32>) -> f32
{
    // 1. find current tetrahedron T and it's four vertices */

    // calculate s and x */
    let s = floor(p + dot(p, vec3<f32>(F3)));
    let x = p - s + dot(s, vec3<f32>(G3));

    // calculate i1 and i2 */
    let e = step(vec3<f32>(0.0), x - x.yzx);
    let i1 = e*(1.0 - e.zxy);
    let i2 = 1.0 - e.zxy*(1.0 - e);

    // x1, x2, x3 */
    let x1 = x - i1 + G3;
    let x2 = x - i2 + 2.0*G3;
    let x3 = x - 1.0 + 3.0*G3;

    // 2. find four surflets and store them in d */
    var w = vec4<f32>(0.);
    var d = vec4<f32>(0.);

    // calculate surflet weights */
    w.x = dot(x, x);
    w.y = dot(x1, x1);
    w.z = dot(x2, x2);
    w.w = dot(x3, x3);

    // w fades from 0.6 at the center of the surflet to 0.0 at the margin */
    w = max(0.6 - w, vec4<f32>(0.0));

    // calculate surflet components */
    d.x = dot(random3(s), x);
    d.y = dot(random3(s + i1), x1);
    d.z = dot(random3(s + i2), x2);
    d.w = dot(random3(s + 1.0), x3);

    // multiply d by w^4 */
    w = w * w;
    w = w * w;
    d = d * w;

    // 3. return the sum of the four surflets */
    return dot(d, vec4<f32>(52.0));
}

// const matrices for 3d rotation */
let rot1 = mat3x3<f32>(vec3<f32>(-0.37, 0.36, 0.85), vec3<f32>(-0.14,-0.93, 0.34), vec3<f32>(0.92, 0.01,0.4));
let rot2 = mat3x3<f32>(vec3<f32>(-0.55,-0.39, 0.74), vec3<f32>( 0.33,-0.91,-0.24), vec3<f32>(0.77, 0.12,0.63));
let rot3 = mat3x3<f32>(vec3<f32>(-0.71, 0.52,-0.47), vec3<f32>(-0.08,-0.72,-0.68), vec3<f32>(-0.7,-0.45,0.56));

// directional artifacts can be reduced by rotating each octave */
fn simplex3d_fractal(m: vec3<f32>) -> f32
{
    return   0.5333333 * simplex3d(m * rot1)
            +0.2666667*simplex3d(2.0 * m * rot2)
            +0.1333333*simplex3d(4.0 * m * rot3)
            +0.0666667*simplex3d(8.0 * m );
}

[[stage(compute), workgroup_size(8, 8, 8)]]
fn main([[builtin(global_invocation_id)]] id: vec3<u32>) {
    let pos: vec3<i32> = vec3<i32>(pos.pos.xyz) + vec3<i32>(id);
    
    var amp = 140.0;
    var freq = 0.003;

    var density: f32 = f32(-pos.y);
    // density = 50.0 - distance(vec3<f32>(pos), vec3<f32>(0.0, -50.0, 0.0));

    for (var i = 0; i < 10; i = i+1) {
        density = density + simplex3d(vec3<f32>(pos) * freq) * amp;
        amp = amp * 0.5;
        freq = freq * 2.0;
    }    

    values.data[to_index(id)] = density;
}
