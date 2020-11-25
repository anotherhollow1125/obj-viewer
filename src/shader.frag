#version 450

layout(location = 0) in vec2 v_tex_coords;
layout(location = 1) in vec3 v_normal;
layout(location = 2) in vec3 v_position;

layout(location = 0) out vec4 f_color;

layout(set = 1, binding = 0)
uniform Uniforms {
    vec3 u_view_position;
    mat4 u_view_proj; // unused
    uint u_light_num;
};

struct Light {
    vec3 position;
    vec3 color;
    float intensity;
    float radius;
    uint is_spotlight;
    float limitcos_inner;
    float limitcos_outer;
    vec3 limitdir;
};

layout(set = 3, binding = 0)
buffer Lights {
    Light lights[];
};

layout(set = 0, binding = 0) uniform texture2D t_diffuse;
layout(set = 0, binding = 1) uniform sampler s_diffuse;
layout(set = 0, binding = 2)
uniform MaterialUniform {
    uint use_texture;
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
        uint l_is_spotlight = lights[i].is_spotlight;
        float l_intensity = lights[i].intensity;
        float l_radius = lights[i].radius;

        vec3 surface_to_light = normalize(l_position - v_position);
        float spot_target_check = dot(surface_to_light, -lights[i].limitdir);
        float in_light = max(1 - l_is_spotlight, smoothstep(
            lights[i].limitcos_outer,
            lights[i].limitcos_inner,
            spot_target_check
        ));

        l_radius = max(l_radius, 0.000001);
        vec3 ambient_color = l_color * l_radius / max(l_radius, distance(l_position, v_position));
        ambient_color *= in_light;

        vec3 normal = normalize(v_normal);
        vec3 light_dir = normalize(l_position - v_position);

        float diffuse_strength = max(dot(normal, light_dir), 0.0);
        vec3 diffuse_color = diffuse_strength * in_light * l_color;

        vec3 view_dir = normalize(u_view_position - v_position);
        vec3 half_dir = normalize(view_dir + light_dir);

        float specular_strength = pow(max(dot(normal, half_dir), 0.0), 32);
        vec3 specular_color = specular_strength * in_light * l_color;

        result += l_intensity * (ambient_color + diffuse_color + specular_color) * object_color.xyz;
    }

    f_color = vec4(result, object_color.a);
}