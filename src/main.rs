use obj_viewer::shader_settings::{
    model,
    light,
    ShaderState,
};
use model::*;
use light::*;

use winit::{
    event::*,
    event_loop::{EventLoop, ControlFlow},
    window::WindowBuilder,
};
use futures::executor::block_on;

use anyhow::*;
use std::rc::Rc;

use cgmath::prelude::*;

fn main() -> Result<()> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .build(&event_loop)
        .unwrap();

    let state_w = block_on(ShaderState::new(&window, prepare_objects));

    let mut state = match state_w {
        Ok(s) => s,
        Err(e) => {
            return Err(e);
        },
    };

    let mut last_render_time = std::time::Instant::now();
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() && !state.input(event) => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput {
                    input,
                    ..
                } => {
                    match input {
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        _ => (),
                    }
                },
                WindowEvent::Resized(physical_size) => {
                    state.resize(*physical_size);
                },
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    state.resize(**new_inner_size);
                },
                _ => (),
            },
            Event::RedrawRequested(_) => {
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;
                state.update(dt, |_| Ok(())).unwrap();
                state.render();
            },
            Event::MainEventsCleared => {
                window.request_redraw();
            },
            _ => (),
        }
    });

    // Ok(())
}

fn prepare_objects(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout
) -> Result<(Vec<Instance>, Vec<Light>, Vec<Instance>)>
{
    let lights = vec![
        Light::new(0, (-5.0, 10.0, 5.0).into(), (1.0, 1.0, 1.0).into(), 0.4, 1.0),
        Light::new_spotlight(
            1, (0.0, 2.1, 1.2).into(), (1.0, 1.0, 0.0).into(),
            1.0, // intensity
            0.42,
            0.99, // inner
            0.9, // outer
            (0.0, -1.0, 0.0).into()
        ),
    ];

    let assets_dir = std::path::Path::new(env!("OUT_DIR")).join("assets");
    let house2 = Model::load(
        0, device, queue, layout,
        assets_dir.join("house2.obj"),
    )?;
    let house2 = Rc::new(house2);
    
    let house_i = Model::instantiate(
        house2.clone(),
        "house2".to_string(),
        (0.0, 0.0, 0.0).into(),
        cgmath::Quaternion::from_axis_angle(
            cgmath::Vector3::unit_z(),
            cgmath::Deg(0.0)
        ),
        1.0
    );

    let bulb = Model::load(
        1, device, queue, layout,
        assets_dir.join("bulb.obj"),
    )?;
    let bulb = Rc::new(bulb);

    let bulb_i = Model::instantiate(
        bulb.clone(),
        "bulb".to_string(),
        (0.0, 2.08, 1.2).into(),
        cgmath::Quaternion::from_axis_angle(
            cgmath::Vector3::unit_z(),
            cgmath::Deg(0.0)
        ),
        0.042
    );

    Ok((
        vec![house_i],
        lights,
        vec![bulb_i]
    ))
}