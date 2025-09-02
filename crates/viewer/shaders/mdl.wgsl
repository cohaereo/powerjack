struct VertexOutput {
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

struct PushConstants {
    view: mat4x4<f32>,
    model: mat4x4<f32>,
}
var<push_constant> pc: PushConstants;

fn mat4_to_mat3(mat: mat4x4<f32>) -> mat3x3<f32> {
    return mat3x3<f32>(
        mat[0].xyz,
        mat[1].xyz,
        mat[2].xyz
    );
}

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
) -> VertexOutput {
    var result: VertexOutput;
    result.normal = mat4_to_mat3(pc.model) * normal;
    result.uv = uv;
    result.position = pc.view * pc.model * vec4<f32>(position, 1.0);
    return result;
}

@group(0)
@binding(0)
var r_texture: texture_2d<f32>;

@group(0)
@binding(1)
var r_sampler_linear: sampler;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.5, -0.5, -1.0));
    let diffuse = max(dot(vertex.normal * 0.5 + 0.5, -light_dir), 0.02);

    let sample = textureSample(r_texture, r_sampler_linear, vertex.uv);

    // return vec4<f32>(fract(vertex.uv.x) * diffuse, fract(vertex.uv.y) * diffuse, 0.0, 1.0);
    return vec4<f32>(sample.xyz * diffuse, 1.0);
}
