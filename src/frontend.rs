use crate::config::Config;
use crate::term::Command;
use crossbeam::channel::{Receiver, Sender};
use font_kit::handle::Handle;
use font_kit::source::SystemSource;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::TextureQuery;
use sdl2::{rwops, VideoSubsystem};
use uuid::Uuid;

// handle the annoying Rect i32
macro_rules! rect(
    ($x:expr, $y:expr, $w:expr, $h:expr) => (
        Rect::new($x as i32, $y as i32, $w as u32, $h as u32)
    )
);

pub trait Frontend {
    fn r#type(&mut self, text: String);
    fn poll_event(&mut self);
}

pub struct Sdl2TerminalFrontend {
    pub config: Config,
    pub buffer: Vec<char>,
    // TODO: change to a hashmap
    pub history: Vec<Command>,
    pub canvas: sdl2::render::Canvas<sdl2::video::Window>,
    pub sdl_context: sdl2::Sdl,
    pub receiver: Receiver<Command>,
    pub sender: Sender<Command>,
    pub video_subsys: VideoSubsystem,
}

impl Sdl2TerminalFrontend {
    pub fn build(
        config: Config,
        sender: Sender<Command>,
        receiver: Receiver<Command>,
    ) -> Sdl2TerminalFrontend {
        let history: Vec<Command> = Vec::new();
        let sdl_context = sdl2::init().unwrap();
        let video_subsys = sdl_context.video().unwrap();
        let mut window = video_subsys
            .window("MTTY", config.screen_width, config.screen_height)
            .position_centered()
            .opengl()
            .build()
            .unwrap();
        window.set_opacity(config.transparency).unwrap();
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
            sender,
            receiver,
            history,
            video_subsys,
        }
    }
}

impl Frontend for Sdl2TerminalFrontend {
    fn r#type(&mut self, text: String) {
        if text == "Backspace" {
            if self.buffer.len() > 0 {
                self.buffer.pop();
            }
            return;
        }

        if text == "Space" {
            self.buffer.push(' ');
            return;
        }

        if text == "Return" {
            let command = Command {
                id: Uuid::new_v4(),
                command: self.buffer.iter().collect(),
                args: vec![], // TODO: parse args from command
                response: Vec::new(),
            };

            self.buffer.clear();
            self.sender.send(command.clone()).unwrap();
            self.history.push(command);

            return;
        }

        for c in text.chars() {
            self.buffer.push(c);
        }
    }

    fn poll_event(&mut self) {
        let config = &self.config.clone();
        let font_family = SystemSource::new()
            .select_family_by_name(&config.font)
            .unwrap();

        let mut rwops = rwops::RWops::from_bytes(&[]);

        for font in font_family.fonts() {
            match font {
                Handle::Path { path, .. } => {
                    rwops = Ok(rwops::RWops::from_file(path, "r").unwrap());
                    break;
                }
                Handle::Memory { bytes, .. } => {
                    rwops = rwops::RWops::from_bytes(&bytes);
                    break;
                }
            }
        }

        let binding = sdl2::ttf::init().unwrap();
        let texture_creator = self.canvas.texture_creator();
        let mut font = binding.load_font_from_rwops(rwops.unwrap(), config.font_size).unwrap();
        font.set_style(sdl2::ttf::FontStyle::NORMAL);
        font.set_kerning(true);
        self.canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));

        let mut event_pump = self.sdl_context.event_pump().unwrap();
        'mainloop: loop {
            for event in event_pump.poll_iter().collect::<Vec<Event>>() {
                let response = self.receiver.try_recv();

                if let Ok(response) = response {
                    let index = self
                        .history
                        .iter()
                        .position(|x| x.id == response.id)
                        .unwrap();
                    self.history[index].response = response.response;
                }

                let mut history_text = "".to_string();
                for command in self.history.iter() {
                    history_text.push_str(command.command.as_str());
                    history_text.push_str("\n");
                    for response in command.response.iter() {
                        history_text.push_str(response.as_str());
                        history_text.push_str("\n");
                    }
                }

                let buffer_string = self.buffer.iter().collect::<String>();
                let rendered_text = format!("{}>: {}", history_text, buffer_string);

                let surface = font
                    .render(rendered_text.as_str())
                    .blended_wrapped(Color::RGBA(255, 255, 255, 255), config.screen_width)
                    .map_err(|e| e.to_string())
                    .unwrap();
                let texture = texture_creator
                    .create_texture_from_surface(&surface)
                    .map_err(|e| e.to_string())
                    .unwrap();

                self.canvas.clear();

                let TextureQuery { width, height, .. } = texture.query();

                let target = get_text_rect(width, height);

                self.canvas.copy(&texture, None, Some(target)).unwrap();
                self.canvas.present();

                match event {
                    Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    }
                    | Event::Quit { .. } => break 'mainloop,
                    Event::KeyDown { keycode, .. } => {
                        let key_state = event_pump.keyboard_state();

                        if key_state.is_scancode_pressed(Scancode::LGui)
                            && key_state.is_scancode_pressed(Scancode::V)
                        {
                            let text = &self.video_subsys.clipboard().clipboard_text().unwrap();
                            self.r#type(text.to_string());
                        } else if key_state.is_scancode_pressed(Scancode::LCtrl)
                            || key_state.is_scancode_pressed(Scancode::LGui)
                        {
                        } else {
                            self.r#type(keycode.unwrap().to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn get_text_rect(rect_width: u32, rect_height: u32) -> Rect {
    rect!(0, 0, rect_width, rect_height)
}
