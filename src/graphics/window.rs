use crate::graphics::app::App;
use crate::config::Config;
use std::sync::Arc;
use std::sync::RwLock;
use winit::
    event_loop::{ControlFlow, EventLoop};

pub fn display_grid(config: Arc<RwLock<Config>>) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(config);
    event_loop.run_app(&mut app).unwrap();

    Ok(())
}
