#[macro_use]
extern crate glium;

extern crate rand;

mod ifs;
mod vertex;
mod gui;

use gui::{MouseState, State, draw_gui};

use std::time::Instant;

#[macro_use]
extern crate imgui;
extern crate imgui_glium_renderer;

use imgui::ImGui;
use imgui_glium_renderer::Renderer;


fn main() {
    use glium::{glutin, Surface};

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new();
    let context = glutin::ContextBuilder::new();
    let display = glium::Display::new(window, context, &events_loop).expect("Failed to initialize display");

    let mut imgui = ImGui::init();
    imgui.set_ini_filename(None);
    let mut renderer = Renderer::init(&mut imgui, &display).expect("Failed to initialize renderer");

    let indices = glium::index::NoIndices(glium::index::PrimitiveType::Points);

    let vertex_shader_src = r#"
        #version 330
        in vec2 position;
        in float hue;
        out float v_hue;

        uniform mat3 transform;

        void main() {
            v_hue = hue;
            vec3 pos = vec3(position, 1.0) * transform;
            gl_Position = vec4(pos.xy, 0.0, 1.0);
        }
    "#;

    let fragment_shader_src = r#"
        #version 330
        in float v_hue;
        out vec4 color;

        vec3 hsv2rgb(vec3 c) {
            vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
            vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
            return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
        }

        void main() {
            color = vec4(hsv2rgb(vec3(1-v_hue, 0.8, 0.8)), 1.0);
        }
    "#;

    let program = glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None).expect("Failed to build program");

    let mut closed = false;
    let mut scale: f32 = 1.0;
    let mut xpos: f32 = 0.0;
    let mut ypos: f32 = 0.0;

    let mut last_frame = Instant::now();
    let mut mouse_state = MouseState::default();
    let mut state = State::default();

    while !closed {
        events_loop.poll_events(|event| {
            use glium::glutin::WindowEvent::*;
            use glium::glutin::ElementState::Pressed;
            use glium::glutin::{MouseButton, MouseScrollDelta, TouchPhase};
            match event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    Closed => closed = true,
                    KeyboardInput { input, device_id: _ } => {
                        if let Some(v) = input.virtual_keycode {
                            match v {
                                // Divide by scale so moving feels uniform
                                glutin::VirtualKeyCode::Up => ypos += 0.05 / scale,
                                glutin::VirtualKeyCode::Down => ypos -= 0.05 / scale,
                                glutin::VirtualKeyCode::Right => xpos += 0.05 / scale,
                                glutin::VirtualKeyCode::Left => xpos -= 0.05 / scale,
                                glutin::VirtualKeyCode::Q => scale *= 1.10,
                                glutin::VirtualKeyCode::Z => scale *= 0.9,
                                _ => ()
                            }
                        }
                    },
                    CursorMoved { position: (x, y), .. } => mouse_state.pos = (x as i32, y as i32),
                    MouseInput { state, button, .. } => {
                        match button {
                            MouseButton::Left => mouse_state.pressed.0 = state == Pressed,
                            MouseButton::Right => mouse_state.pressed.1 = state == Pressed,
                            MouseButton::Middle => mouse_state.pressed.2 = state == Pressed,
                            _ => {}
                        }
                    },
                    MouseWheel {
                        delta: MouseScrollDelta::LineDelta(_, y),
                        phase: TouchPhase::Moved,
                        ..
                    } |
                    MouseWheel {
                        delta: MouseScrollDelta::PixelDelta(_, y),
                        phase: TouchPhase::Moved,
                        ..
                    } => mouse_state.wheel = y,
                    _ => ()
                }
                _ => (),
            }
        });

        let now = Instant::now();
        let delta = now - last_frame;
        let delta_s = delta.as_secs() as f32 + delta.subsec_nanos() as f32 / 1_000_000_000.0;
        last_frame = now;

        mouse_state.update_imgui(&mut imgui);

        // Generate fractal
        let mut sys = state.get_sys();
        let fract = sys.generate(state.num_points as usize);
        let vertex_buffer = glium::VertexBuffer::new(&display, &fract).expect("vertex buffer");
        // Translate/scale matrix
        let transform = [[scale, 0.0, -xpos * scale],
                         [0.0, scale, -ypos * scale],
                         [0.0, 0.0, scale]];

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        target.draw(&vertex_buffer, &indices, &program,
                    &uniform! { transform: transform },
                    &Default::default()).expect("Fractal draw failed");
        // Draw GUI
        let gl_window = display.gl_window();
        let size_points = gl_window.get_inner_size_points().unwrap();
        let size_pixels = gl_window.get_inner_size_pixels().unwrap();
        let ui = imgui.frame(size_points, size_pixels, delta_s);
        draw_gui(&ui, &mut state);
        renderer.render(&mut target, ui).expect("Rendering failed");

        target.finish().unwrap();
    }
}
