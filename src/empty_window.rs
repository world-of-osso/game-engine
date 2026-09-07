use std::num::NonZeroU32;
use std::rc::Rc;

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle};
#[cfg(target_os = "linux")]
use winit::platform::wayland::WindowAttributesExtWayland;
use winit::window::{Window, WindowId};

const TITLE: &str = "game-engine — empty baseline";
const BACKGROUND: u32 = 0x00181818;

pub(crate) fn run(services: Option<bevy::app::App>) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|error| format!("create event loop: {error}"))?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let context = Context::new(event_loop.owned_display_handle())
        .map_err(|error| format!("create softbuffer context: {error}"))?;
    let mut app = EmptyWindowApp::new(context, services);

    event_loop
        .run_app(&mut app)
        .map_err(|error| format!("run event loop: {error}"))?;

    app.error.map_or(Ok(()), Err)
}

struct EmptyWindowApp {
    window: Option<Rc<Window>>,
    surface: Option<Surface<OwnedDisplayHandle, Rc<Window>>>,
    // Declared after `surface` so the surface drops first.
    context: Context<OwnedDisplayHandle>,
    error: Option<String>,
    services: Option<bevy::app::App>,
}

impl EmptyWindowApp {
    fn new(context: Context<OwnedDisplayHandle>, services: Option<bevy::app::App>) -> Self {
        Self {
            window: None,
            surface: None,
            context,
            error: None,
            services,
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let attributes = window_attributes(self.services.is_some());
        let window = Rc::new(
            event_loop
                .create_window(attributes)
                .map_err(|error| format!("create window: {error}"))?,
        );
        let surface = Surface::new(&self.context, Rc::clone(&window))
            .map_err(|error| format!("create softbuffer surface: {error}"))?;

        window.request_redraw();
        self.window = Some(window);
        self.surface = Some(surface);
        Ok(())
    }

    fn paint_background(&mut self) -> Result<(), String> {
        let window = self.window.as_ref().ok_or("paint without a window")?;
        let size = window.inner_size();
        let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return Ok(());
        };
        let surface = self.surface.as_mut().ok_or("paint without a surface")?;

        surface
            .resize(width, height)
            .map_err(|error| format!("resize softbuffer surface: {error}"))?;
        let mut buffer = surface
            .buffer_mut()
            .map_err(|error| format!("acquire softbuffer: {error}"))?;
        buffer.fill(BACKGROUND);
        window.pre_present_notify();
        buffer
            .present()
            .map_err(|error| format!("present softbuffer: {error}"))
    }

    fn exit_with_error(&mut self, event_loop: &ActiveEventLoop, error: String) {
        self.error = Some(error);
        event_loop.exit();
    }
}

impl ApplicationHandler for EmptyWindowApp {
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(app) = &mut self.services {
            app.update();
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            if let Err(error) = self.create_window(event_loop) {
                self.exit_with_error(event_loop, error);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self
            .window
            .as_ref()
            .is_none_or(|window| window.id() != window_id)
        {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.paint_background() {
                    self.exit_with_error(event_loop, error);
                }
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn window_attributes(services: bool) -> winit::window::WindowAttributes {
    let title = if services {
        "game-engine — Bevy core services"
    } else {
        TITLE
    };
    let attributes = Window::default_attributes().with_title(title);

    #[cfg(target_os = "linux")]
    let attributes = attributes.with_name(
        if services {
            "com.worldofosso.game-engine.services"
        } else {
            "com.worldofosso.game-engine.empty"
        },
        "game-engine",
    );

    attributes
}
