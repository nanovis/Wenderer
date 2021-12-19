struct VertexInput{
    [[location(0)]] pos: vec3<f32>;
    [[location(1)]] tex_coord: vec2<f32>;
};

struct VertexOutput{
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] tex_coord: vec2<f32>;
};

// this vertex_shader is equivalent to ./deprecated_glsl_shaders/canvas.vert
[[stage(vertex)]]
fn vertex_shader(vertex: VertexInput) -> VertexOutput{
    var out: VertexOutput;
    out.tex_coord = vertex.tex_coord;
    out.clip_position = vec4<f32>(vertex.pos, 1.0);
    return out;
}

struct FragmentUniforms{
    step_size: f32;
    base_distance: f32;
    opacity_threshold: f32;
    ambient: f32;
    diffuse: f32;
    specular: f32;
    shininess: f32;
};

[[group(0), binding(0)]] var front_face_tex: texture_2d<f32>;
[[group(0), binding(1)]] var front_face_sampler: sampler;
[[group(0), binding(2)]] var back_face_tex: texture_2d<f32>;
[[group(0), binding(3)]] var back_face_sampler: sampler;

[[group(1), binding(0)]] var volume_data: texture_3d<f32>;
[[group(1), binding(1)]] var volume_sampler: sampler;

[[group(2), binding(0)]] var tf_tex: texture_1d<f32>;
[[group(2), binding(1)]] var tf_sampler: sampler;

[[group(3),binding(0)]] var<uniform> uniforms: FragmentUniforms;

fn sample_volume(position: vec3<f32>) -> f32{
    return textureSample(volume_data, volume_sampler, position).r;
}

fn sample_tf(scalar: f32) -> vec4<f32>{
    return textureSample(tf_tex, tf_sampler, scalar);
}

// this vertex_shader is equivalent to ./deprecated_glsl_shaders/canvas.frag
[[stage(fragment)]]
fn fragment_shader(in : VertexOutput) -> [[location(0)]] vec4<f32>{
    let I_ambient = vec3<f32>(uniforms.ambient);
    let I_diffuse = vec3<f32>(uniforms.diffuse);
    let I_specular = vec3<f32>(uniforms.specular);
    let delta = uniforms.step_size / 2.0;
    let start_volume_coord = textureSample(front_face_tex, front_face_sampler, in.tex_coord).rgb;
    let end_volume_coord = textureSample(back_face_tex, back_face_sampler, in.tex_coord).rgb;
    let ray_dir = normalize(end_volume_coord - start_volume_coord);
    var position:vec3<f32> = start_volume_coord;
    var composite_color:vec4<f32> = vec4<f32>(0.0);
    let max_marching_step = i32(length(end_volume_coord - start_volume_coord)/uniforms.step_size);
    let x_delta = vec3<f32>(delta, 0.0, 0.0);
    let y_delta = vec3<f32>(0.0, delta, 0.0);
    let z_delta = vec3<f32>(0.0, 0.0, delta);
    for(var i:i32 = 0; i<max_marching_step; i = i+1){
        let scalar = sample_volume(position);
        let src = sample_tf(scalar);
        let opacity = 1.0 - pow(1.0 - src.a, uniforms.step_size / uniforms.base_distance);
        let new_src = vec4<f32>(src.rgb*opacity, opacity);
        var normal : vec3<f32>;
        normal.x = sample_volume(position + x_delta) - sample_volume(position - x_delta);
        normal.y = sample_volume(position + y_delta) - sample_volume(position - y_delta);
        normal.z = sample_volume(position + z_delta) - sample_volume(position - z_delta);
        normal = normalize(normal);
        let dir_dot_norm = dot(ray_dir, normal);
        var specular_color : vec3<f32> = vec3<f32>(0.0);
        var diffuse_color : vec3<f32> = vec3<f32>(0.0);
        if(dir_dot_norm > 0.0){
            diffuse_color = dir_dot_norm * I_diffuse;
            let v = normalize(-position);
            let r = reflect(-ray_dir, normal);
            let r_dot_v = max(dot(r, v), 0.0);
            let pf = pow(r_dot_v, uniforms.shininess);
            specular_color = I_specular * pf;
        }
        let final_color = vec4<f32>(I_ambient + diffuse_color + specular_color, 1.0)* new_src;
        composite_color = (1.0 - composite_color.a) * final_color + composite_color;
        if (composite_color.a > uniforms.opacity_threshold){
            break;
        }
        position = position + ray_dir * uniforms.step_size;
    }
    return composite_color;
}