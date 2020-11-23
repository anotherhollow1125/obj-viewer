#version 450

layout(location = 0) in vec3 a_position;
layout(location = 1) in vec2 a_tex_coords;
layout(location = 2) in vec3 a_normal;

layout(location = 0) out vec2 v_tex_coords;
layout(location = 1) out vec3 v_normal;
layout(location = 2) out vec3 v_position;

layout(set = 1, binding = 0)
uniform Uniforms {
    vec3 u_view_position; // unused
    mat4 u_view_proj;
    int u_light_num; // unused
};

struct Instance {
    mat4 transform;
    mat4 transform_norm;
};

layout(set = 2, binding = 0)
buffer Instances {
    Instance instances[];
};

void main() {
    v_tex_coords = a_tex_coords;

    mat4 instance_matrix = instances[gl_InstanceIndex].transform;
    mat4 insnorm_matrix = instances[gl_InstanceIndex].transform_norm;
    // mat3 normal_matrix = mat3(transpose(inverse(instance_matrix)));
    mat3 normal_matrix = mat3(insnorm_matrix);
    v_normal = normal_matrix * a_normal;

    vec4 instance_space = instance_matrix * vec4(a_position, 1.0);
    v_position = instance_space.xyz;
    gl_Position = u_view_proj * instance_matrix * vec4(a_position, 1.0);
}