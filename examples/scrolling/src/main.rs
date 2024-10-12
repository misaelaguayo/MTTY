use sdl2::{event::Event, keyboard::Keycode, pixels::Color};

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsys = sdl_context.video().unwrap();
    const SCREEN_W: u32 = 800;
    const SCREEN_H: u32 = 600;
    let source: sdl2::rect::Rect = sdl2::rect::Rect::new(0, 0, SCREEN_W/32, SCREEN_H/32);
    let destination: sdl2::rect::Rect = sdl2::rect::Rect::new(0, 0, SCREEN_W-20, SCREEN_H-20);

    let window = video_subsys
        .window("Scrolling", SCREEN_W, SCREEN_H)
        .opengl()
        .build()
        .unwrap();

    let mut canvas = window
        .into_canvas()
        .build()
        .map_err(|e| e.to_string())
        .unwrap();
    let texture_creator = canvas.texture_creator();
    canvas.set_draw_color(Color::RGBA(255, 255, 255, 255));
    canvas.present();
    let mut event_pump = sdl_context.event_pump().expect("Failed to get event pump");

    'mainloop: loop {
        for event in event_pump.poll_iter(){
            match event {
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                }
                | Event::Quit { .. } => break 'mainloop,
                _ => {}
            }
        }
    }
}
