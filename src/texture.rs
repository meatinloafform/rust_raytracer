// from yume

use std::path::Path;

use anyhow::anyhow;
use sdl2::{image::LoadSurface, render::TextureCreator, surface::Surface};

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
}