// from yume

use std::path::Path;

use anyhow::anyhow;
use sdl2::{image::LoadSurface, rect::Rect, render::TextureCreator, surface::Surface};

pub struct Texture<'a> {
    pub inner: sdl2::render::Texture<'a>,
    pub width: u32,
    pub height: u32
}

impl<'a> Texture<'a> {
    pub fn new<T>(surface: Surface<'a>, creator: &'a TextureCreator<T>) -> anyhow::Result<Self> {
        let surf_width = surface.width();
        let surf_height = surface.height();

        Ok(Self {
            inner: creator.create_texture_from_surface(surface).map_err(|e| anyhow!("failed to load texture: {}", e))?,
            height: surf_height,
            width: surf_width
        })
    }

    pub fn from_file<T, P: AsRef<Path>>(file: P, creator: &'a TextureCreator<T>) -> anyhow::Result<Self> {
        let surface = Surface::from_file(file);

        if let Ok(surf) = surface {
            Ok(Self::new(surf, creator)?)
        } else {
            Err(anyhow::Error::msg(surface.err().unwrap_or("failed to load texture".to_string())))
        }
    }

    pub fn new_repeating<T, P: AsRef<Path>>(file: P, creator: &'a TextureCreator<T>, x: u32, y: u32) -> anyhow::Result<Self> {
        let base_surface = Surface::from_file(file).unwrap();
        let mut target_surface = Surface::new(base_surface.width() * x, base_surface.height() * y, base_surface.pixel_format_enum()).unwrap();
        
        for y in 0..y {
            for x in 0..x {
                base_surface.blit(
                    None, 
                    &mut target_surface, 
                    Rect::new(
                        (x * base_surface.width()) as i32,
                        (y * base_surface.height()) as i32,
                        base_surface.width(),
                        base_surface.height()
                    )
                ).unwrap();
            }
        }

        Self::new(target_surface, creator)
    }

    pub fn set_color_mod(&mut self, color: (u8, u8, u8)) {
        self.inner.set_color_mod(color.0, color.1, color.2);
    }

    pub fn set_alpha_mod(&mut self, a: u8) {
        self.inner.set_alpha_mod(a);
    }
    
    pub fn color_mod(&self) -> (u8, u8, u8) {
        self.inner.color_mod()
    }

    pub fn alpha_mod(&self) -> u8 {
        self.inner.alpha_mod()
    }
}