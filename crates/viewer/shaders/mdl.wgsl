struct VertexOutput {
    @location(0) normal: vec3<f32>,
    @builtin(position) position: vec4<f32>,
};

struct PushConstants {
    view: mat4x4<f32>,
    model: mat4x4<f32>,
}
var<push_constant> pc: PushConstants;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
) -> VertexOutput {
    var result: VertexOutput;
    result.normal = normal;
    result.position = pc.view * pc.model * vec4<f32>(position, 1.0);
    return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(-1.0, -1.0, -1.0));
    let diffuse = max(dot(vertex.normal, light_dir), 0.0);
    return vec4<f32>(diffuse, diffuse, diffuse, 1.0);
}
