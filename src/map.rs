use nalgebra::Vector2;
use sdl2::pixels::Color;

use crate::texture;


// must impl Default + Copy + Clone
type CellType = u8;

pub struct Map<'a> {
    pub width: u32,
    pub height: u32,
    pub cell_size: f32,
    data: Vec<CellType>,
    pub textures: Vec<texture::Texture<'a>>,
    pub ceiling: Color,
    pub floor: Color,
    pub segments: Vec<(Vector2<f32>, Vector2<f32>)>
}

pub struct RaycastResult {
    pub cell: (u32, u32),
    pub index: usize,
    pub pos: (f32, f32),
    pub u: f32
}

fn raycast_helpers(cell_size: f32, pos: f32, dir: f32) -> (i32, i32, f32, f32) {
    let tile = (pos / cell_size).floor() + 1.0;

    let d_tile;
    let dt;

    if dir > 0.0 {
        d_tile = 1;
        dt = (tile * cell_size - pos) / dir;
    } else if dir == 0.0 {
        return (tile as i32, 0, std::f32::MAX, 0.0)
    } else {
        d_tile = -1;
        dt = ((tile - 1.0) * cell_size - pos) / dir;
    }

    (tile as i32, d_tile, dt, d_tile as f32 * cell_size / dir)
}

impl<'a> Map<'a> {
    pub fn new(width: u32, height: u32, cell_size: f32) -> Self {
        Self {
            width, height, cell_size,
            data: vec![CellType::default(); (width * height) as usize],
            textures: Vec::new(),
            ceiling: Color::BLACK,
            floor: Color::BLACK,
            segments: Vec::new()
        }
    }

    pub fn add_texture(&mut self, texture: texture::Texture<'a>) -> usize {
        self.textures.push(texture);
        self.textures.len() - 1
    }

    pub fn set(&mut self, x: u32, y: u32, to: CellType) {
        self.data[(y * self.width + x) as usize] = to;
    }

    pub fn get(&self, x: u32, y: u32) -> CellType {
        self.data[(y * self.width + x) as usize]
    }

    pub fn from_string(string: &str, width: u32, height: u32, cell_size: f32) -> Self {
        let mut map = Self::new(width, height, cell_size);

        let mut chars = string.chars();
        for y in 0..height {
            for x in 0..width {
                let id = chars.next().unwrap().to_digit(10).unwrap();
                map.set(x, y, id as u8);
            }
        }

        map.regenerate_segments();

        map
    }

    pub fn cast_ray(&self, x: f32, y: f32, dir_x: f32, dir_y: f32) -> Option<RaycastResult> {
        let (mut tile_x, dtile_x, mut dt_x, ddt_x) = raycast_helpers(self.cell_size, x, dir_x);
        let (mut tile_y, dtile_y, mut dt_y, ddt_y) = raycast_helpers(self.cell_size, y, dir_y);

        let mut t = 0.0;

        let mut cur_x: f32 = 0.0;
        let mut cur_y: f32 = 0.0;
        let mut was_y = dt_y < dt_x;

        if dir_x.powf(2.0) + dir_y.powf(2.0) > 0.0 {
            while tile_x >= 0 && tile_x < self.width as i32 && tile_y >= 0 && tile_y < self.height as i32 {
                if self.get(tile_x as u32, tile_y as u32) != 0 {

                    let u = if was_y {
                        cur_x.abs() % self.cell_size
                    } else {
                        cur_y.abs() % self.cell_size
                    };

                    return Some(RaycastResult {
                        cell: (tile_x as u32, tile_y as u32),
                        index: (tile_y * self.width as i32 + tile_x) as usize,
                        pos: (cur_x, cur_y),
                        u: u / self.cell_size
                    })
                }

                if dt_x < dt_y {
                    tile_x += dtile_x;
                    let dt = dt_x;
                    t += dt;
                    dt_x += ddt_x - dt;
                    dt_y  -= dt;
                    was_y = false
                } else {
                    tile_y += dtile_y;
                    let dt = dt_y;
                    t += dt;
                    dt_x -= dt;
                    dt_y += ddt_y - dt;
                    was_y = true;
                }

                cur_x = x + dir_x * t;
                cur_y = y + dir_y * t;
            }
        } else if self.get(tile_x as u32, tile_y as u32) != 0 {
            return Some(RaycastResult {
                cell: (tile_x as u32, tile_y as u32),
                index: (tile_y * self.width as i32 + tile_x) as usize,
                pos: (cur_x, cur_y),
                u: 0.5
            })
        }

        None
    }

    pub fn shade(&self, distance: f32) -> f32 {
        ((1.0 / distance) * 2.0).min(1.0)
    }

    pub fn shade_from_height(&self, height: u32, screen_height: u32) -> f32 {
        ((2.0 * height as f32) / (screen_height as f32)).min(1.0)
    }

    fn add_segment_square(&mut self, cell_x: u32, cell_y: u32) {
        let x = (cell_x as f32 - 1.0) * self.cell_size;
        let y = (cell_y as f32 - 1.0) * self.cell_size;
        self.segments.push((Vector2::new(x, y), Vector2::new(x, y + self.cell_size)));
        self.segments.push((Vector2::new(x, y + self.cell_size), Vector2::new(x + self.cell_size, y + self.cell_size)));
        self.segments.push((Vector2::new(x + self.cell_size, y + self.cell_size), Vector2::new(x + self.cell_size, y)));
        self.segments.push((Vector2::new(x + self.cell_size, y), Vector2::new(x, y)));
    }

    // TODO: joining, removing redundant segments
    pub fn regenerate_segments(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                if self.get(x, y) > 0 {
                    self.add_segment_square(x, y);
                }
            }
        }
    }
}