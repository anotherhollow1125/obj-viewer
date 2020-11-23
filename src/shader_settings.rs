mod camera;
use camera::*;
mod uniform;
use uniform::*;
mod texture;
use texture::*;
mod model;
use model::*;
mod light;
use light::*;

use cgmath::prelude::*;
use anyhow::*;
use winit::{
    event::*,
    window::Window,
};
use std::rc::Rc;

pub struct ShaderState {
    w_size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,

    depth_texture: Texture,

    camera: Camera,
    camera_controller: CameraController,

    uniform_setting: UniformSetting,

    #[allow(dead_code)]
    instance: Instance,
    instance_2: Instance,
    instance_setting: InstanceSetting,

    lights: Vec<Light>,
    light_setting: LightSetting,

    light_instance_setting: InstanceSetting,
    light_instance_1: Instance,
    #[allow(dead_code)]
    light_instance_2: Instance,

    render_pipeline: wgpu::RenderPipeline,
    light_render_pipeline: wgpu::RenderPipeline,
}

impl ShaderState {
    // Creating some of the wgpu types requires async code
    pub async fn new(window: &Window) -> Result<Self> {
        let w_size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            },
        ).await.context("adapter is None.")?;

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                shader_validation: true,
            },
            None, // Trace path
        ).await.context("device or queue is None.")?;

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: w_size.width,
            height: w_size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let texture_setting = texture::TextureSetting::new(&device);
        let depth_texture = texture::Texture::create_depth_texture(&device, &sc_desc, "depth_texture");

        let camera = Camera {
            // eye: (0.0, 1.0, 2.0).into(),
            eye: (0.0, 15.0, 32.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: sc_desc.width as f32 / sc_desc.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let lights = vec![
            Light {
                id: 0,
                position: (2.0, 2.0, 0.0).into(),
                color: (1.0, 1.0, 1.0).into(),
            },
            Light {
                id: 1,
                position: (0.0, 5.0, 0.0).into(),
                color: (1.0, 0.0, 0.0).into(),
            },
        ];

        let mut uniform_setting = uniform::UniformSetting::new(&device, lights.len() as u32);
        uniform_setting.uniforms.update_view_proj(&camera);

        let assets_dir = std::path::Path::new(env!("OUT_DIR")).join("assets");
        let mdel = model::Model::load(
            0,
            &device,
            &queue,
            &texture_setting.layout,
            assets_dir.join("dice2.obj"),
        )?;
        let mdel = Rc::new(mdel);

        let mut instance = Model::instantiate(
            mdel.clone(),
            (1.0, 0.0, 0.0).into(),
            cgmath::Quaternion::from_axis_angle(
                cgmath::Vector3::unit_z(),
                cgmath::Deg(0.0)
            ),
            1.0
        );

        let mut instance_2 = Model::instantiate(
            mdel.clone(),
            (-1.0, 0.0, 0.0).into(),
            cgmath::Quaternion::from_axis_angle(
                cgmath::Vector3::unit_z(),
                cgmath::Deg(0.0)
            ),
            0.7
        );

        let mut ins_vec = vec![&mut instance, &mut instance_2];
        let instance_setting = InstanceSetting::new(&device, &mut ins_vec);

        let t = lights.iter().collect::<Vec<_>>();
        let light_setting = LightSetting::new(&device, &t);

        let render_pipeline_layout =
            device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    // gfx_definesに近い...?
                    bind_group_layouts: &[
                        &texture_setting.layout,
                        &uniform_setting.layout,
                        &instance_setting.layout,
                        &light_setting.layout,
                    ],
                    push_constant_ranges: &[],
                }
            );

        let render_pipeline = create_render_pipeline(
            &device,
            render_pipeline_layout,
            &sc_desc,
            wgpu::include_spirv!("./shader.vert.spv"),
            wgpu::include_spirv!("./shader.frag.spv"),
        )?;

        let light_model_1 = model::Model::load(
            2,
            &device,
            &queue,
            &texture_setting.layout,
            assets_dir.join("white_cube.obj"),
        )?;
        let light_model_1 = Rc::new(light_model_1);

        let mut light_instance_1 = Model::instantiate(
            light_model_1.clone(),
            lights[0].position,
            cgmath::Quaternion::from_axis_angle(
                cgmath::Vector3::unit_z(),
                cgmath::Deg(0.0)
            ),
            0.2
        );

        let light_model_2 = model::Model::load(
            3,
            &device,
            &queue,
            &texture_setting.layout,
            assets_dir.join("red_cube.obj"),
        )?;
        let light_model_2 = Rc::new(light_model_2);

        let mut light_instance_2 = Model::instantiate(
            light_model_2.clone(),
            lights[1].position,
            cgmath::Quaternion::from_axis_angle(
                cgmath::Vector3::unit_z(),
                cgmath::Deg(0.0)
            ),
            0.2
        );

        let mut ins_vec = vec![&mut light_instance_1, &mut light_instance_2];
        let light_instance_setting = InstanceSetting::new(&device, &mut ins_vec);

        let light_render_pipeline_layout =
            device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Light Render Pipeline Layout"),
                    bind_group_layouts: &[
                        &texture_setting.layout,
                        &uniform_setting.layout,
                        &light_instance_setting.layout,
                    ],
                    push_constant_ranges: &[],
                }
            );

        let light_render_pipeline = create_render_pipeline(
            &device,
            light_render_pipeline_layout,
            &sc_desc,
            wgpu::include_spirv!("./no_shade.vert.spv"),
            wgpu::include_spirv!("./no_shade.frag.spv"),
        )?;

        Ok(Self {
            w_size,
            surface,
            device,
            queue, // command queue
            sc_desc,
            swap_chain,

            depth_texture,

            camera,
            camera_controller: CameraController::new(0.2),

            uniform_setting,

            // mdel,
            instance,
            instance_2,
            instance_setting,

            lights,
            light_setting,

            light_instance_1,
            light_instance_2,
            light_instance_setting,

            render_pipeline,
            light_render_pipeline,
        })
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.w_size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);

        self.camera.aspect = self.sc_desc.width as f32 / self.sc_desc.height as f32;

        self.depth_texture = texture::Texture::create_depth_texture(
            &self.device,
            &self.sc_desc,
            "depth_texture"
        );
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.camera_controller.process_events(event)
    }

    pub fn update(&mut self) -> Result<()> {
        self.camera_controller.update_camera(&mut self.camera);
        self.uniform_setting.uniforms.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.uniform_setting.buffer,
            0,
            bytemuck::cast_slice(&[self.uniform_setting.uniforms])
        );

        // Update instances
        self.instance.rotation = cgmath::Quaternion::from_axis_angle(
            cgmath::Vector3::unit_y(),
            cgmath::Deg(1.0),
        ) * self.instance.rotation;
        self.instance_setting.update_instance(&self.queue, &self.instance).unwrap();

        self.instance_2.rotation = cgmath::Quaternion::from_axis_angle(
            cgmath::Vector3::unit_z(),
            cgmath::Deg(1.0),
        ) * self.instance_2.rotation;
        self.instance_setting.update_instance(&self.queue, &self.instance_2).unwrap();

        // Update the light
        {
            let mut light = &mut self.lights[0];
            let p = cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(1.0))
                * light.position;
            light.position = p;
            self.light_instance_1.position = p;
        }
        self.light_setting.update_light(&self.queue, &self.lights[0]);
        self.light_instance_setting.update_instance(&self.queue, &self.light_instance_1)?;

        Ok(())
    }

    pub fn render(&mut self) {
        let frame = self.swap_chain.get_current_frame()
            .expect("Timeout getting texture")
            .output;

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            }
        );

        // borrow encoder as &mut
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            // 色について
            color_attachments: &[
                wgpu::RenderPassColorAttachmentDescriptor {
                    // 書き出し先
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: true,
                    }
                }
            ],
            // 深さについて
            depth_stencil_attachment: Some(
                wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }
            ),
        });

        render_pass.set_pipeline(&self.light_render_pipeline);
        render_pass.draw_model_instance_groups(
            &self.light_instance_setting,
            &self.uniform_setting.bind_group,
            &self.light_setting.bind_group,
        );

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.draw_model_instance_groups(
            &self.instance_setting,
            &self.uniform_setting.bind_group,
            &self.light_setting.bind_group,
        );
        // borrow end
        drop(render_pass);

        // Submit Command. and its result will appear on frame.
        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
    }
}

fn create_render_pipeline(
    device: &wgpu::Device,
    render_pipeline_layout: wgpu::PipelineLayout,
    sc_desc: &wgpu::SwapChainDescriptor,
    vert_src: wgpu::ShaderModuleSource,
    frag_src: wgpu::ShaderModuleSource,
) -> Result<wgpu::RenderPipeline> {
    let vs_module = device.create_shader_module(vert_src);
    let fs_module = device.create_shader_module(frag_src);

    let res = device.create_render_pipeline(
        &wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(
                wgpu::RasterizationStateDescriptor {
                    // 三角形で描画するの意味。(それしかないらしい)
                    // Counter clockwise の略。右手系標準ということ
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: wgpu::CullMode::Back,
                    depth_bias: 0,
                    depth_bias_slope_scale: 0.0,
                    depth_bias_clamp: 0.0,
                    clamp_depth: false,
                }
            ),
            color_states: &[
                wgpu::ColorStateDescriptor {
                    format: sc_desc.format,
                    color_blend: wgpu::BlendDescriptor::REPLACE,
                    // 透明度も塗り替え
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                },
            ],
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            depth_stencil_state: Some(
                wgpu::DepthStencilStateDescriptor {
                    format: texture::Texture::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
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
    Ok(res)
}