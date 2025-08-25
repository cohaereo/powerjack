struct VertexOutput {
    @location(0) texcoord: vec2<f32>,
    @location(1) lightmap_texcoord: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) color: vec4<f32>,
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
) -> VertexOutput {
    var result: VertexOutput;
    result.texcoord = texcoord;
    result.lightmap_texcoord = lightmap_texcoord;
    result.normal = normal * 0.5 + 0.5;
    result.color = color;
    result.position = pc.view * vec4<f32>(position, 1.0);
    return result;
}

// @group(0)
// @binding(0)
// var r_lightmap: texture_2d<f32>;
// @group(0)
// @binding(1)
// var r_lightmap_sampler: sampler;

// @group(1)
// @binding(0)
// var r_texture: texture_2d<f32>;
// @group(1)
// @binding(1)
// var r_texture_sampler: sampler;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    var lightDir = normalize(vec3<f32>(0.5, 0.5, 0.0));
    var diff = max(dot(vertex.normal, lightDir), 0.0);
    return vec4(diff, diff, diff, 1.0) * vertex.color;
}
