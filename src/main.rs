use obj_viewer::shader_settings::{
    model,
    light,
    ShaderState,
    shadowmap,
    camera,
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
        .with_title("Obj File Viewer: Snow theme.")
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
                state.update(dt, |s| {

                    let mut main_light = s.light_book[0].borrow_mut();
                    let old_position = main_light.position;
                    main_light.position =
                        cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(0.1))
                        * old_position;
                    s.light_buffer.update_light(&s.queue, &main_light);

                    let pos = main_light.position;
                    main_light.shadow.update(
                        Some(pos),
                        None,
                        &s.queue,
                        &mut s.shadow_uniform_buffer,
                    );

                    drop(main_light);

                    let mut sub_light = s.light_book[2].borrow_mut();
                    let old_position = sub_light.position;
                    sub_light.position =
                        cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(0.1))
                        * old_position;
                    s.light_buffer.update_light(&s.queue, &sub_light);

                    let pos = sub_light.position;
                    sub_light.shadow.update(
                        Some(pos),
                        None,
                        &s.queue,
                        &mut s.shadow_uniform_buffer,
                    );

                    /*
                    s.shadowmap.position = cgmath::Point3::from_vec(main_light.position);
                    s.shadowmap.direction = -main_light.position.normalize();
                    s.shadowmap.update_view_proj(&s.queue);
                    */

                    Ok(())
                }).unwrap();
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
    sc_desc: &wgpu::SwapChainDescriptor,
    texture_layout: &wgpu::BindGroupLayout,
    instance_layout: &wgpu::BindGroupLayout,
    shadow_texture: &wgpu::Texture,
) -> Result<(Vec<Instance>, Vec<Light>, Vec<Instance>)>
{
    use shadowmap::DirUpdateWay;
    use camera::Projection;

    let pos1 = (-5.0, 10.0, 5.0);
    let pos2 = (0.0, 2.1, 1.2);

    let shadow_1 = shadowmap::ShadowMap::new(
        0,
        pos1.into(),
        (0.0, 0.0, 0.0).into(), // don't use
        0.5,
        DirUpdateWay::SunLight {
            anchor_pos: (0.0, 0.0, 0.0).into(),
        },
        Projection::new(sc_desc.width, sc_desc.height, cgmath::Deg(45.0), 0.1, 100.0),
        device,
        queue,
        sc_desc,
        instance_layout,
        shadow_texture,
    );

    let spot_light_dir = (0.0, -1.0, 0.0);
    let shadow_2 = shadowmap::ShadowMap::new(
        1,
        pos2.into(),
        spot_light_dir.into(),
        0.0,
        DirUpdateWay::SpotLight,
        Projection::new(sc_desc.width, sc_desc.height, cgmath::Deg(120.0), 0.1, 100.0),
        device,
        queue,
        sc_desc,
        instance_layout,
        shadow_texture,
    );

    let pos3 = (-5.0, 10.0, -5.0);
    let shadow_3 = shadowmap::ShadowMap::new(
        2,
        pos3.into(),
        (0.0, 0.0, 0.0).into(), // don't use
        0.5,
        DirUpdateWay::SunLight {
            anchor_pos: (0.0, 0.0, 0.0).into(),
        },
        Projection::new(sc_desc.width, sc_desc.height, cgmath::Deg(45.0), 0.1, 100.0),
        device,
        queue,
        sc_desc,
        instance_layout,
        shadow_texture,
    );

    let lights = vec![
        Light::new(0, pos1.into(), (1.0, 1.0, 1.0).into(), 0.4, 1.0, shadow_1),
        Light::new_spotlight(
            1, pos2.into(), (1.0, 1.0, 0.0).into(),
            1.0, // intensity
            0.42,
            0.99, // inner
            0.85, // outer
            spot_light_dir.into(),
            shadow_2
        ),
        Light::new(2, pos3.into(), (0.0, 0.0, 1.0).into(), 0.4, 1.0, shadow_3),
    ];

    let assets_dir = std::path::Path::new(env!("OUT_DIR")).join("assets");
    let house2 = Model::load(
        0, device, queue, texture_layout,
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
        1, device, queue, texture_layout,
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