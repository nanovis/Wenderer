#version 450

layout(location=0) in vec3 v_pos;
layout(location=1) in vec3 v_color;

layout(location=0) out vec3 vColor;

void main() {
    vColor = v_color;
    gl_Position = vec4(v_pos, 1.0);
}
