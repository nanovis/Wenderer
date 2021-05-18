#version 450

layout(location=0) in vec3 v_pos;
layout(location=1) in vec2 v_tex_coord;

layout(location=0) out vec2 vTexCoord;

layout(set=1, binding=0)
uniform Uniforms{
    mat4 u_view_proj;
};

void main() {
    vTexCoord = v_tex_coord;
    gl_Position = u_view_proj * vec4(v_pos, 1.0);
}
