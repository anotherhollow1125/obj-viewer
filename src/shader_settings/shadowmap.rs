use crate::shader_settings::texture::Texture;
use crate::shader_settings::camera::Projection;
use crate::shader_settings::model::{self, Vertex, InstanceSetting};
use cgmath::*;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct LightViewUniform {
    view_proj: Matrix4<f32>,
    tex_width: u32,
    tex_height: u32,
}

unsafe impl bytemuck::Pod for LightViewUniform {}
unsafe impl bytemuck::Zeroable for LightViewUniform {}

impl LightViewUniform {
    fn new(tex_width: u32, tex_height: u32) -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity(),
            tex_width,
            tex_height,
        }
    }
}

pub struct ShadowMap {
    pub position: Point3<f32>,
    pub direction: Vector3<f32>,
    projection: Projection,
    light_view_uniform: LightViewUniform,
    pub render_pipeline: wgpu::RenderPipeline,
    pub texture: Texture,
    target_view: wgpu::TextureView,
    // bake_layout: wgpu::BindGroupLayout,
    pub uniform_buffer: wgpu::Buffer,
    bake_bind_group: wgpu::BindGroup,

    // pub tex_layout: wgpu::BindGroupLayout,
    // pub tex_bind_group: wgpu::BindGroup,
}

impl ShadowMap {
    const SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    const VIEW_CONFIG: wgpu::TextureViewDescriptor<'static> = wgpu::TextureViewDescriptor {
        label: Some("shadow"),
        format: None,
        dimension: Some(wgpu::TextureViewDimension::D2),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        level_count: None,
        base_array_layer: 0,
        array_layer_count: std::num::NonZeroU32::new(1),
    };

    pub fn new(
        position: Point3<f32>,
        direction: Vector3<f32>,
        projection: Projection,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sc_desc: &wgpu::SwapChainDescriptor,
        instance_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let texture = Texture::create_depth_texture(
            device,
            sc_desc,
            "ShadowMap texture",
            // Some(&Self::VIEW_CONFIG),
            None,
        );
        let target_view = texture.texture.create_view(&Self::VIEW_CONFIG);
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
        let light_view_uniform = LightViewUniform::new(sc_desc.width, sc_desc.height);
        let uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("shadowmap Buffer"),
                contents: bytemuck::cast_slice(&[light_view_uniform]),
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            }
        );
        let bake_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &bake_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(uniform_buffer.slice(..)),
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

        /*
        let tex_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            multisampled: false,
                            dimension: wgpu::TextureViewDimension::D2,
                            component_type: wgpu::TextureComponentType::Uint,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            comparison: true,
                        },
                        count: None,
                    },
                ],
                label: Some("shadowMap_bind_group_layout"),
            }
        );

        let tex_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &tex_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                ],
                label: None,
            }
        );
        */

        let mut res = Self {
            position,
            direction,
            projection,
            light_view_uniform,
            render_pipeline,
            texture,
            target_view,
            // bake_layout,
            uniform_buffer,
            bake_bind_group,

            // tex_layout,
            // tex_bind_group,
        };

        res.update_view_proj(queue);
        /*
        queue.write_buffer(
            &res.uniform_buffer,
            0,
            bytemuck::cast_slice(&[res.light_view_uniform])
        );
        */

        res
    }

    // 光源位置変更時に呼び出す必要がある
    pub fn update_view_proj(&mut self, queue: &wgpu::Queue) {
        // self.light_view_uniform.view_position = self.position.to_homogeneous();
        let m = Matrix4::look_at_dir(
            self.position,
            self.direction.normalize(),
            Vector3::unit_y(),
        );
        self.light_view_uniform.view_proj = self.projection.calc_matrix() * m;
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.light_view_uniform])
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

    pub fn resize(&mut self, device: &wgpu::Device, sc_desc: &wgpu::SwapChainDescriptor) {
        self.projection.resize(sc_desc.width, sc_desc.height);

        self.texture = Texture::create_depth_texture(
            device,
            sc_desc,
            "ShadowMap texture",
            // Some(&Self::VIEW_CONFIG),
            None,
        );
    }

    pub fn render_to_texture(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        instance_setting: &InstanceSetting,
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
            instance_setting,
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
        instance_setting: &'b InstanceSetting,
        uni_bg: &'b wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawShadow<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_shadow_of_instance_groups(
        &mut self,
        instance_setting: &'b InstanceSetting,
        uni_bg: &'b wgpu::BindGroup,
    ) {
        for (model, group) in instance_setting.group_book.iter() {
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