// use crate::shader_settings::texture::Texture;
use crate::shader_settings::camera::Projection;
use crate::shader_settings::model::{self, Vertex, ModelInstanceGroupBook};
use cgmath::*;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ShadowUniform {
    view_proj: Matrix4<f32>,
    tex_width: u32,
    tex_height: u32,
    darkness: f32,
    _p: u32,
}

unsafe impl bytemuck::Pod for ShadowUniform {}
unsafe impl bytemuck::Zeroable for ShadowUniform {}

impl ShadowUniform {
    fn new(tex_width: u32, tex_height: u32, darkness: f32) -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity(),
            tex_width,
            tex_height,
            darkness,
            _p: 0,
        }
    }
}

pub struct ShadowUniformBuffer {
    pub buffer: wgpu::Buffer,
}

impl ShadowUniformBuffer {
    pub fn new(device: &wgpu::Device, uniforms: &[ShadowUniform]) -> Self {
        let buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Shadow Uniforms Buffer"),
                contents: bytemuck::cast_slice(uniforms),
                usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST,
            }
        );

        Self {
            buffer,
        }
    }

    pub fn update_uniform(&mut self, queue: &wgpu::Queue, shadowmap: &ShadowMap) {
        let offset = shadowmap.id * std::mem::size_of::<ShadowUniform>();
        let offset = offset as u64;
        queue.write_buffer(
            &self.buffer,
            offset,
            bytemuck::cast_slice(&[shadowmap.shadow_uniform])
        );
    }
}

pub enum DirUpdateWay {
    SunLight { // don't use vec3_arg, it use the light pos
        anchor_pos: Vector3<f32>, // use it to decide the direction
    },
    SpotLight, // recognize vec3_arg as direction
    Constant {
        dir: Vector3<f32>,
    },
    Custom {
        f: Box<dyn Fn(Vector3<f32>) -> Vector3<f32>>,
    },
}

pub struct ShadowMap {
    id: usize,
    pub position: Point3<f32>,
    pub direction: Vector3<f32>,
    dir_update_way: DirUpdateWay,
    projection: Projection,
    pub shadow_uniform: ShadowUniform,
    pub render_pipeline: wgpu::RenderPipeline,
    // pub texture: Texture,
    target_view: wgpu::TextureView,
    // bake_layout: wgpu::BindGroupLayout,
    uniform_buffer_for_bake: wgpu::Buffer,
    bake_bind_group: wgpu::BindGroup,

    // pub tex_layout: wgpu::BindGroupLayout,
    // pub tex_bind_group: wgpu::BindGroup,
}

impl ShadowMap {
    const SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    fn view_config<'a>(id: usize) -> wgpu::TextureViewDescriptor<'a> {
        wgpu::TextureViewDescriptor {
            label: Some("shadow"),
            format: None,
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            level_count: None,
            base_array_layer: id as u32,
            array_layer_count: std::num::NonZeroU32::new(1),
        }
    }

    pub fn new(
        id: usize,
        position: Point3<f32>,
        init_vec: Vector3<f32>,
        darkness: f32,
        dir_update_way: DirUpdateWay,
        projection: Projection,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sc_desc: &wgpu::SwapChainDescriptor,
        instance_layout: &wgpu::BindGroupLayout,
        shadow_texture: &wgpu::Texture,
    ) -> Self {
        let v_conf = Self::view_config(id);
        let target_view = shadow_texture.create_view(&v_conf);
        let bake_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::UniformBuffer {
                            dynamic: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("shadowMap_bind_group_layout"),
            }
        );
        let shadow_uniform = ShadowUniform::new(sc_desc.width, sc_desc.height, darkness);

        let uniform_buffer_for_bake = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("shadowmap Buffer"),
                contents: bytemuck::cast_slice(&[shadow_uniform]),
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            }
        );

        let bake_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &bake_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(uniform_buffer_for_bake.slice(..)),
                    },
                ],
                label: None,
            }
        );

        let render_pipeline_layout =
            device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[
                        &bake_layout,
                        &instance_layout,
                    ],
                    push_constant_ranges: &[],
                }
            );

        let render_pipeline = ShadowMap::create_render_pipeline(
            device,
            render_pipeline_layout,
            wgpu::include_spirv!("../bake.vert.spv"),
        );

        let res = Self {
            id,
            position,
            direction: init_vec,
            dir_update_way,
            projection,
            shadow_uniform,
            render_pipeline,
            target_view,
            // bake_layout,
            uniform_buffer_for_bake,
            bake_bind_group,

            // tex_layout,
            // tex_bind_group,
        };

        // res.update(None, Some(init_vec), queue, shadow_uniform_buffer);
        // res.update_view_proj(queue);

        queue.write_buffer(
            &res.uniform_buffer_for_bake,
            0,
            bytemuck::cast_slice(&[res.shadow_uniform])
        );

        res
    }

    pub fn update(
        &mut self,
        pos_v_w: Option<Vector3<f32>>,
        dir_v_w: Option<Vector3<f32>>,
        queue: &wgpu::Queue,
        shadow_uniform_buffer: &mut ShadowUniformBuffer
    ) {
        use DirUpdateWay::*;

        if let Some(pos_v) = pos_v_w {
            self.position = Point3::from_vec(pos_v);
        }

        if let Some(dir_v) = dir_v_w {
            self.direction = match &self.dir_update_way {
                SunLight {anchor_pos} => anchor_pos - self.position.to_vec(),
                SpotLight => dir_v,
                Constant {dir} => *dir,
                Custom { f } => f(dir_v),
            };
        } else {
            match &self.dir_update_way {
                SunLight {anchor_pos} => self.direction = anchor_pos - self.position.to_vec(),
                // Constant {dir} => self.direction = *dir,
                _ => (),
            }
        }

        self.update_view_proj(queue, shadow_uniform_buffer);
    }

    // 光源位置変更時等に呼び出す必要がある
    fn update_view_proj(&mut self, queue: &wgpu::Queue, shadow_uniform_buffer: &mut ShadowUniformBuffer) {
        // self.shadow_uniform.view_position = self.position.to_homogeneous();
        let n = self.direction.normalize();
        let axis = if n.x.powi(2) + n.z.powi(2) != 0.0 {
            Vector3::unit_y()
        } else {
            Vector3::unit_x()
        };

        let m = Matrix4::look_at_dir(
            self.position,
            n,
            axis,
        );
        self.shadow_uniform.view_proj = self.projection.calc_matrix() * m;

        shadow_uniform_buffer.update_uniform(queue, self);

        queue.write_buffer(
            &self.uniform_buffer_for_bake,
            0,
            bytemuck::cast_slice(&[self.shadow_uniform])
        );
    }

    fn create_render_pipeline(
        device: &wgpu::Device,
        render_pipeline_layout: wgpu::PipelineLayout,
        vert_src: wgpu::ShaderModuleSource,
    ) -> wgpu::RenderPipeline {
        let vs_module = device.create_shader_module(vert_src);
    
        // 設定値参考
        // https://github.com/gfx-rs/wgpu-rs/blob/master/examples/shadow/main.rs
        let res = device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("Shadow Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex_stage: wgpu::ProgrammableStageDescriptor {
                    module: &vs_module,
                    entry_point: "main",
                },
                fragment_stage: None,
                rasterization_state: Some(
                    wgpu::RasterizationStateDescriptor {
                        // 三角形で描画するの意味。(それしかないらしい)
                        // Counter clockwise の略。右手系標準ということ
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: wgpu::CullMode::Back,
                        depth_bias: 2,
                        depth_bias_slope_scale: 2.0,
                        depth_bias_clamp: 0.0,
                        clamp_depth: device.features().contains(wgpu::Features::DEPTH_CLAMPING),
                    }
                ),
                color_states: &[],
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                depth_stencil_state: Some(
                    wgpu::DepthStencilStateDescriptor {
                        format: Self::SHADOW_FORMAT,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::LessEqual,
                        stencil: wgpu::StencilStateDescriptor::default(),
                    }
                ),
                vertex_state: wgpu::VertexStateDescriptor {
                    // index_format: wgpu::IndexFormat::Uint16,
                    index_format: wgpu::IndexFormat::Uint32,
                    vertex_buffers: &[
                        // Vertex::desc()
                        model::ModelVertex::desc()
                    ],
                },
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            }
        );
        
        res
    }

    #[allow(dead_code)]
    pub fn resize(&mut self, sc_desc: &wgpu::SwapChainDescriptor) {
        self.projection.resize(sc_desc.width, sc_desc.height);
    }

    pub fn render_to_texture(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        // instance_setting: &InstanceSetting,
        model_instance_group_book: &ModelInstanceGroupBook,
    ) {
        // borrow encoder as &mut
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            // 色について
            color_attachments: &[],
            // 深さについて
            depth_stencil_attachment: Some(
                wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &self.target_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }
            ),
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.draw_shadow_of_instance_groups(
            // instance_setting,
            model_instance_group_book,
            &self.bake_bind_group,
        );
        // borrow end
        // drop(render_pass);

        // queue.submit(std::iter::once(encoder.finish()));
    }
}

pub trait DrawShadow<'a, 'b>
where
    'b: 'a,
{
    fn draw_shadow_of_instance_groups(
        &mut self,
        // instance_setting: &'b InstanceSetting,
        model_instance_group_book: &'b ModelInstanceGroupBook,
        uni_bg: &'b wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawShadow<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_shadow_of_instance_groups(
        &mut self,
        // instance_setting: &'b InstanceSetting,
        model_instance_group_book: &'b ModelInstanceGroupBook,
        uni_bg: &'b wgpu::BindGroup,
    ) {
        for (model, group) in model_instance_group_book.group_book.iter() {
            for mesh in &model.meshes {
                self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                self.set_index_buffer(mesh.index_buffer.slice(..));
                self.set_bind_group(0, &uni_bg, &[]);
                self.set_bind_group(1, &group.bind_group, &[]);
                self.draw_indexed(0..mesh.num_elements, 0, 0..(group.len as u32));
            }
        }
    }
}