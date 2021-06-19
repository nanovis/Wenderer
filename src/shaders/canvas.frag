#version 450
layout(location=0) in vec2 texCoord;
layout(location=0) out vec4 fColor;

layout(set=0, binding = 0) uniform texture2D front_face_tex;// `set = 0` correspond to render_pass.set_bind_group(0, ..)
layout(set=0, binding = 1) uniform sampler front_face_sampler;
layout(set=0, binding = 2) uniform texture2D back_face_tex;
layout(set=0, binding = 3) uniform sampler back_face_sampler;
layout(set=1, binding = 0) uniform texture3D volumeData;
layout(set=1, binding = 1) uniform sampler volumeSampler;
layout(set=2, binding = 0) uniform texture1D tf_tex;
layout(set=2, binding = 1) uniform sampler tf_sampler;

layout(set=3, binding=0) uniform Uniforms{
    float StepSize;
    float BaseDistance;
    float OpacityThreshold;
    float ambient;
    float diffuse;
    float specular;
    float shininess;
};

float sampleVolume(vec3 position){
    return texture(sampler3D(volumeData, volumeSampler), position).r;
}

vec4 sampleTransferFunction(float scalar){
    return texture(sampler1D(tf_tex, tf_sampler), scalar);
}

void main() {
    vec3 I_ambient = vec3(ambient);
    vec3 I_diffuse = vec3(0.5);
    vec3 I_specular = vec3(0.5);
    float delta = StepSize / 2.0;
    vec3 startVolumeCoord= texture(sampler2D(front_face_tex, front_face_sampler), texCoord).rgb;
    vec3 endVolumeCoord = texture(sampler2D(back_face_tex, back_face_sampler), texCoord).rgb;
    vec3 rayDir = normalize(endVolumeCoord - startVolumeCoord);
    vec3 position = startVolumeCoord;
    vec4 compositeColor = vec4(0.0);
    int maxMarchingSteps = int(length(endVolumeCoord - startVolumeCoord) / StepSize);
    vec3 xDelta = vec3(delta, 0.0, 0.0);
    vec3 yDelta = vec3(0.0, delta, 0.0);
    vec3 zDelta = vec3(0.0, 0.0, delta);
    for (int i=0;i < maxMarchingSteps; i++){
        float scalar = sampleVolume(position);
        vec4 src = sampleTransferFunction(scalar);
        float opacity = src.a;
        opacity = 1.0 - pow(1.0- opacity, StepSize/ BaseDistance);// opacity correction
        vec4 newSrc = vec4(src.rgb* opacity, opacity);
        vec4 finalColor;
        // shading
        vec3 normal;
        normal.x = sampleVolume(position + xDelta) - sampleVolume(position - xDelta);
        normal.y = sampleVolume(position + yDelta) - sampleVolume(position - yDelta);
        normal.z = sampleVolume(position + zDelta) - sampleVolume(position - zDelta);
        normal = normalize(normal);
        float dirDotNorm = dot(rayDir, normal);
        vec3 specularColor = vec3(0.0);
        vec3 diffuseColor = vec3(0.0);
        if (dirDotNorm>0.0){
            diffuseColor = dirDotNorm * I_diffuse;
            vec3 v = normalize(-position);
            vec3 r = reflect(-rayDir, normal);
            float R_dot_V= max(dot(r, v), 0.0);
            float pf = (R_dot_V == 0.0)? 0.0: pow(R_dot_V, shininess);
            specularColor = I_specular * pf;
        }
        finalColor = vec4(I_ambient + diffuseColor + specularColor, 1.0) * newSrc;
        compositeColor = (1.0 - compositeColor.a) * finalColor + compositeColor;// front-to-back compositing
        if (compositeColor.a > OpacityThreshold)// early ray termination
        break;
        position += rayDir * StepSize;
    }
    fColor = compositeColor;
}
