// skip now.

use crate::shader_settings::model;
use wgpu::util::DeviceExt;
#[macro_use]
use anyhow::*;

// インスタンスの設定: たとえ1つでも、全てインスタンスを設置する形で実装するようにしたい
// 要検証
pub struct<'a> Instance<'a> {
    // モデルのインデックス番号でも良かったけど、
    // 開けた場所で使いたかったのでモデルとライフタイムを一致させたい
    model: &'a model::Model,
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct InstanceRaw {
    model: cgmath::Matrix4<f32>,
}

unsafe impl bytemuck::Pod for InstanceRaw {}
unsafe impl bytemuck::Zeroable for InstanceRaw {}

impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model:
                cgmath::Matrix4::from_translation(self.position)
                * cgmath::Matrix4::from(self.rotation),
        }
    }
}

pub struct<'a> InstanceGroup<'a> {
    model: &'a model::Model,
    buffer: wgpu::Buffer,
    layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl InstanceGroup {
    pub fn new(device: &wgpu::Device, instances: Vec<Instance>) -> Result<Self> {
        if instances.len() == 0 {
            return Err(anyhow!("Vector is empty."));
        }

        let id = instances[0].model.id;

        // あまり美しくないけど、簡単のために
        // 最初と同じオブジェクトしか描写しないことで誤った描写を回避する。
        let instance_data = instances.iter()
            .filter(|ins| **(ins.model.id) == id)
            .map(Instance::to_raw).collect::<Vec<_>>();

        let buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsage::STORAGE,
            }
        );

        let layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            readonly: true,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("instance_bind_group_layout"),
            }
        );

        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &uniform_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(buffer.slice(..))
                    }
                ],
                label: Some("instance_bind_group"),
            }
        );

        Ok(Self {
            model: &instances[0].model,
            buffer,
            layout,
            bind_group,
        })
    }
}