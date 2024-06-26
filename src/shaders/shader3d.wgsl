struct Uniforms{
    view_proj_mat: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput{
    @location(0) v_pos: vec3<f32>,
    @location(1) v_coord: vec3<f32>,
}

struct VertexOutput{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) v_coord: vec3<f32>,
};

// simple vertex shader for drawing a box
@vertex
fn vertex_shader(vertex: VertexInput) -> VertexOutput{
    var out: VertexOutput;
    out.v_coord = vertex.v_coord;
    out.clip_position = uniforms.view_proj_mat * vec4<f32>(vertex.v_pos, 1.0);
    return out;
}

// draw fragment position in world space to buffer
@fragment
fn fragment_shader(in: VertexOutput) -> @location(0) vec4<f32>{
    return vec4<f32>(in.v_coord, 1.0);
}