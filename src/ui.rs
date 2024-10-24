use std::{any::Any, collections::HashMap, path::Path, rc::Rc};

use sdl2::{rect::Rect, render::{Canvas, RenderTarget, TextureCreator}};

use crate::{player::Player, texture};

pub struct UI<'a> {
    pub fonts: FontBank<'a>,
    pub frames: FrameBank<'a>
}

impl<'a> UI<'a> {
    pub fn new<T>(creator: &'a TextureCreator<T>) -> anyhow::Result<Self> {
        anyhow::Ok(Self {
            fonts: FontBank::load_all(creator)?,
            frames: FrameBank::load_all(creator)?
        })
    }

    pub fn update(&mut self, player: &Player) {

    }

    pub fn draw<T: RenderTarget>(&self, canvas: &mut Canvas<T>) {
        self.frames.test.draw(canvas, 25, 25, 400, 300);
        self.fonts.textured.draw_string_at(canvas, &String::from("hello, world .! ,,"), 50.0, 50.0);   
    }
}

pub struct FontBank<'a> {
    test: Font<'a>,
    textured: Font<'a>
}

pub struct FrameBank<'a> {
    test: NineCell<'a>
}

const YUME_FONT_STRING: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .!,-�?§µ";
const YUME_MINIFONT_STRING: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .!,-�?§µ()[]{}<>'\"`+=/\\*|#%^:;";

impl<'a> FontBank<'a> {
    pub fn load_all<T>(creator: &'a TextureCreator<T>) -> anyhow::Result<Self> {
        let yume_string = YUME_FONT_STRING.chars().collect::<Vec<char>>();

        anyhow::Ok(Self {
            test: Font::from_path("res/textures/ui/fonts/menu.png", &yume_string, 6, 10, 10, 2.0, creator)?,
            textured: Font::from_path("res/textures/ui/fonts/textured.png", &yume_string, 12, 20, 10, 1.0, creator)?,
        })
    }
}

impl<'a> FrameBank<'a> {
    pub fn load_all<T>(creator: &'a TextureCreator<T>) -> anyhow::Result<Self> {
        anyhow::Ok(Self {
            test: NineCell::from_file("res/textures/ui/frames/9cell_test.png", creator, 2.0, [(32, 32); 9])?
        })
    }
}

pub struct Font<'a> {
    pub image: texture::Texture<'a>,
    char_width: u32,
    char_height: u32,
    width: u32,
    scale: f32,
    chars_map: HashMap<char, Rect>,
    x_padding: i32,
    y_padding: i32
}

impl<'a> Font<'a> {
    pub fn draw_char_at<T: RenderTarget>(&self, canvas: &mut Canvas<T>, char: char, x: f32, y: f32) {
        if let Some(src) = self.chars_map.get(&char) {
            let dst = Rect::new(x as i32, y as i32, (self.char_width as f32 * self.scale) as u32, (self.char_height as f32 * self.scale) as u32);

            canvas.copy(&self.image.inner, *src, dst).unwrap();
        }
    }

    pub fn draw_string_at<T: RenderTarget>(&self, canvas: &mut Canvas<T>, string: &String, x: f32, y: f32) {
        let mut cur_x = x;

        for char in string.chars() {
            cur_x += (self.char_width as f32 * self.scale) + self.x_padding as f32;
            self.draw_char_at(canvas, char, cur_x, y);
        }
    }

    pub fn from_path<P: AsRef<Path>, T>(path: P, chars: &Vec<char>, char_width: u32, char_height: u32, width: u32, scale: f32, creator: &'a TextureCreator<T>) -> anyhow::Result<Self> {
        let texture = texture::Texture::from_file(path, creator)?;

        let mut chars_map = HashMap::new();
        for (i, char) in chars.iter().enumerate() {
            let x = i as u32 % width;
            let y = i as u32 / width;

            chars_map.insert(*char, 
                Rect::new(x as i32 * char_width as i32, y as i32 * char_height as i32, char_width, char_height)
            );
        }

        anyhow::Ok(Self {
            image: texture,
            char_height,
            char_width,
            width,
            scale,
            chars_map,
            x_padding: 1,
            y_padding: 1
        })
    }
}

pub struct NineCell<'a> {
    pub image: texture::Texture<'a>,

    // Cell sizes
    tl: Rect, cu: Rect, tr: Rect, cl: Rect, mid: Rect, cr: Rect, bl: Rect, cd: Rect, br: Rect,
    scale: f32
}

fn scaled_x(r: Rect, scale: f32) -> u32 {
    (r.w as f32 * scale) as u32
}

fn scaled_y(r: Rect, scale: f32) -> u32 {
    (r.h as f32 * scale) as u32
}

impl<'a> NineCell<'a> {
    pub fn from_file<P: AsRef<Path>, T>(path: P, creator: &'a TextureCreator<T>, scale: f32, cell_sizes: [(u32, u32); 9]) -> anyhow::Result<Self> {
        let texture = texture::Texture::from_file(path, creator)?;

        Ok(Self {
                    image: texture,
                    tl: Rect::new(0, 0, cell_sizes[0].0, cell_sizes[0].1),
                    cu: Rect::new(cell_sizes[0].0 as i32, 0, cell_sizes[1].0, cell_sizes[1].1),
                    tr: Rect::new(cell_sizes[0].0 as i32 + cell_sizes[1].0 as i32, 0, cell_sizes[2].0, cell_sizes[2].1),
                    cl: Rect::new(0, cell_sizes[0].1 as i32, cell_sizes[3].0, cell_sizes[3].1),
                    mid: Rect::new(cell_sizes[3].0 as i32, cell_sizes[1].1 as i32, cell_sizes[4].0, cell_sizes[4].1),
                    cr: Rect::new(cell_sizes[3].0 as i32 + cell_sizes[4].0 as i32, cell_sizes[2].1 as i32, cell_sizes[5].0, cell_sizes[5].1),
                    bl: Rect::new(0, cell_sizes[0].1 as i32 + cell_sizes[3].1 as i32, cell_sizes[6].0, cell_sizes[6].1),
                    cd: Rect::new(cell_sizes[6].0 as i32, cell_sizes[1].1 as i32 + cell_sizes[4].1 as i32, cell_sizes[7].0, cell_sizes[7].1),
                    br: Rect::new(cell_sizes[6].0 as i32 + cell_sizes[7].0 as i32, cell_sizes[2].1 as i32 + cell_sizes[5].1 as i32, cell_sizes[8].0, cell_sizes[8].1),
                    scale
                })
    }

    pub fn draw<T: RenderTarget>(&self, canvas: &mut Canvas<T>, x: i32, y: i32, w: u32, h: u32) {
        let mut mid_width = w - (2 * (self.tl.w as f32 * self.scale) as u32).min(w);
        let mid_x_count = mid_width / scaled_x(self.tl, self.scale);
        mid_width = mid_x_count * scaled_x(self.tl, self.scale);

        let mut mid_height = h - (2 * (self.tl.h as f32 * self.scale) as u32).min(h);
        println!("{}", mid_height);
        let mid_y_count = mid_height / scaled_y(self.tl, self.scale);
        mid_height = mid_y_count * scaled_y(self.tl, self.scale);

        canvas.copy(
            &self.image.inner, 
            self.tl, 
            Rect::new(x, y, (self.tl.w as f32 * self.scale) as u32, (self.tl.h as f32 * self.scale) as u32)
        ).unwrap();
        canvas.copy(
            &self.image.inner, 
            self.tr, 
            Rect::new(x + (self.tl.w as f32 * self.scale) as i32 + mid_width as i32, y, (self.tr.w as f32 * self.scale) as u32, (self.tr.h as f32 * self.scale) as u32)
        ).unwrap();
        canvas.copy(
            &self.image.inner,
            self.bl,
            Rect::new(x, y + (self.tl.h as f32 * self.scale) as i32 + mid_height as i32, scaled_x(self.bl, self.scale), scaled_y(self.bl, self.scale))
        ).unwrap();
        canvas.copy(
            &self.image.inner,
            self.br,
            Rect::new(x + (self.tl.w as f32 * self.scale) as i32 + mid_width as i32, y + (self.tl.h as f32 * self.scale) as i32 + mid_height as i32, scaled_x(self.br, self.scale), scaled_y(self.br, self.scale))
        ).unwrap();

        let mut pos_x = x + (self.tl.w as f32 * self.scale) as i32;
        for _ in 0..mid_x_count {
            canvas.copy(
                &self.image.inner,
                self.cu,
                Rect::new(pos_x, y, scaled_x(self.cu, self.scale), scaled_y(self.cu, self.scale))
            ).unwrap();

            canvas.copy(
                &self.image.inner,
                self.cd,
                Rect::new(pos_x, y + (self.tl.h as f32 * self.scale) as i32 + mid_height as i32, scaled_x(self.cd, self.scale), scaled_y(self.cd, self.scale))
            ).unwrap();

            pos_x += scaled_x(self.cu, self.scale) as i32
        }

        let mut pos_y = y + (self.tl.h as f32 * self.scale) as i32;
        for _ in 0..mid_y_count {
            canvas.copy(
                &self.image.inner,
                self.cl,
                Rect::new(x, pos_y, scaled_x(self.cl, self.scale), scaled_y(self.cl, self.scale))
            ).unwrap();

            canvas.copy(
                &self.image.inner,
                self.cr,
                Rect::new(x + (self.tl.w as f32 * self.scale) as i32 + mid_width as i32, pos_y, scaled_x(self.cr, self.scale), scaled_y(self.cr, self.scale))
            ).unwrap();

            pos_y += scaled_y(self.cl, self.scale) as i32;
        }

        pos_y = y + (self.tl.h as f32 * self.scale) as i32;//  + scaled_y(self.tl, self.scale) as i32;
        for _ in 0..mid_y_count {
            pos_x = x + (self.tl.w as f32 * self.scale) as i32; // + scaled_x(self.tl, self.scale) as i32;
            for _ in 0..mid_x_count {
                canvas.copy(
                    &self.image.inner,
                    self.mid,
                    Rect::new(pos_x, pos_y, scaled_x(self.mid, self.scale), scaled_y(self.mid, self.scale))
                ).unwrap();

                pos_x += scaled_x(self.mid, self.scale) as i32;
            }

            pos_y += scaled_y(self.mid, self.scale) as i32;
        }
    }
}