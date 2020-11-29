pub mod camera;
use camera::*;
pub mod uniform;
use uniform::*;
pub mod texture;
use texture::*;
pub mod model;
use model::*;
pub mod light;
use light::*;
pub mod shadowmap;
use shadowmap::*;

#[allow(unused_imports)]
use cgmath::prelude::*;
use anyhow::*;
use winit::{
    event::*,
    window::Window,
};

use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

pub struct ShaderState {
    w_size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    device: wgpu::Device,
    pub queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,

    depth_texture: Texture,

    camera_setting: CameraSetting,

    uniform_setting: UniformSetting,

    pub instance_setting: InstanceSetting,
    pub light_buffer: LightBuffer, 
    pub light_instance_setting: InstanceSetting,

    pub shadowmap: ShadowMap,

    render_pipeline: wgpu::RenderPipeline,
    light_render_pipeline: wgpu::RenderPipeline,

    pub instance_book: HashMap<String, Rc<RefCell<Instance>>>,
    pub light_book: Vec<Rc<RefCell<Light>>>,
}

impl ShaderState {
    // Creating some of the wgpu types requires async code
    pub async fn new<F>(
        window: &Window,
        f: F
    ) -> Result<Self>
    where
        F: Fn(
            &wgpu::Device,
            &wgpu::Queue,
            &wgpu::BindGroupLayout,
        ) -> Result<(Vec<Instance>, Vec<Light>, Vec<Instance>)>
    {
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
        let depth_texture = texture::Texture::create_depth_texture(&device, &sc_desc, "depth_texture", None);

        let camera_setting = CameraSetting::new(sc_desc.width, sc_desc.height);

        let (
            mut instances,
            lights,
            mut light_instances
        ) = f(&device, &queue, &texture_setting.layout)?;

        let lights_len = lights.len();

        let mut ins_vec = instances.iter_mut().collect::<Vec<_>>();
        let instance_setting = InstanceSetting::new(&device, &mut ins_vec);
        let lig_vec = lights.iter().collect::<Vec<_>>();
        // let light_setting = LightSetting::new(&device, &lig_vec);
        let light_buffer = LightBuffer::new(&device, &lig_vec);
        drop(lig_vec);

        let mut ins_vec = light_instances.iter_mut().collect::<Vec<_>>();
        // 影は不要
        let light_instance_setting = InstanceSetting::new(&device, &mut ins_vec);
        drop(ins_vec);

        let instance_book = vec![
            instances, light_instances
        ].into_iter()
            .flatten()
            .map(|instance| {
                let name = instance.name.clone();
                (name, Rc::new(RefCell::new(instance)))
            }).collect::<HashMap<_, _>>();

        let mut light_book = lights
            .into_iter()
            .map(|light| Rc::new(RefCell::new(light)))
            .collect::<Vec<_>>();
        light_book.sort();

        let main_light = light_book[0].borrow();

        let shadowmap = ShadowMap::new(
            cgmath::Point3::from_vec(main_light.position),
            -main_light.position.normalize(),
            camera_setting.projection.clone(),
            &device,
            &queue,
            &sc_desc,
            &instance_setting.layout,
        );

        drop(main_light);

        let mut uniform_setting = uniform::UniformSetting::new(
            &device,
            lights_len as u32,
            &light_buffer.buffer,
            &shadowmap,
        );
        uniform_setting.uniforms.update_view_proj(
            &camera_setting.camera,
            &camera_setting.projection,
        );

        let render_pipeline_layout =
            device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    // gfx_definesに近い...?
                    bind_group_layouts: &[
                        &texture_setting.layout,
                        &uniform_setting.layout,
                        &instance_setting.layout,
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

            camera_setting,

            uniform_setting,

            instance_setting,
            light_buffer,
            light_instance_setting,

            shadowmap,

            render_pipeline,
            light_render_pipeline,

            instance_book,
            light_book,
        })
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.w_size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);

        self.camera_setting.projection.resize(new_size.width, new_size.height);

        self.depth_texture = texture::Texture::create_depth_texture(
            &self.device,
            &self.sc_desc,
            "depth_texture",
            None
        );

        self.shadowmap.resize(&self.device, &self.sc_desc);
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input: KeyboardInput {
                    virtual_keycode: Some(key),
                    state,
                    ..
                },
                ..
            } => self.camera_setting.camera_controller.process_keyboard(*key, *state),
            WindowEvent::MouseWheel {
                delta,
                ..
            } => {
                self.camera_setting.camera_controller.process_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.camera_setting.mouse_pressed = *state == ElementState::Pressed;
                true
            }
            WindowEvent::CursorMoved {
                position,
                ..
            } => {
                let winit::dpi::PhysicalPosition{x, y} = self.camera_setting.last_mouse_pos;
                let mouse_dx = position.x - x;
                let mouse_dy = position.y - y;
                self.camera_setting.last_mouse_pos = *position;
                if self.camera_setting.mouse_pressed {
                    self.camera_setting.camera_controller
                        .process_mouse(mouse_dx, mouse_dy);
                }
                true
            }
            _ => false,
        }
    }

    pub fn update<F>(&mut self, dt: std::time::Duration, f: F) -> Result<()>
    where
        F: Fn(&mut Self) -> Result<()>
    {
        self.camera_setting.camera_controller
            .update_camera(&mut self.camera_setting.camera, dt);
        self.uniform_setting.uniforms
            .update_view_proj(
                &self.camera_setting.camera,
                &self.camera_setting.projection,
            );
        self.queue.write_buffer(
            &self.uniform_setting.buffer,
            0,
            bytemuck::cast_slice(&[self.uniform_setting.uniforms])
        );

        f(self)?;

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

        self.shadowmap.render_to_texture(
            &mut encoder,
            &self.instance_setting,
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
            // &self.light_setting.bind_group,
        );

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.draw_model_instance_groups(
            &self.instance_setting,
            &self.uniform_setting.bind_group,
            // &self.light_setting.bind_group,
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