#version 450

layout(location = 0) in vec3 a_position;
// layout(location = 1) in vec2 a_tex_coords;
// layout(location = 2) in vec3 a_normal;

layout(set = 0, binding = 0)
uniform Uniforms {
    mat4 u_view_proj;
};

struct Instance {
    mat4 transform;
    mat4 transform_norm;
};

layout(set = 1, binding = 0)
buffer Instances {
    Instance instances[];
};

void main() {
    mat4 instance_matrix = instances[gl_InstanceIndex].transform;
    vec4 instance_space = instance_matrix * vec4(a_position, 1.0);
    gl_Position = u_view_proj * instance_space;
}