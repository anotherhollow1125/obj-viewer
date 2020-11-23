#version 450

layout(location = 0) in vec2 v_tex_coords;
layout(location = 1) in vec3 v_normal;
layout(location = 2) in vec3 v_position;

layout(location = 0) out vec4 f_color;

layout(set = 1, binding = 0)
uniform Uniforms {
    vec3 u_view_position;
    mat4 u_view_proj; // unused
    int u_light_num;
};

struct Light {
    vec3 position;
    vec3 color;
};

layout(set = 3, binding = 0)
buffer Lights {
    Light lights[];
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
    vec4 object_color;
    if (use_texture == 1) {
        object_color = texture(sampler2D(t_diffuse, s_diffuse), v_tex_coords);
    } else {
        object_color = vec4(u_diffuse, 1.0);
    }

    vec3 result = vec3(0.0, 0.0, 0.0);

    for (int i = 0; i < u_light_num; i++) {
        vec3 l_position = lights[i].position;
        vec3 l_color = lights[i].color;

        float ambient_strength = 0.1;
        vec3 ambient_color = l_color * ambient_strength;

        vec3 normal = normalize(v_normal);
        vec3 light_dir = normalize(l_position - v_position);

        float diffuse_strength = max(dot(normal, light_dir), 0.0);
        vec3 diffuse_color = l_color * diffuse_strength;

        vec3 view_dir = normalize(u_view_position - v_position);
        vec3 half_dir = normalize(view_dir + light_dir);

        float specular_strength = pow(max(dot(normal, half_dir), 0.0), 32);
        vec3 specular_color = specular_strength * l_color;

        result += (ambient_color + diffuse_color + specular_color) * object_color.xyz;
    }

    f_color = vec4(result, object_color.a);
}