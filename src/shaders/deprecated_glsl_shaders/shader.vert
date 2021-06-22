#version 450

layout(location=0) in vec3 v_pos;
layout(location=1) in vec3 v_coord;

layout(location=0) out vec3 vCoord;

layout(set=0, binding=0)
uniform Uniforms{
    mat4 u_view_proj;
};

void main() {
    vCoord = v_coord;
    gl_Position = u_view_proj * vec4(v_pos, 1.0);
}