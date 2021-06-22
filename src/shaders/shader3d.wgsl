[[block]]
struct Uniforms{
    view_proj_mat: mat4x4<f32>;
};

[[group(0), binding(0)]]
var<uniform> uniforms: Uniforms;

struct VertexInput{
    [[location(0)]] v_pos: vec3<f32>;
    [[location(1)]] v_coord: vec3<f32>;
};

struct VertexOutput{
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] v_coord: vec3<f32>;
};

// this vertex_shader is equivalent to ./deprecated_glsl_shaders/shader.vert
[[stage(vertex)]]
fn vertex_shader(vertex: VertexInput) -> VertexOutput{
    var out: VertexOutput;
    out.v_coord = vertex.v_coord;
    out.clip_position = uniforms.view_proj_mat * vec4<f32>(vertex.v_pos, 1.0);
    return out;
}

// this fragment_shader is equivalent to ./deprecated_glsl_shaders/shader.frag
[[stage(fragment)]]
fn fragment_shader(in: VertexOutput) -> [[location(0)]] vec4<f32>{
    return vec4<f32>(in.v_coord, 1.0);
}