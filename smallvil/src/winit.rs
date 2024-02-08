use std::time::Duration;

use smithay::{
    backend::{
        allocator::Fourcc,
        input::{InputEvent, KeyState, KeyboardKeyEvent},
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement, gles::GlesRenderer,
            ExportMem,
        },
        winit::{self, WinitEvent},
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::calloop::EventLoop,
    utils::{Rectangle, Transform},
};
use std::path::Path;

use crate::{CalloopData, Smallvil};

fn save_buffer_to_png(
    renderer: &mut GlesRenderer,
    path: &Path,
    w: i32,
    h: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Saving image");
    let mapping = renderer
        .copy_framebuffer(Rectangle::from_loc_and_size((0, 0), (w, h)), Fourcc::Abgr8888)
        .expect("Failed to map framebuffer");
    let copy = renderer.map_texture(&mapping).expect("Failed to read mapping");
    image::save_buffer(path, copy, w as u32, h as u32, image::ColorType::Rgba8)
        .expect("Failed to save image");
    println!("Saved image");
    Ok(())
}

pub fn init_winit(
    event_loop: &mut EventLoop<CalloopData>,
    data: &mut CalloopData,
) -> Result<(), Box<dyn std::error::Error>> {
    let display_handle = &mut data.display_handle;
    let state = &mut data.state;

    let (mut backend, winit) = winit::init()?;

    let mode = Mode {
        size: backend.window_size(),
        refresh: 60_000,
    };

    let output = Output::new(
        "winit".to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Smithay".into(),
            model: "Winit".into(),
        },
    );
    let _global = output.create_global::<Smallvil>(display_handle);
    output.change_current_state(Some(mode), Some(Transform::Flipped180), None, Some((0, 0).into()));
    output.set_preferred(mode);

    state.space.map_output(&output, (0, 0));

    let mut damage_tracker = OutputDamageTracker::from_output(&output);

    let mut screenshot_requested = false;

    std::env::set_var("WAYLAND_DISPLAY", &state.socket_name);

    event_loop.handle().insert_source(winit, move |event, _, data| {
        let display = &mut data.display_handle;
        let state = &mut data.state;

        match event {
            WinitEvent::Resized { size, .. } => {
                output.change_current_state(
                    Some(Mode {
                        size,
                        refresh: 60_000,
                    }),
                    None,
                    None,
                    None,
                );
            }
            WinitEvent::Input(event) => {
                match event {
                    InputEvent::Keyboard { event, .. } => {
                        if event.key_code() == 4 && event.state() == KeyState::Pressed {
                            println!("Good key event");
                            screenshot_requested = true;
                        }
                    }
                    _ => {}
                };
                state.process_input_event(event);
            }
            WinitEvent::Redraw => {
                let size = backend.window_size();
                let damage = Rectangle::from_loc_and_size((0, 0), size);

                backend.bind().unwrap();
                smithay::desktop::space::render_output::<_, WaylandSurfaceRenderElement<GlesRenderer>, _, _>(
                    &output,
                    backend.renderer(),
                    1.0,
                    0,
                    [&state.space],
                    &[],
                    &mut damage_tracker,
                    [0.1, 0.1, 0.1, 1.0],
                )
                .unwrap();
                if screenshot_requested {
                    let path = Path::new("foo.png");
                    let current_mode = output.current_mode().unwrap();
                    if let Err(e) =
                        save_buffer_to_png(backend.renderer(), path, current_mode.size.w, current_mode.size.h)
                    {
                        eprintln!("Failed to save buffer to 'foo.png': {}", e);
                    }
                    screenshot_requested = false;
                }
                backend.submit(Some(&[damage])).unwrap();

                state.space.elements().for_each(|window| {
                    window.send_frame(
                        &output,
                        state.start_time.elapsed(),
                        Some(Duration::ZERO),
                        |_, _| Some(output.clone()),
                    )
                });

                state.space.refresh();
                state.popups.cleanup();
                let _ = display.flush_clients();

                // Ask for redraw to schedule new frame.
                backend.window().request_redraw();
            }
            WinitEvent::CloseRequested => {
                state.loop_signal.stop();
            }
            _ => (),
        };
    })?;

    Ok(())
}
