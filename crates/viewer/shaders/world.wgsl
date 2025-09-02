struct VertexOutput {
    @location(0) texcoord: vec2<f32>,
    @location(1) lightmap_texcoord: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) color: vec4<f32>,
    @location(4) face_index: u32,
    @builtin(position) position: vec4<f32>,
};

struct PushConstants {
    view: mat4x4<f32>
}
var<push_constant> pc: PushConstants;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) texcoord: vec2<f32>,
    @location(3) lightmap_texcoord: vec2<f32>,
    @location(4) color: vec4<f32>,
    @location(5) face_index: u32,
) -> VertexOutput {
    var result: VertexOutput;
    result.texcoord = texcoord;
    result.lightmap_texcoord = lightmap_texcoord;
    result.normal = normal * 0.5 + 0.5;
    result.color = color;
    result.face_index = face_index;
    result.position = pc.view * vec4<f32>(position, 1.0);
    return result;
}

struct MapFace {
    lightmap_size: u32,
    lightmap_offset: i32,
    flags: u32,
    texture_index: i32,
}

@group(0)
@binding(0)
var<storage, read> r_lightmap: array<u32>;

@group(0)
@binding(1)
var<storage, read> r_faces: array<MapFace>;

@group(1)
@binding(0)
var r_texture_array: binding_array<texture_2d<f32>>;

@group(1)
@binding(1)
var r_sampler_linear: sampler;

// @group(1)
// @binding(0)
// var r_texture: texture_2d<f32>;
// @group(1)
// @binding(1)
// var r_texture_sampler: sampler;

fn load_lightmap_texel(offset: u32) -> vec3<f32> {
    let v = r_lightmap[offset];
    let r = (v >> 24) & 0xFF;
    let g = (v >> 16) & 0xFF;
    let b = (v >> 8) & 0xFF;
    let exponent_packed: u32 = v & 0xFF;
    let exponent = i32(exponent_packed) - 127;
    let color = vec3<f32>(f32(r), f32(g), f32(b)) / 255.0;

    return color * pow(2.0, f32(exponent));
}

fn sample_lightmap(texcoord: vec2<f32>, face: MapFace) -> vec3<f32> {
    let lightmap_size = vec2<u32>(
        face.lightmap_size >> 16,
        face.lightmap_size & 0xFFFF
    );

    let texcoord_scaled = vec2(
        clamp(texcoord.x, 0.0, f32(lightmap_size.x)),
        clamp(texcoord.y, 0.0, f32(lightmap_size.y))
    );
    let offset = u32(u32(lightmap_size.x) * u32(texcoord_scaled.y) + u32(texcoord_scaled.x));
    return load_lightmap_texel(u32(face.lightmap_offset) + offset);
}

fn sample_lightmap_bilinear(texcoord: vec2<f32>, face: MapFace) -> vec3<f32> {
    let tl = sample_lightmap(texcoord, face);
    let tr = sample_lightmap(texcoord + vec2(1.0, 0.0), face);
    let br = sample_lightmap(texcoord + vec2(1.0, 1.0), face);
    let bl = sample_lightmap(texcoord + vec2(0.0, 1.0), face);

    let lerp_x = texcoord.x - floor(texcoord.x);
    let lerp_y = texcoord.y - floor(texcoord.y);

    let top = mix(tl, tr, lerp_x);
    let bottom = mix(bl, br, lerp_x);
    return mix(top, bottom, lerp_y);
}

const FACE_IS_DISPLACEMENT: u32 = 1 << 0;
const FACE_IS_SKY2D: u32 = 1 << 1;
const FACE_IS_SKY3D: u32 = 1 << 2;
const FACE_IS_SKY: u32 = FACE_IS_SKY2D | FACE_IS_SKY3D;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let face = r_faces[vertex.face_index];
    if((face.flags & FACE_IS_SKY3D) != 0) {
        discard;
    }
    if((face.flags & FACE_IS_SKY2D) != 0) {
        return vec4(0.7f, 0.88f, 0.99f, 1f);
    }

    var light = vec3<f32>(1.0, 1.0, 1.0);
    if (face.lightmap_offset >= 0) {
        light = sample_lightmap_bilinear(vertex.lightmap_texcoord, face);
    }

    var sample = textureSampleLevel(
        r_texture_array[face.texture_index],
        r_sampler_linear,
        vertex.texcoord,
        0.0
    );

    return vec4(light * sample.xyz, 1.0);
}
