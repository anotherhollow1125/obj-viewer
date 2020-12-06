use wgpu::util::DeviceExt;
use crate::shader_settings::shadowmap;

// 光源の設定
pub struct Light {
    pub id: usize,
    pub position: cgmath::Vector3<f32>,
    pub color: cgmath::Vector3<f32>,
    pub intensity: f32,
    pub radius: f32,
    pub is_spotlight: bool,
    pub limitcos_inner: f32,
    pub limitcos_outer: f32,
    pub limitdir: cgmath::Vector3<f32>,
    pub shadow: shadowmap::ShadowMap,
}

impl Light {
    pub fn new(
        id: usize,
        position: cgmath::Vector3<f32>,
        color: cgmath::Vector3<f32>,
        intensity: f32,
        radius: f32,
        shadow: shadowmap::ShadowMap,
    ) -> Self {
        Self {
            id, position, color,
            intensity,
            radius,
            is_spotlight: false,
            limitcos_inner: 0.9,
            limitcos_outer: 0.1,
            limitdir: (0.0, 0.0, 0.0).into(),
            shadow,
        }
    }

    pub fn new_spotlight(
        id: usize,
        position: cgmath::Vector3<f32>,
        color: cgmath::Vector3<f32>,
        intensity: f32,
        radius: f32,
        limitcos_inner: f32,
        limitcos_outer: f32,
        limitdir: cgmath::Vector3<f32>,
        shadow: shadowmap::ShadowMap,
    ) -> Self {
        Self {
            id, position, color,
            intensity,
            radius,
            is_spotlight: true,
            limitcos_inner,
            limitcos_outer,
            limitdir,
            shadow,
        }
    }

    pub fn to_raw(&self) -> LightRaw {
        LightRaw {
            position: self.position,
            _p1: 0,
            color: self.color,
            intensity: self.intensity,
            radius: self.radius,
            is_spotlight: if self.is_spotlight { 1 } else { 0 },
            limitcos_inner: self.limitcos_inner,
            limitcos_outer: self.limitcos_outer,
            limitdir: self.limitdir,
            _p2: 0,
        }
    }
}

use std::cmp::{PartialEq, Eq, Ordering, Ord, PartialOrd};
use std::hash::{Hash, Hasher};

impl PartialEq for Light {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Light {}

impl Hash for Light {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Ord for Light {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for Light {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct LightRaw {
    position: cgmath::Vector3<f32>,
    _p1: u32, // 16 byte dummy data ... u32 ...? (size_of(u32) == 4 byte)
    // => I got it. 16 byte == size_of(Vec4<f32>) and it is 1 set. so this u32 is important.
    color: cgmath::Vector3<f32>,
    intensity: f32,
    radius: f32,
    is_spotlight: u32,
    limitcos_inner: f32,
    limitcos_outer: f32,
    limitdir: cgmath::Vector3<f32>,
    _p2: u32,
}

unsafe impl bytemuck::Zeroable for LightRaw {}
unsafe impl bytemuck::Pod for LightRaw {}

pub struct LightBuffer {
    pub buffer: wgpu::Buffer,
}

impl LightBuffer {
    pub fn new(device: &wgpu::Device, lights: &[&Light]) -> Self {
        let light_raws = lights.iter().map(|light| light.to_raw()).collect::<Vec<_>>();

        let buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Lights Buffer"), // VB ... ?
                contents: bytemuck::cast_slice(&light_raws),
                usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST,
            }
        );

        Self {
            buffer,
        }
    }

    pub fn update_light(&mut self, queue: &wgpu::Queue, light: &Light) {
        let offset = light.id * std::mem::size_of::<LightRaw>();
        let offset = offset as u64;
        queue.write_buffer(
            &self.buffer,
            offset,
            bytemuck::cast_slice(&[light.to_raw()])
        );
    }
}

/*
pub struct LightSetting {
    pub buffer: wgpu::Buffer,
    pub layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl LightSetting {
    pub fn new(device: &wgpu::Device, lights: &[&Light]) -> Self {
        let light_raws = lights.iter().map(|light| light.to_raw()).collect::<Vec<_>>();

        let buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Lights Buffer"), // VB ... ?
                contents: bytemuck::cast_slice(&light_raws),
                usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST,
            }
        );

        let layout =
            device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            readonly: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: None,
                }
            );

        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(buffer.slice(..))
                }],
                label: None,
            }
        );

        Self {
            buffer,
            layout,
            bind_group,
        }
    }

    pub fn update_light(&mut self, queue: &wgpu::Queue, light: &Light) {
        let offset = light.id * std::mem::size_of::<LightRaw>();
        let offset = offset as u64;
        queue.write_buffer(
            &self.buffer,
            offset,
            bytemuck::cast_slice(&[light.to_raw()])
        );
    }
}
*/