use cgmath::prelude::*;
use wgpu::util::DeviceExt;
use crate::shader_settings::camera::{Camera, Projection};
use crate::shader_settings::light::Light;
use crate::shader_settings::texture;
// use crate::shader_settings::shadowmap;

// uniformの設定
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Uniforms {
    view_position: cgmath::Vector4<f32>,
    view_proj: cgmath::Matrix4<f32>,
    light_num: u32,
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

impl Uniforms {
    pub fn new(light_num: u32) -> Self {
        // use cgmath::SquareMatrix;
        Self {
            view_position: Zero::zero(),
            view_proj: cgmath::Matrix4::identity(),
            light_num,
        }
    }

    // 視点変更時に呼び出す必要がありそう
    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view_position = camera.position.to_homogeneous();
        self.view_proj = projection.calc_matrix() * camera.calc_matrix();
    }
}

pub struct UniformSetting {
    pub uniforms: Uniforms,
    pub buffer: wgpu::Buffer,
    pub layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl UniformSetting {
    pub fn new(
        device: &wgpu::Device,
        // light_num: u32,
        lights: &[&Light],
        light_buffer: &wgpu::Buffer,
        shadow_uniform_buffer: &wgpu::Buffer,
        shadow_texture: &texture::Texture,
        // shadow_texture: &texture::Texture,
        // shadowmap: &shadowmap::ShadowMap,
        // shadowmaps: &[&shadowmap::ShadowMap],
    ) -> Self {
        let light_num = lights.len() as u32;
        let uniforms = Uniforms::new(light_num);

        let buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Uniform Buffer"),
                contents: bytemuck::cast_slice(&[uniforms]),
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            }
        );

        let layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::UniformBuffer {
                            dynamic: false, // 動的配列を使うか否かみたいな意味
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            readonly: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            readonly: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            multisampled: false,
                            component_type: wgpu::TextureComponentType::Float,
                            dimension: wgpu::TextureViewDimension::D2Array,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            comparison: true,
                        },
                        count: None,
                    },
                ],
                label: Some("uniform_bind_group_layout"),
            }
        );

        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(buffer.slice(..))
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(light_buffer.slice(..))
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Buffer(shadow_uniform_buffer.slice(..)),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&shadow_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&shadow_texture.sampler),
                    },
                ],
                label: Some("uniform_bind_group"),
            }
        );

        Self {
            uniforms,
            buffer,
            layout,
            bind_group,
        }
    }
}