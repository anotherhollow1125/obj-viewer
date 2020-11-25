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
                {
                    let mut dice1 = state.instance_book
                        .get("dice1")
                        .unwrap()
                        .borrow_mut();
                    dice1.rotation = cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_y(),
                        cgmath::Deg(1.0),
                    ) * dice1.rotation;

                    let mut dice2 = state.instance_book
                        .get("dice2")
                        .unwrap()
                        .borrow_mut();
                    dice2.rotation = cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_z(),
                        cgmath::Deg(1.0),
                    ) * dice2.rotation;

                    let mut light = state.light_book[0].borrow_mut();
                    let p = cgmath::Quaternion::from_axis_angle(
                            (0.0, 1.0, 0.0).into(),
                            cgmath::Deg(1.0)
                        ) * light.position;
                    light.position = p;
                    let mut white_light = state.instance_book
                        .get("white_light")
                        .unwrap()
                        .borrow_mut();
                    white_light.position = p;
                }
                state.update(|s| {
                    let dice1 = s.instance_book
                        .get("dice1")
                        .unwrap()
                        .borrow();
                    s.instance_setting
                        .update_instance(&s.queue, &dice1)?;
                    let dice2 = s.instance_book
                        .get("dice2")
                        .unwrap()
                        .borrow();
                    s.instance_setting
                        .update_instance(&s.queue, &dice2)?;
                    let light = s.light_book[0].borrow();
                    s.light_setting
                        .update_light(&s.queue, &light);
                    let white_light = s.instance_book
                        .get("white_light")
                        .unwrap()
                        .borrow();
                    s.light_instance_setting
                        .update_instance(&s.queue, &white_light)?;

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
    layout: &wgpu::BindGroupLayout
) -> Result<(Vec<Instance>, Vec<Light>, Vec<Instance>)>
{
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

    let assets_dir = std::path::Path::new(env!("OUT_DIR")).join("assets");
    let dice2 = Model::load(
        0,
        device,
        queue,
        layout,
        assets_dir.join("dice2.obj"),
    )?;

    let dice2 = Rc::new(dice2);
    
    let instance_1 = Model::instantiate(
        dice2.clone(),
        "dice1".to_string(),
        (1.0, 0.0, 0.0).into(),
        cgmath::Quaternion::from_axis_angle(
            cgmath::Vector3::unit_z(),
            cgmath::Deg(0.0)
        ),
        1.0
    );
    
    let instance_2 = Model::instantiate(
        dice2.clone(),
        "dice2".to_string(),
        (-1.0, 0.0, 0.0).into(),
        cgmath::Quaternion::from_axis_angle(
            cgmath::Vector3::unit_z(),
            cgmath::Deg(0.0)
        ),
        0.7
    );

    let light_model_1 = Model::load(
        2,
        device,
        queue,
        layout,
        assets_dir.join("white_cube.obj"),
    )?;
    let light_model_1 = Rc::new(light_model_1);
    
    let light_instance_1 = Model::instantiate(
        light_model_1.clone(),
        "white_light".to_string(),
        lights[0].position,
        cgmath::Quaternion::from_axis_angle(
            cgmath::Vector3::unit_z(),
            cgmath::Deg(0.0)
        ),
        0.2
    );
    
    let light_model_2 = Model::load(
        3,
        device,
        queue,
        layout,
        assets_dir.join("red_cube.obj"),
    )?;
    let light_model_2 = Rc::new(light_model_2);
    
    let light_instance_2 = Model::instantiate(
        light_model_2.clone(),
        "red_light".to_string(),
        lights[1].position,
        cgmath::Quaternion::from_axis_angle(
            cgmath::Vector3::unit_z(),
            cgmath::Deg(0.0)
        ),
        0.2
    );

    Ok((
        vec![instance_1, instance_2],
        lights,
        vec![light_instance_1, light_instance_2]
    ))
}