use wgpu::util::DeviceExt;

// memo
// StorageBuffer
// let lights: Vec<Light>;
// contents: bytemuck::cast_slice(&lights);

// 光源の設定
pub struct Light {
    pub id: usize,
    pub position: cgmath::Vector3<f32>,
    pub color: cgmath::Vector3<f32>,
}

impl Light {
    pub fn to_raw(&self) -> LightRaw {
        LightRaw {
            position: self.position,
            _p1: 0,
            color: self.color,
            _p2: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct LightRaw {
    pub position: cgmath::Vector3<f32>,
    _p1: u32, // 16 byte dummy data ... u32 ...? (size_of(u32) == 4 byte)
    // => I got it. 16 byte == size_of(Vec4<f32>) and 1 word (this mean use one line of memory address.). so this u32 is important.
    pub color: cgmath::Vector3<f32>,
    _p2: u32,
    // skip now.
    /*
    is_spotlight: u32,
    limitcos_inner: f32,
    limitcos_outer: f32,
    */
}

unsafe impl bytemuck::Zeroable for LightRaw {}
unsafe impl bytemuck::Pod for LightRaw {}

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