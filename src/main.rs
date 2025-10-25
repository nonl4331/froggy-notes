use std::time::Instant;

use image::GenericImageView;
use winit::{
    event::{DeviceEvent, ElementState, Event, WindowEvent},
    event_loop::EventLoop,
    keyboard::{Key, NamedKey},
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
                    event: WindowEvent::KeyboardInput { event, .. },
                    ..
                } if event.state == ElementState::Pressed => match event.logical_key {
                    Key::Character(c) => {
                        if let Ok(c) = c.parse() {
                            note.change_cursor(TextCursorAction::AddCharacter(c));
                        }
                    }
                    Key::Named(NamedKey::Backspace) => {
                        note.change_cursor(TextCursorAction::BackspaceCharacter)
                    }
                    Key::Named(NamedKey::Delete) => {
                        note.change_cursor(TextCursorAction::DeleteCharacter)
                    }
                    Key::Named(NamedKey::Escape) => note.change_cursor(TextCursorAction::Unselect),
                    Key::Named(NamedKey::ArrowRight) => {
                        note.change_cursor(TextCursorAction::MoveRight)
                    }
                    Key::Named(NamedKey::ArrowLeft) => {
                        note.change_cursor(TextCursorAction::MoveLeft)
                    }
                    _ => {}
                },
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

#[derive(Debug)]
struct Statement {
    text: String,
    bounding_box: BoundingBox,
    col: [u8; 4],
}

const TEXT_PIXEL_SIZE: usize = 1;
const TEXT_PIXEL_OFFSET_X: usize = 40;
const TEXT_PIXEL_OFFSET_RIGHT: usize = 5;
const TEXT_PIXEL_OFFSET_BOTTOM: usize = 5;
const TEXT_PIXEL_OFFSET_Y: usize = 40;

const TEXT_PIXELS_WIDE: usize =
    (WIDTH as usize - TEXT_PIXEL_OFFSET_X - TEXT_PIXEL_OFFSET_RIGHT) / TEXT_PIXEL_SIZE;
const TEXT_PIXELS_HIGH: usize =
    (HEIGHT as usize - TEXT_PIXEL_OFFSET_Y - TEXT_PIXEL_OFFSET_BOTTOM) / TEXT_PIXEL_SIZE;
const LINE_SPACING: i32 = 15;

const DEMO_TEXT: &str = "Write something here..............AAAAAAAAAAAAAAAAAAAAAA";
const CURSOR_TEXT: &str = "Write something here...";
impl Statement {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            bounding_box: BoundingBox {
                size: Coord::new(0, 0),
                offset: Coord::new(0, 0),
            },
            col: [255; 4],
        }
    }
    pub fn render_into(
        &mut self,
        font_buffer: &mut [[u8; 4]],
        cursor: &mut Coord,
        mut text_cursor: TextCursor,
        font: &BdfFont,
    ) {
        // fall back to demo text if statement is empty
        let mut render_text = &self.text[..];
        self.col = [255; 4];
        if self.text.is_empty() && text_cursor == TextCursor::None {
            render_text = DEMO_TEXT;
            self.col = [127, 127, 127, 255];
        } else if self.text.is_empty() {
            render_text = " ";
        }

        let mut chars = render_text.chars();
        let mut nc = chars.next();
        let mut statement = BoundingBox {
            size: Coord::new(0, 0),
            offset: *cursor,
        };
        let mut lc = *cursor;
        let mut char_idx = 0;

        // render each character
        while let Some(c) = nc {
            let g = font.glyphs.get(c).unwrap();
            let bb = g.bounding_box;

            // wrap to newline if would overflow
            if TextCursor::InText(char_idx) == text_cursor {
                let bb = font.glyphs.get(' ').unwrap().bounding_box;
                if lc.x + bb.offset.x + bb.size.x >= TEXT_PIXELS_WIDE as i32 {
                    lc.x = 0;
                    lc.y += bb.size.y;
                    continue;
                }
            }
            if lc.x + bb.offset.x + bb.size.x >= TEXT_PIXELS_WIDE as i32 {
                lc.x = 0;
                lc.y += bb.size.y;
                continue;
            }

            // write pixels to framebuffer if they fit
            for i in 0..(bb.size.x * bb.size.y) as usize {
                let x = i % bb.size.x as usize;
                let y = i / bb.size.x as usize;
                if g.pixel(x, y) {
                    let x = x as i32 + lc.x + bb.offset.x;
                    let y = y as i32 + lc.y + bb.offset.y;
                    if x >= 0
                        && y >= 0
                        && (x as usize) < TEXT_PIXELS_WIDE
                        && (y as usize) < TEXT_PIXELS_HIGH
                    {
                        font_buffer[x as usize + y as usize * TEXT_PIXELS_WIDE] = self.col;
                    }
                }
            }

            // place text cursor inline with character
            if TextCursor::InText(char_idx) == text_cursor {
                nc = Some('_');
                char_idx += 1;
                continue;
            }
            char_idx += 1;

            // expand bounding box of statement and move cursor
            lc.x += g.device_width.x;
            statement.size.x = statement.size.x.max(lc.x - cursor.x);
            statement.size.y = statement.size.y.max(lc.y - cursor.y);

            if let Some(new_c) = chars.next() {
                nc = Some(new_c);
            } else {
                // potentially add cursor to end of line
                if text_cursor == TextCursor::End {
                    nc = Some('_');
                    text_cursor = TextCursor::None;
                    continue;
                }
                // if move cursor down a line so next statement is on newline
                lc.x = 0;
                lc.y += bb.size.y;
                nc = None;
            }
        }
        cursor.y = lc.y + LINE_SPACING;
    }
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextCursor {
    InText(usize),
    End,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TextCursorAction {
    MoveLeft,
    MoveRight,
    Select(f32, f32),
    Unselect,
    AddCharacter(char),
    BackspaceCharacter,
    DeleteCharacter,
}

struct Note {
    start: Instant,
    frog: Frog,
    font: BdfFont,
    statements: Vec<Statement>,
    // statement index, character index
    text_cursor: (usize, TextCursor),
}

impl Note {
    fn new() -> Self {
        let font = bdf_parser::BdfFont::parse(include_bytes!("../res/Tamzen7x14r.bdf")).unwrap();

        let mut statements = vec![Statement::new()];
        Self {
            frog: Frog::new(),
            start: Instant::now(),
            font,
            statements,
            text_cursor: (0, TextCursor::End),
        }
    }
    fn change_cursor(&mut self, action: TextCursorAction) {
        let statement = &mut self.statements[self.text_cursor.0];
        match (action, self.text_cursor.1) {
            (TextCursorAction::MoveLeft, TextCursor::InText(idx)) if idx > 0 => {
                self.text_cursor.1 = TextCursor::InText(idx - 1);
            }
            (TextCursorAction::MoveLeft, TextCursor::End) if statement.text.len() > 0 => {
                self.text_cursor.1 = TextCursor::InText(statement.text.len() - 1);
            }

            (TextCursorAction::MoveRight, TextCursor::InText(idx))
                if idx + 1 < statement.text.len() =>
            {
                self.text_cursor.1 = TextCursor::InText(idx + 1);
            }
            (TextCursorAction::MoveRight, TextCursor::InText(_)) => {
                self.text_cursor.1 = TextCursor::End;
            }
            (TextCursorAction::Select(_, _), _) => {
                self.text_cursor.0 = 0;
            }
            (TextCursorAction::Unselect, _) => self.text_cursor.1 = TextCursor::None,
            (TextCursorAction::BackspaceCharacter, TextCursor::InText(idx)) if idx > 0 => {
                statement.text.remove(idx - 1);
                self.text_cursor.1 = TextCursor::InText(idx - 1);
            }
            (TextCursorAction::BackspaceCharacter, TextCursor::End) if statement.text.len() > 0 => {
                statement.text.remove(statement.text.len() - 1);
            }
            (TextCursorAction::AddCharacter(c), TextCursor::InText(idx)) => {
                statement.text.insert(idx, c);
            }
            (TextCursorAction::AddCharacter(c), TextCursor::End) => {
                statement.text.push(c);
            }
            (TextCursorAction::DeleteCharacter, TextCursor::InText(idx))
                if idx < statement.text.len() =>
            {
                statement.text.remove(idx);
                if idx == statement.text.len() {
                    self.text_cursor.1 = TextCursor::End;
                }
            }
            _ => {}
        }

        // handle delation of statements
        if statement.text.is_empty() {
            // change cursor state to make it more predictable
            self.text_cursor.1 = TextCursor::End;

            // remove statement if it isn't the last one
            if self.statements.len() > 1 {
                self.statements.remove(self.text_cursor.0);
                // move cursor up to the next statement above
                if self.text_cursor.0 > 0 {
                    self.text_cursor.0 -= 1;
                } else if !self.statements[0].text.is_empty() {
                    // move it the start of the statement if we move statement ontop of
                    // cursor
                    self.text_cursor.1 = TextCursor::InText(0);
                }
            }
        }
        // make cursor more predictable
        let statement = &self.statements[self.text_cursor.0];
        if self.text_cursor.1 == TextCursor::InText(0) && statement.text.is_empty() {
            self.text_cursor.1 = TextCursor::End;
        }
    }
    fn update(&mut self) {
        let t = self.start.elapsed().as_secs_f32();
        self.frog.position = [0.5 + 0.4 * t.sin(), 0.5 + 0.4 * t.cos()];
    }
    fn draw(&mut self, frame: &mut [u8]) {
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
        let mut font_buffer = vec![[0u8; 4]; TEXT_PIXELS_HIGH * TEXT_PIXELS_WIDE];

        // text layer
        let mut cursor = Coord::new(0, 1);
        for (i, statement) in self.statements.iter_mut().enumerate() {
            let mut tc = TextCursor::None;
            if i == self.text_cursor.0 {
                tc = self.text_cursor.1;
            }
            statement.render_into(&mut font_buffer, &mut cursor, tc, &self.font);
        }

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
        Self {
            x: x as usize,
            y: y as usize,
            data,
        }
    }
}
