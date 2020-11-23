use obj_viewer::shader_settings::ShaderState;

use winit::{
    event::*,
    event_loop::{EventLoop, ControlFlow},
    window::WindowBuilder,
};
use futures::executor::block_on;

use anyhow::*;

fn main() -> Result<()> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .build(&event_loop)
        .unwrap();
    let state_w = block_on(ShaderState::new(
        &window,
    ));

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
                state.update().unwrap();
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

