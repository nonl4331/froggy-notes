use std::time::Instant;

use image::GenericImageView;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

use bdf_parser::*;

use pixels::{Pixels, SurfaceTexture};

const WIDTH: u32 = 324;
const HEIGHT: u32 = 324;

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("Froggy Notes")
        .with_inner_size(winit::dpi::LogicalSize::new(WIDTH, HEIGHT))
        .with_min_inner_size(winit::dpi::LogicalSize::new(WIDTH, HEIGHT))
        .with_max_inner_size(winit::dpi::LogicalSize::new(WIDTH, HEIGHT))
        .with_decorations(false)
        .build(&event_loop)
        .unwrap();

    let mut note = Note::new();

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture).unwrap()
    };

    event_loop
        .run(|event, elwt| {
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    println!("The close button was pressed; stopping");
                    elwt.exit();
                }
                Event::AboutToWait => {
                    window.request_redraw();
                }
                Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => {
                    note.draw(pixels.frame_mut());
                    pixels.render().unwrap();
                }
                _ => (),
            }
            note.update();
            window.request_redraw();
        })
        .unwrap();
}

const FROG_PIXEL_WIDTH: i16 = 5;

struct Note {
    start: Instant,
    frog: Frog,
    font: BdfFont,
}

impl Note {
    fn new() -> Self {
        let font = bdf_parser::BdfFont::parse(include_bytes!("../res/Tamzen7x14r.bdf")).unwrap();

        Self {
            frog: Frog::new(),
            start: Instant::now(),
            font,
        }
    }

    fn update(&mut self) {
        let t = self.start.elapsed().as_secs_f32();
        self.frog.position = [0.5 + 0.4 * t.sin(), 0.5 + 0.4 * t.cos()];
    }
    fn draw(&self, frame: &mut [u8]) {
        // background
        for pixel in frame.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[50, 205, 50, 255]);
        }

        // frog layer
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let x = (i % WIDTH as usize) as f32 / WIDTH as f32;
            let y = (i / WIDTH as usize) as f32 / HEIGHT as f32;

            if let Some(col) = self.frog.query_uv(x, y) {
                pixel.copy_from_slice(&col);
            }
        }
        const TEXT_PIXELS_WIDE: usize = 400;
        const TEXT_PIXELS_HIGH: usize = 100;
        let mut font_buffer = vec![[0u8; 4]; TEXT_PIXELS_HIGH * TEXT_PIXELS_WIDE];

        // text layer
        let test_text = "Hello World!";
        let mut cursor = (0, 1);
        for c in test_text.chars() {
            let g = self.font.glyphs.get(c).unwrap();
            let bb = g.bounding_box;
            for i in 0..(bb.size.x * bb.size.y) as usize {
                let x = i % bb.size.x as usize;
                let y = i / bb.size.x as usize;
                if g.pixel(x, y) {
                    let x = x as i32 + cursor.0 + bb.offset.x;
                    let y = y as i32 + cursor.1 + bb.offset.y;
                    if x >= 0
                        && y >= 0
                        && (x as usize) < TEXT_PIXELS_WIDE
                        && (y as usize) < TEXT_PIXELS_HIGH
                    {
                        font_buffer[x as usize + y as usize * TEXT_PIXELS_WIDE] = [255; 4];
                    }
                }
            }

            cursor.0 += g.device_width.x;
        }

        const TEXT_PIXEL_SIZE: usize = 2;
        const TEXT_PIXEL_OFFSET_X: usize = 40;
        const TEXT_PIXEL_OFFSET_Y: usize = 40;
        let xy_to_tx_pixel = |x: usize, y: usize| {
            if x < TEXT_PIXEL_OFFSET_X || y < TEXT_PIXEL_OFFSET_Y {
                return None;
            }
            let x = (x - TEXT_PIXEL_OFFSET_X) / TEXT_PIXEL_SIZE;
            let y = (y - TEXT_PIXEL_OFFSET_Y) / TEXT_PIXEL_SIZE;
            if x < TEXT_PIXELS_WIDE && y < TEXT_PIXELS_HIGH {
                return Some(font_buffer[x as usize + y as usize * TEXT_PIXELS_WIDE]);
            }
            None
        };
        // draw text layer
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let x = (i % WIDTH as usize) as i16;
            let y = (i / WIDTH as usize) as i16;

            if let Some(col) = xy_to_tx_pixel(x as usize, y as usize) {
                if col[3] != 0 {
                    pixel.copy_from_slice(&col);
                }
            }
        }
    }
}

struct Frog {
    frames: Vec<Bitmap>,
    // UV coordinates
    pub position: [f32; 2],
    frame_num: usize,
    facing_right: bool,
}

impl Frog {
    pub fn new() -> Self {
        let frames = vec![Bitmap::new_from_bytes(include_bytes!("../res/frog1.png"))];
        Self {
            frames,
            position: [0.0, 0.0],
            frame_num: 0,
            facing_right: false,
        }
    }
    pub fn query_uv(&self, u: f32, v: f32) -> Option<[u8; 4]> {

        // move to position
        let mut u = u - self.position[0];
        let mut v = v - self.position[1];
        let frame = &self.frames[self.frame_num];

        let u_size = FROG_PIXEL_WIDTH as f32 * frame.x as f32 / WIDTH as f32;
        let v_size = FROG_PIXEL_WIDTH as f32 * frame.y as f32 / HEIGHT as f32;

        // position relative to centre of frog
        u += 0.5 * u_size;
        v += 0.5 * v_size;

        // scale correctly
        u /= u_size;
        v /= v_size;

        // flip
        if self.facing_right {
            u = 1.0 - u;
        }

        if !((0.0..1.0).contains(&u) && (0.0..1.0).contains(&v)) {
            return None;
        }

        // get pixel
        let u = (u * frame.x as f32) as usize;
        let v = (v * frame.y as f32) as usize;

        let pixel = frame.data[u + frame.x * v];
        if pixel[3] == 0 {
            return None;
        }
        Some(pixel)
    }
}

struct Bitmap {
    pub x: usize,
    pub y: usize,
    pub data: Vec<[u8; 4]>,
}

impl Bitmap {
    pub fn new_from_bytes(v: &[u8]) -> Self {
        let img = image::load_from_memory(v).unwrap();
        let (x, y) = img.dimensions();
        let data: Vec<[u8; 4]> = img
            .into_rgba8()
            .into_vec()
            .chunks_exact(4)
            .map(|c| <[u8; 4]>::try_from(c).unwrap())
            .collect();
        Self { x: x as usize, y: y as usize, data }
    }
}
