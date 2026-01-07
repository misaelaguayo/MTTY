use std::sync::RwLock;
use std::sync::Arc;
use tokio::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::Size;
use winit::event::WindowEvent;
use winit::event_loop;
use winit::window::Window;

use crate::config::Config;
use crate::graphics::state::State;

pub struct App {
    config: Arc<RwLock<Config>>,
    state: Option<State>,
}

impl App {
    pub fn new(config: Arc<RwLock<Config>>) -> Self {
        Self {
            config,
            state: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &event_loop::ActiveEventLoop) {
        let config = self.config.read().unwrap();
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("MTTY")
                        .with_inner_size(Size::Physical(winit::dpi::PhysicalSize {
                            width: config.width as u32,
                            height: config.height as u32,
                        })),
                )
                .unwrap(),
        );
        let state = pollster::block_on(State::new(window.clone(), Arc::clone(&self.config)));
        self.state = Some(state);

        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let state = self.state.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                state.render();
                state.get_window().request_redraw();
            }
            WindowEvent::Resized(size) => {
                state.pending_resize = Some(size);
                state.last_resize = Instant::now();
                // state.resize(size);
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &event_loop::ActiveEventLoop) {
        const RESIZE_DEBOUNCE_MS: u128 = 100;
        let state = self.state.as_mut().unwrap();
        if let Some(size) = state.pending_resize {
            if state.last_resize.elapsed().as_millis() >= RESIZE_DEBOUNCE_MS {
                state.resize(size);
                state.pending_resize = None;
            }
        }
    }
}
