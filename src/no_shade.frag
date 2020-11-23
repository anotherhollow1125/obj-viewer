#version 450

layout(location = 0) in vec2 v_tex_coords;
layout(location = 0) out vec4 f_color;

layout(set = 1, binding = 0)
uniform Uniforms {
    vec3 u_view_position;
    mat4 u_view_proj; // unused
    int u_light_num;
};

layout(set = 0, binding = 0) uniform texture2D t_diffuse;
layout(set = 0, binding = 1) uniform sampler s_diffuse;
layout(set = 0, binding = 2)
uniform MaterialUniform {
    int use_texture;
    vec3 u_ambient;
    vec3 u_diffuse;
    vec3 u_specular;
};

void main() {
    if (use_texture == 1) {
        f_color = texture(sampler2D(t_diffuse, s_diffuse), v_tex_coords);
    } else {
        f_color = vec4(u_diffuse, 1.0);
    }
}