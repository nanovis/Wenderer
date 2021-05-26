#version 450
layout(location=0) in vec3 v_pos;
layout(location=1) in vec2 v_tex_coord;

layout(location=0) out vec2 vTexCoord;

void main() {
    vTexCoord = v_tex_coord;
    gl_Position = vec4(v_pos, 1.0);
}
