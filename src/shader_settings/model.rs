use crate::shader_settings::texture;
use anyhow::*;
use std::path::*;
use std::ops::Range;

use wgpu::util::DeviceExt;
use cgmath::prelude::*;

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ModelVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    normal: [f32; 3],
}

unsafe impl bytemuck::Pod for ModelVertex {}
unsafe impl bytemuck::Zeroable for ModelVertex {}

impl Vertex for ModelVertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        const OFFSET_2: wgpu::BufferAddress = std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress;
        const OFFSET_3: wgpu::BufferAddress = std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress + OFFSET_2;
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float3,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: OFFSET_2,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float2,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: OFFSET_3,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float3,
                },
            ],
        }
    }
}

pub struct Model {
    pub id: usize,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

use std::cmp::{PartialEq, Eq};
use std::hash::{Hash, Hasher};

impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Model {}

impl Hash for Model {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub struct Material {
    pub name: String,
    // Optionに変更する -> 代わりに、代替のテクスチャを充てることにした
    pub diffuse_texture: texture::Texture,
    pub matuni_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct MaterialUniform {
    use_texture: u32,
    _p1: cgmath::Vector3<f32>,
    ambient_color: cgmath::Vector3<f32>,
    _p2: u32,
    diffuse_color: cgmath::Vector3<f32>,
    _p3: u32,
    specular_color: cgmath::Vector3<f32>,
    _p4: u32,
}

unsafe impl bytemuck::Pod for MaterialUniform {}
unsafe impl bytemuck::Zeroable for MaterialUniform {}

// メッシュは頂点の集まりのこと。
// https://github.com/PistonDevelopers/wavefront_obj の
// wavefront_obj で言うところの各オブジェ
pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize, // インデックスくさい -> そうだった
}

impl Model {
    pub fn load<P: AsRef<Path>>(
        id: usize,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        path: P,
    ) -> Result<Self> {
        let (obj_models, obj_materials) = tobj::load_obj(path.as_ref(), true)?;

        // 画像ファイルは同一階層にあると仮定
        let containing_folder = path.as_ref().parent()
            .context("Directory has no parent")?;

        let mut materials = Vec::new();
        for mat in obj_materials {
            let diffuse_path = mat.diffuse_texture; // ?
            let diffuse_texture_w = if diffuse_path != "" {
                Some(texture::Texture::load(
                    device,
                    queue,
                    containing_folder.join(diffuse_path)
                )?)
            } else {
                None
            };
            let material_uniform = MaterialUniform {
                use_texture: if diffuse_texture_w.is_some() { 1 } else { 0 },
                _p1: (0.0, 0.0, 0.0).into(),
                ambient_color: mat.ambient.into(),
                _p2: 0,
                diffuse_color: mat.diffuse.into(),
                _p3: 0,
                specular_color: mat.specular.into(),
                _p4: 0,
            };
            let matuni_buffer = device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Material Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[material_uniform]),
                    usage: wgpu::BufferUsage::UNIFORM,
                }
            );
            let diffuse_texture = if let Some(t) = diffuse_texture_w {
                t
            } else {
                texture::Texture::load(
                    device,
                    queue,
                    "./assets/default_texture.png",
                )?
            };
            let bind_group = device.create_bind_group(
                &wgpu::BindGroupDescriptor {
                    layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Buffer(matuni_buffer.slice(..))
                        },
                    ],
                    label: None,
                }
            );

            materials.push(
                Material {
                    name: mat.name,
                    diffuse_texture,
                    matuni_buffer,
                    bind_group,
                }
            );
        }

        let mut meshes = Vec::new();
        for m in obj_models {
            let mut vertices = Vec::new();
            // x, y, z 全部を一つにしている模様
            for i in 0..(m.mesh.positions.len() / 3) {
                vertices.push(
                    ModelVertex {
                        position: [
                            m.mesh.positions[i * 3    ],
                            m.mesh.positions[i * 3 + 1],
                            m.mesh.positions[i * 3 + 2],
                        ],
                        tex_coords: [
                            m.mesh.texcoords[i * 2    ],
                            1.0-m.mesh.texcoords[i * 2 + 1],
                        ],
                        normal: [
                            m.mesh.normals[i * 3    ],
                            m.mesh.normals[i * 3 + 1],
                            m.mesh.normals[i * 3 + 2],
                        ],
                    }
                );
            }

            let vertex_buffer = device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Vertex Buffer", path.as_ref())),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsage::VERTEX,
                }
            );
            let index_buffer = device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Index Buffer", path.as_ref())),
                    contents: bytemuck::cast_slice(&m.mesh.indices),
                    usage: wgpu::BufferUsage::INDEX,
                }
            );

            meshes.push(
                Mesh {
                    name: m.name,
                    vertex_buffer,
                    index_buffer,
                    num_elements: m.mesh.indices.len() as u32,
                    // 1つ以上はマテリアルは存在するはず
                    material: m.mesh.material_id.unwrap_or(0),
                }
            );
        }

        Ok(Self { id, meshes, materials })
    }
}

// メソッドを生やす
// ライフタイムの意味は'bは'aより長生き、だったはず
pub trait DrawModel<'a, 'b>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        uni_bg: &'b wgpu::BindGroup,
        ins_bg: &'b wgpu::BindGroup,
        // lig_bg: &'b wgpu::BindGroup,
        // shm_bg: &'b wgpu::BindGroup,
    );

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        ins_range: Range<u32>,
        uni_bg: &'b wgpu::BindGroup,
        ins_bg: &'b wgpu::BindGroup,
        // lig_bg: &'b wgpu::BindGroup,
        // shm_bg: &'b wgpu::BindGroup,
    );

    fn draw_model(
        &mut self,
        model: &'b Model,
        uni_bg: &'b wgpu::BindGroup,
        ins_bg: &'b wgpu::BindGroup,
        // lig_bg: &'b wgpu::BindGroup,
        // shm_bg: &'b wgpu::BindGroup,
    );
    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        ins_range: Range<u32>,
        uni_bg: &'b wgpu::BindGroup,
        ins_bg: &'b wgpu::BindGroup,
        // lig_bg: &'b wgpu::BindGroup,
        // shm_bg: &'b wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawModel<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        uni_bg: &'b wgpu::BindGroup,
        ins_bg: &'b wgpu::BindGroup,
        // lig_bg: &'b wgpu::BindGroup,
        // shm_bg: &'b wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(
            mesh,
            material,
            0..1,
            uni_bg,
            ins_bg,
            // lig_bg,
            // shm_bg,
        );
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        ins_range: Range<u32>,
        uni_bg: &'b wgpu::BindGroup,
        ins_bg: &'b wgpu::BindGroup,
        // lig_bg: &'b wgpu::BindGroup,
        // shm_bg: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..));
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, uni_bg, &[]);
        self.set_bind_group(2, ins_bg, &[]);
        // self.set_bind_group(3, lig_bg, &[]);
        // self.set_bind_group(4, shm_bg, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, ins_range);
    }

    fn draw_model(
        &mut self,
        model: &'b Model,
        uni_bg: &'b wgpu::BindGroup,
        ins_bg: &'b wgpu::BindGroup,
        // lig_bg: &'b wgpu::BindGroup,
        // shm_bg: &'b wgpu::BindGroup,
    ) {
        self.draw_model_instanced(model, 0..1, uni_bg, ins_bg, /*lig_bg, shm_bg*/);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        ins_range: Range<u32>,
        uni_bg: &'b wgpu::BindGroup,
        ins_bg: &'b wgpu::BindGroup,
        // lig_bg: &'b wgpu::BindGroup,
        // shm_bg: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(
                mesh,
                material,
                ins_range.clone(),
                uni_bg,
                ins_bg,
                // lig_bg,
                // shm_bg,
            );
        }
    }
}

use cgmath::{
    Vector3,
    Quaternion,
};

use std::rc::Rc;

// 拡大 -> 回転 -> 移動
pub struct Instance {
    pub name: String,
    index: usize,
    model: Rc<Model>,
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: f32,
}

impl PartialEq for Instance {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Eq for Instance {}

impl Hash for Instance {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct InstanceRaw {
    transform: cgmath::Matrix4<f32>,
    transform_norm: cgmath::Matrix4<f32>,
}

unsafe impl bytemuck::Pod for InstanceRaw {}
unsafe impl bytemuck::Zeroable for InstanceRaw {}

impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        let transform = cgmath::Matrix4::from_translation(self.position)
            * cgmath::Matrix4::from(self.rotation)
            * cgmath::Matrix4::from_scale(self.scale);
        let mut t = transform.invert().unwrap_or(transform);
        t.transpose_self();
        InstanceRaw {
            transform,
            transform_norm: t,
        }
    }
}

impl Model {
    pub fn instantiate(
        model: Rc<Model>,
        name: String,
        position: Vector3<f32>,
        rotation: Quaternion<f32>,
        scale: f32,
    ) -> Instance {
        Instance {
            name,
            index: 0,
            model,
            position,
            rotation,
            scale,
        }
    }
}

use std::collections::HashMap;

pub struct ModelInstanceGroup {
    pub len: usize,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

pub struct InstanceSetting {
    pub layout: wgpu::BindGroupLayout,
    pub group_book: HashMap<Rc<Model>, ModelInstanceGroup>,
}

impl InstanceSetting {
    pub fn new(device: &wgpu::Device, instances: &mut [&mut Instance]) -> Self {
        let layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            readonly: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("instance_bind_group_layout"),
            }
        );

        let mut instance_sort = HashMap::new();

        for ins in instances.iter_mut() {
            let v = instance_sort.entry(ins.model.clone()).or_insert(Vec::new());
            v.push(ins);
        }

        let group_book = instance_sort.into_iter().map(|(model, sorted)| {
            let initial_data = sorted.into_iter()
                .enumerate()
                .map(|(i, ins)| {
                    ins.index = i;
                    ins.to_raw()
                }).collect::<Vec<_>>();

            let buffer = device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Instance Buffer"),
                    contents: bytemuck::cast_slice(&initial_data),
                    usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST,
                }
            );

            let bind_group = device.create_bind_group(
                &wgpu::BindGroupDescriptor {
                    layout: &layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(buffer.slice(..))
                        }
                    ],
                    label: Some("instance_bind_group"),
                }
            );

            (model, ModelInstanceGroup {
                len: initial_data.len(),
                buffer,
                bind_group,
            })
        }).collect::<HashMap<_, _>>();

        Self {
            layout,
            group_book,
        }
    }

    #[allow(dead_code)]
    pub fn update_instance(
        &self,
        queue: &wgpu::Queue,
        instance: &Instance,
    ) -> Result<()> {
        let raw = instance.to_raw();
        let ref buffer = (self.group_book.get(&instance.model).context("Invalid Instance")?).buffer;
        let offset = instance.index * std::mem::size_of::<InstanceRaw>();
        queue.write_buffer(buffer, offset as u64, bytemuck::cast_slice(&[raw]));

        Ok(())
    }
}

pub trait DrawModelInstanceGroups<'a, 'b>
where
    'b: 'a,
{
    fn draw_model_instance_groups(
        &mut self,
        instance_setting: &'b InstanceSetting,
        uni_bg: &'b wgpu::BindGroup,
        // lig_bg: &'b wgpu::BindGroup,
        // shm_bg: &'b wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawModelInstanceGroups<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_model_instance_groups(
        &mut self,
        instance_setting: &'b InstanceSetting,
        uni_bg: &'b wgpu::BindGroup,
        // lig_bg: &'b wgpu::BindGroup,
        // shm_bg: &'b wgpu::BindGroup,
    ) {
        for (model, group) in instance_setting.group_book.iter() {
            self.draw_model_instanced(
                model,
                0..(group.len as u32),
                uni_bg,
                &group.bind_group,
                // lig_bg,
                // shm_bg,
            );
        }
    }
}