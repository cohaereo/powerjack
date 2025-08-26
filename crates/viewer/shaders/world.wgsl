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
    lightmap_size: vec2<i32>,
    lightmap_offset: i32,
    padding: u32,
}

@group(0)
@binding(0)
var<storage, read> r_lightmap: array<u32>;
@group(0)
@binding(1)
var<storage, read> r_faces: array<MapFace>;

// @group(1)
// @binding(0)
// var r_texture: texture_2d<f32>;
// @group(1)
// @binding(1)
// var r_texture_sampler: sampler;

fn sample_lightmap_texel(offset: u32) -> vec3<f32> {
    let v = r_lightmap[offset];
    let r = (v >> 24) & 0xFF;
    let g = (v >> 16) & 0xFF;
    let b = (v >> 8) & 0xFF;
    let exponent_packed: u32 = v & 0xFF;
    let exponent = i32(exponent_packed << 24) >> 24;
    let color = vec3<f32>(f32(r), f32(g), f32(b)) / 255.0;

    return color * pow(2.0, f32(exponent));
    // return color;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let face = r_faces[vertex.face_index];
    var light = vec3<f32>(1.0, 1.0, 1.0);
    if (face.lightmap_offset >= 0) {
        light = vec3<f32>(0.0, 0.0, 0.0);
        for(var i: i32 = 0; i < face.lightmap_size.x; i++) {
            for(var j: i32 = 0; j < face.lightmap_size.y; j++) {
                let offset = face.lightmap_offset + i * face.lightmap_size.y + j;
                light += sample_lightmap_texel(u32(offset));
            }
        }
        light /= f32(face.lightmap_size.x * face.lightmap_size.y);
    }

    // return vec4(vertex.lightmap_texcoord.xy, 0.0, 1.0);
    var lightDir = normalize(vec3<f32>(0.5, 0.5, 0.0));
    var diff = max(dot(vertex.normal, lightDir), 0.0);
    return vec4(light * diff * vertex.color.xyz, 1.0);
    // return vec4(diff, diff, diff, 1.0) * vertex.color;
}
