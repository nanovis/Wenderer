#version 450
layout(location=0) in vec2 vTexCoord;
layout(location=0) out vec4 f_color;

layout(set=0, binding = 0) uniform texture2D front_face_tex;// `set = 0` correspond to render_pass.set_bind_group(0, ..)
layout(set=0, binding = 1) uniform sampler front_face_sampler;
layout(set=0, binding = 2) uniform texture2D back_face_tex;
layout(set=0, binding = 3) uniform sampler back_face_sampler;
layout(set=1, binding = 0) uniform texture3D data_tex;
layout(set=1, binding = 1) uniform sampler data_sampler;
layout(set=2, binding = 0) uniform texture1D tf_tex;
layout(set=2, binding = 1) uniform sampler tf_sampler;

void main() {
    vec3 front_face_color = texture(sampler2D(front_face_tex, front_face_sampler), vTexCoord).rgb;
    vec3 back_face_color = texture(sampler2D(back_face_tex, back_face_sampler), vTexCoord).rgb;
    f_color = vec4(front_face_color - back_face_color, 1.0);
}
