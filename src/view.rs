use sdl2::image::INIT_PNG;
use sdl2::pixels::Color;

#[derive(Clone, Debug)]
pub struct RGB {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl RGB {
    pub fn new(r: f64, g: f64, b: f64) -> Self {
        RGB { r, g, b }
    }
    pub fn scale(&self, x: f64) -> RGB {
        RGB {
            r: self.r * x,
            g: self.g * x,
            b: self.b * x,
        }
    }
}

impl From<RGB> for Color {
    fn from(rgb: RGB) -> Self {
        let f = |l: f64| l.min(255.) as u8;
        Color::RGB(f(rgb.r), f(rgb.g), f(rgb.b))
    }
}


pub fn get_window_canvas() -> (sdl2::render::Canvas<sdl2::video::Window>, sdl2::render::TextureCreator<sdl2::video::WindowContext>, sdl2::EventPump) {
    let sdl_context = sdl2::init()
        .unwrap();
    let video_subsystem = sdl_context
        .video()
        .unwrap();
    let _image_context = sdl2::image::init(INIT_PNG).unwrap();
    let window = video_subsystem.window("colours", 800, 600)
        .resizable()
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .unwrap();

    let texture_creator = canvas.texture_creator();

    let events = sdl_context.event_pump().unwrap();
    canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();
    (canvas, texture_creator, events)
}
