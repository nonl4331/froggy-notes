use glyph_brush_layout::{FontId, GlyphPositioner, Layout, SectionGeometry, SectionText};
use image::GenericImageView;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

use ab_glyph::{Font, FontArc, FontRef, Glyph, PxScale, PxScaleFont, point};

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

    let mut world = World::new();

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
                    world.draw(pixels.frame_mut());
                    pixels.render().unwrap();
                }
                _ => (),
            }
            world.update();
            window.request_redraw();
        })
        .unwrap();
}

const BOX_SIZE: i16 = 6;
const FROG_PIXEL_WIDTH: i16 = 5;

struct World {
    box_x: i16,
    box_y: i16,
    velocity_x: i16,
    velocity_y: i16,
    frog: Bitmap,
    font: FontArc,
}

impl World {
    fn new() -> Self {
        let font = ab_glyph::FontArc::try_from_slice(include_bytes!("../res/font3.ttf")).unwrap();

        Self {
            box_x: 24,
            box_y: 16,
            velocity_x: 1,
            velocity_y: 1,
            frog: Bitmap::new("res/frog1.png"),
            font,
        }
    }

    fn update(&mut self) {
        if self.box_x <= 0 || self.box_x + BOX_SIZE > WIDTH as i16 {
            self.velocity_x *= -1;
        }
        if self.box_y <= 0 || self.box_y + BOX_SIZE > HEIGHT as i16 {
            self.velocity_y *= -1;
        }

        self.box_x += self.velocity_x;
        self.box_y += self.velocity_y;
    }
    fn draw(&self, frame: &mut [u8]) {
        // box layer
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let x = (i % WIDTH as usize) as i16;
            let y = (i / WIDTH as usize) as i16;

            let inside_the_box = x >= self.box_x
                && x < self.box_x + BOX_SIZE
                && y >= self.box_y
                && y < self.box_y + BOX_SIZE;

            let rgba = if inside_the_box {
                [0x5e, 0x48, 0xe8, 0xff]
            } else {
                [50, 205, 50, 255]
            };

            pixel.copy_from_slice(&rgba);
        }
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let x = (i % WIDTH as usize) as i16 / FROG_PIXEL_WIDTH;
            let y = (i / WIDTH as usize) as i16 / FROG_PIXEL_WIDTH;

            if let Some(col) = self.frog.query_pixel(x as u32, y as u32) {
                pixel.copy_from_slice(&col);
            }
        }
        // frog layer
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let x = (i % WIDTH as usize) as i16 / FROG_PIXEL_WIDTH;
            let y = (i / WIDTH as usize) as i16 / FROG_PIXEL_WIDTH;

            if let Some(col) = self.frog.query_pixel(x as u32, y as u32) {
                pixel.copy_from_slice(&col);
            }
        }
        const TEXT_PIXELS_WIDE: usize = 400;
        const TEXT_PIXELS_HIGH: usize = 100;
        let mut font_buffer = vec![[0u8; 4]; TEXT_PIXELS_HIGH * TEXT_PIXELS_WIDE];

        // text layer
        let test_text = "abfdnajkl;1$@!$789 124";
        let glyphs = Layout::default().calculate_glyphs(&[&self.font], &SectionGeometry {
            screen_position: (0.0, 0.0),
            ..SectionGeometry::default()
            },
            &[
                SectionText {
                    text: test_text,
                    scale: self.font.pt_to_px_scale(24.0).unwrap(),
                    font_id: FontId(0),
                },
            ]

        );

        for glyph in glyphs {
            if let Some(outline) = self.font.outline_glyph(glyph.glyph) {
                let bb = outline.px_bounds();
                let posx = bb.min.x as u32;
                let posy = bb.min.y as u32;
                outline.draw(|x, y, v| {
                    if v > 0.0 {
                        let x = posx + x;
                        let y = posy + y;
                        if (x as usize) < TEXT_PIXELS_WIDE && (y as usize) < TEXT_PIXELS_HIGH {
                            font_buffer[x as usize + y as usize * TEXT_PIXELS_WIDE] = [(255.0) as u8; 4];
                        }
                    }
                });
            }
        }

        const TEXT_PIXEL_SIZE: usize = 1;
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

struct Bitmap {
    x: u32,
    y: u32,
    data: Vec<[u8; 4]>,
}

impl Bitmap {
    pub fn new(f: &str) -> Self {
        let img = image::open(f).unwrap();
        let (x, y) = img.dimensions();
        let data: Vec<[u8; 4]> = img
            .into_rgba8()
            .into_vec()
            .chunks_exact(4)
            .map(|c| <[u8; 4]>::try_from(c).unwrap())
            .collect();
        Self { x, y, data }
    }
    pub fn query_pixel(&self, px: u32, py: u32) -> Option<[u8; 4]> {
        if px >= self.x || py >= self.y {
            return None;
        }
        let pixel = self.data[(px + self.x * py) as usize];
        if pixel[3] == 0 {
            return None;
        }
        Some(pixel)
    }
}
