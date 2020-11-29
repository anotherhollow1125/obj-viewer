#version 450

layout(location = 0) in vec2 v_tex_coords;
layout(location = 1) in vec3 v_normal;
layout(location = 2) in vec4 v_position;
// layout(location = 3) in vec4 s_gl_position;

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

layout(set = 1, binding = 1)
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

layout(set = 1, binding = 2)
uniform ShadowUniforms {
    mat4 shadow_view_proj;
};
layout(set = 1, binding = 3) uniform texture2D t_shadow;
layout(set = 1, binding = 4) uniform samplerShadow s_shadow;

float fetch_shadow(vec4 homogeneous_coords) {
    if (homogeneous_coords.w <= 0.0) {
        return 1.0;
    }
    // compensate for the Y-flip difference between the NDC and texture coordinates
    const vec2 flip_correction = vec2(0.5, -0.5);
    // compute texture coordinates for shadow lookup
    vec3 light_local = vec3(
        homogeneous_coords.xy * flip_correction/homogeneous_coords.w + 0.5,
        homogeneous_coords.z / homogeneous_coords.w
    );
    // do the lookup, using HW PCF and comparison
    return texture(sampler2DShadow(t_shadow, s_shadow), light_local);
}

void main() {
    vec4 object_color;
    if (use_texture == 1) {
        object_color = texture(sampler2D(t_diffuse, s_diffuse), v_tex_coords);
    } else {
        object_color = vec4(u_diffuse, 1.0);
    }

    vec3 result = vec3(0.0, 0.0, 0.0);

    float light_hit = 0.0;

    for (int i = 0; i < u_light_num; i++) {
        vec3 l_position = lights[i].position;
        vec3 l_color = lights[i].color;
        uint l_is_spotlight = lights[i].is_spotlight;
        float l_intensity = lights[i].intensity;
        float l_radius = lights[i].radius;

        vec3 surface_to_light = normalize(l_position - v_position.xyz);
        float spot_target_check = dot(surface_to_light, -lights[i].limitdir);
        float in_light = max(1 - l_is_spotlight, smoothstep(
            lights[i].limitcos_outer,
            lights[i].limitcos_inner,
            spot_target_check
        ));

        l_radius = max(l_radius, 0.000001);
        vec3 ambient_color = l_color * l_radius / max(l_radius, distance(l_position, v_position.xyz));
        ambient_color *= in_light;

        vec3 normal = normalize(v_normal);
        vec3 light_dir = normalize(l_position - v_position.xyz);

        float diffuse_strength = max(dot(normal, light_dir), 0.0);
        vec3 diffuse_color = diffuse_strength * in_light * l_color;

        vec3 view_dir = normalize(u_view_position - v_position.xyz);
        vec3 half_dir = normalize(view_dir + light_dir);

        float specular_strength = pow(max(dot(normal, half_dir), 0.0), 32);
        vec3 specular_color = specular_strength * in_light * l_color;

        result += l_intensity * (ambient_color + diffuse_color + specular_color) * object_color.xyz;

        light_hit = max(in_light, light_hit) * i;
    }

    /*
    vec2 shadow_texcoord;
    vec4 p = shadow_view_proj * v_position;
    shadow_texcoord.x = (1.0 + p.x/p.w) * 0.5;
    shadow_texcoord.y = (1.0 - p.y/p.w) * 0.5;
    float shadow_z = texture(sampler2D(t_shadow, s_shadow), shadow_texcoord).x;
    float z_val = p.z/p.w;

    if (z_val > shadow_z + 0.005) {
        result.rgb = result.rgb * 0.5;
    }
    */
    result.rgb *= max(light_hit, 0.7 * fetch_shadow(shadow_view_proj * v_position));

    f_color = vec4(result, object_color.a);
}