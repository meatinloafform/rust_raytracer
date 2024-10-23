use std::{path::Path, rc::Rc};

use sdl2::render::{Canvas, RenderTarget, TextureCreator};

use crate::{player::Player, texture};

pub struct UI<'a> {
    pub fonts: FontBank<'a>
}

impl<'a> UI<'a> {
    pub fn new<T>(creator: &'a TextureCreator<T>) -> anyhow::Result<Self> {
        anyhow::Ok(Self {
            fonts: FontBank::load_all(creator)?
        })
    }

    pub fn update(&mut self, player: &Player) {

    }

    pub fn draw<T: RenderTarget>(&self, canvas: &mut Canvas<T>) {
        
    }
}

pub struct FontBank<'a> {
    test: Rc<Font<'a>>
}

const YUME_FONT_STRING: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .!,-�?§µ";
const YUME_MINIFONT_STRING: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .!,-�?§µ()[]{}<>'\"`+=/\\*|#%^:;";

impl<'a> FontBank<'a> {
    pub fn test(&self) -> Rc<Font> {
        self.test.clone()
    }

    pub fn load_all<T>(creator: &'a TextureCreator<T>) -> anyhow::Result<Self> {
        let yume_string = YUME_FONT_STRING.chars().collect::<Vec<char>>();

        anyhow::Ok(Self {
            test: Rc::new(Font::from_path("res/textures/ui/fonts/menu.png", yume_string, 6, 10, 10, 1.0, creator)?)
        })
    }
}

pub struct Font<'a> {
    pub image: texture::Texture<'a>,
    char_width: u32,
    char_height: u32,
    width: u32,
    scale: f32
}

impl<'a> Font<'a> {
    pub fn draw_char_at<T: RenderTarget>(&self, canvas: &mut Canvas<T>, char: char, x: f32, y: f32) {

    }

    pub fn from_path<P: AsRef<Path>, T>(path: P, chars: Vec<char>, char_width: u32, char_height: u32, width: u32, scale: f32, creator: &'a TextureCreator<T>) -> anyhow::Result<Self> {
        let texture = texture::Texture::from_file(path, creator)?;

        anyhow::Ok(Self {
            image: texture,
            char_height,
            char_width,
            width,
            scale
        })
    }
}