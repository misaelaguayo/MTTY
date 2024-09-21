use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::TextureQuery;
use std::path::Path;

use crate::config::Config;

pub struct Terminal {
    pub frontend: Box<dyn Frontend>,
    pub backend: State,
}

pub struct State {
    commands: Vec<String>,
}

pub trait Frontend {
    fn r#type(&mut self, text: &str);
    fn poll_event(&mut self);
}

pub struct Sdl2TerminalFrontend {
    pub config: Config,
    pub buffer: Vec<String>,
    pub canvas: sdl2::render::Canvas<sdl2::video::Window>,
    pub sdl_context: sdl2::Sdl,
}

impl Sdl2TerminalFrontend {
    pub fn build(config: Config) -> Sdl2TerminalFrontend {
        let sdl_context = sdl2::init().unwrap();
        let video_subsys = sdl_context.video().unwrap();
        let window = video_subsys
            .window("MTTY", 800, 600)
            .position_centered()
            .opengl()
            .build()
            .unwrap();
        let canvas = window
            .into_canvas()
            .build()
            .map_err(|e| e.to_string())
            .unwrap();
        Sdl2TerminalFrontend {
            canvas,
            sdl_context,
            buffer: Vec::new(),
            config,
        }
    }
}

impl Frontend for Sdl2TerminalFrontend {
    fn r#type(&mut self, text: &str) {
        let config = &self.config;
        let font_path = Path::new(&config.font_path);

        self.buffer.push(text.to_string());
        let buffer_string = self.buffer.join("");
        let texture_creator = self.canvas.texture_creator();
        let binding = sdl2::ttf::init().unwrap();
        let mut font = binding.load_font(font_path, 128).unwrap();
        font.set_style(sdl2::ttf::FontStyle::NORMAL);
        let surface = font
            .render(&buffer_string)
            .blended(Color::RGBA(255, 255, 255, 255))
            .map_err(|e| e.to_string())
            .unwrap();
        let texture = texture_creator
            .create_texture_from_surface(&surface)
            .map_err(|e| e.to_string())
            .unwrap();

        self.canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
        self.canvas.clear();

        let TextureQuery { width, height, .. } = texture.query();

        let padding = 64;
        let target = get_centered_rect(
            width,
            height,
            config.screen_width - padding,
            config.screen_height - padding,
            config.screen_width,
            config.screen_height,
        );

        self.canvas.copy(&texture, None, Some(target)).unwrap();
        self.canvas.present();
    }

    fn poll_event(&mut self) {
        let mut event_pump = self.sdl_context.event_pump().unwrap();
        'mainloop: loop {
            for event in event_pump.poll_iter() {
                match event {
                    Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    }
                    | Event::Quit { .. } => break 'mainloop,
                    // TODO: Handle all other keycodes
                    Event::KeyDown { keycode, .. } => {
                        keycode.map(|keycode| {
                            let key = keycode.to_string();
                            if key.len() == 1 {
                                self.r#type(&key);
                            }
                        });
                    }
                    _ => {}
                }
            }
        }
    }
}

impl Terminal {
    pub fn build(config: Config) -> Terminal {
        Terminal {
            frontend: Box::new(Sdl2TerminalFrontend::build(config)),
            backend: State {
                commands: Vec::new(),
            },
        }
    }

    pub fn run(&mut self) {
        self.frontend.poll_event();
    }
}

// handle the annoying Rect i32
macro_rules! rect(
    ($x:expr, $y:expr, $w:expr, $h:expr) => (
        Rect::new($x as i32, $y as i32, $w as u32, $h as u32)
    )
);

// Scale fonts to a reasonable size when they're too big (though they might look less smooth)
fn get_centered_rect(
    rect_width: u32, 
    rect_height: u32, 
    cons_width: u32, 
    cons_height: u32,
    screen_width: u32,
    screen_height: u32) -> Rect {
    let wr = rect_width as f32 / cons_width as f32;
    let hr = rect_height as f32 / cons_height as f32;

    let (w, h) = if wr > 1f32 || hr > 1f32 {
        if wr > hr {
            println!("Scaling down! The text will look worse!");
            let h = (rect_height as f32 / wr) as i32;
            (cons_width as i32, h)
        } else {
            println!("Scaling down! The text will look worse!");
            let w = (rect_width as f32 / hr) as i32;
            (w, cons_height as i32)
        }
    } else {
        (rect_width as i32, rect_height as i32)
    };

    let cx = (screen_width as i32 - w) / 2;
    let cy = (screen_height as i32 - h) / 2;
    rect!(cx, cy, w, h)
}
