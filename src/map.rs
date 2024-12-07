use std::{cell::{self, RefCell}, collections::HashMap, fs, path::Path, rc::Rc};

use anyhow::Context;
use nalgebra::Vector2;
use sdl2::{pixels::Color, render::TextureCreator};
use serde::{Deserialize, Serialize};

use crate::{collision, texture};

// TODO:
// I noticed you're not adjusting your ray intersection depths for the ray angle which leads to a subtle fish eye effect. 
// If you scale the intersection depth by the cosine of the angle between the ray and the centre ray you'll eliminate the fish eye.

// must impl Default + Copy + Clone
type CellType = u8;
type MapTexture<'a> = Rc<RefCell<texture::Texture<'a>>>;

pub struct Map<'a> {
    pub name: String,
    
    /// Map width (in tiles)
    pub width: u32,

    /// Map height (in tiles)
    pub height: u32,
    pub cell_size: f32,
    data: Vec<CellType>,
    // pub textures: Vec<texture::Texture<'a>>,
    pub textures: Vec<MapTexture<'a>>,
    pub decal_textures: Vec<(MapTexture<'a>, String)>,
    pub decals: Vec<Decal>,
    pub ceiling: Color,
    pub floor: Color,
    pub segments: Vec<(Vector2<f32>, Vector2<f32>)>,
    pub spawnpoint: (f32, f32),
    pub transparent: Vec<bool>,
    pub billboards: Vec<Billboard>,
    pub billboard_textures: Vec<(MapTexture<'a>, String)>,
    pub effects: MapEffects,
    pub lights: Vec<Light>,
    pub adjacent_maps: AdjacentMaps,
    pub load_adjacent: LoadAdjacent,
    pub index: usize
}

/// Store information for loading adjacent maps
#[derive(Default)]
pub struct LoadAdjacent {
    pub left: Option<(String, (i32, i32))>,
    pub right: Option<(String, (i32, i32))>,
    pub up: Option<(String, (i32, i32))>,
    pub down: Option<(String, (i32, i32))>,
}

#[derive(Default)]
pub struct AdjacentMaps {
    // TODO: Remove
    // pub render: AdjacentMapsStorage<'a>,
    pub storage: AdjacentMapsSpecifiers
}

/// Store information about surrounding maps
#[derive(Default)]
pub struct AdjacentMapsSpecifiers {
    pub left: Option<(usize, (i32, i32), (u32, u32))>,
    pub right: Option<(usize, (i32, i32), (u32, u32))>,
    pub up: Option<(usize, (i32, i32), (u32, u32))>,
    pub down: Option<(usize, (i32, i32), (u32, u32))>
}

// #[derive(Default)]
// pub struct AdjacentMapsStorage<'a> {
//     pub left: Option<Rc<Map<'a>>>,
//     pub right: Option<Rc<Map<'a>>>,
//     pub up: Option<Rc<Map<'a>>>,
//     pub down: Option<Rc<Map<'a>>>
// }

/// Store visual map effects
pub struct MapEffects {
    pub wall_height_multi: f32,
    pub shade_walls: bool,
    pub shade_floor: bool,
    pub shade_ceiling: bool,
    pub shade_multi: f32
}

impl MapEffects {
    pub fn new() -> Self {
        Self {
            wall_height_multi: 1.0,
            shade_ceiling: true,
            shade_floor: true,
            shade_multi: 1.0,
            shade_walls: true
        }
    }
}

/// In-world light
pub struct Light {
    pub pos: (f32, f32),
    pub color: (f32, f32, f32),
    pub brightness: f32,
    pub range: f32
}

impl Light {
    pub fn player() -> Self {
        Self {
            brightness: 1.0,
            pos: (0.0, 0.0),
            color: (1.0, 1.0, 1.0),
            range: 5.0
        }
    }
}

/// In-world decal (line segment)
pub struct Decal {
    pub texture: usize,
    pub p0: (f32, f32),
    pub p1: (f32, f32),
    pub y: f32,
    pub height: f32
}

/// In-world billboard (always facing player)
pub struct Billboard {
    pub texture: usize,
    pub origin: (f32, f32),
    pub width: f32,
    pub y: f32,
    pub height: f32
}

pub struct GlobalBillboard {
    pub billboard: Billboard,
    pub map: usize
}

/// Information about a wall hit during raycast
#[derive(Debug)]
pub struct HitWall {
    pub cell: (u32, u32),
    pub index: usize,
    pub pos: (f32, f32),
    pub u: f32,
    pub transparent: bool
}

/// Information about a decal hit during raycast
#[derive(Debug)]
pub struct HitDecal {
    pub decal_index: usize,
    pub u: f32,
    pub pos: (f32, f32)
}

/// Information about an edge hit during raycast
#[derive(Debug)]
pub struct HitEdge {
    pub pos: (f32, f32)
}

/// Information about a billboard hit during raycast
#[derive(Debug)]
pub struct HitBillboard {
    pub billboard_index: usize,
    pub u: f32,
    pub pos: (f32, f32),
    pub global: bool
}

/// Information about a raycast hit
#[derive(Debug)]
pub enum RaycastHit {
    Wall(HitWall),
    Decal(HitDecal),
    Edge(HitEdge),
    Billboard(HitBillboard)
}

/// Full raycast hit data
#[derive(Debug)]
pub struct RaycastResult {
    pub hit: RaycastHit,
    pub light: (f32, f32, f32),
    pub distance: f32,
    pub map: usize
}

/// Describes how the ray should move along a boundry
pub enum EdgeRayMove {
    Zero,
    Keep,
    Edge
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

impl AdjacentMaps {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

const EPSILON: f32 = 1e-5;

fn handle_billboard(
    dir_x: f32, dir_y: f32, billboard: &Billboard, 
    l0: (f32, f32), l1: (f32, f32), x: f32, y: f32, hits: &mut Vec<RaycastResult>,
    i: usize, map_index: usize, last_map: &Option<usize>,
    maps_list: &Vec<Rc<Map>>, distance_travelled: f32,
    global: bool
) {
    let bl0 = (-dir_y * (billboard.width / 2.0) + billboard.origin.0,  dir_x * (billboard.width / 2.0) + billboard.origin.1);
    let bl1 = ( dir_y * (billboard.width / 2.0) + billboard.origin.0, -dir_x * (billboard.width / 2.0) + billboard.origin.1);

    if let Some((lx, ly, s, _)) = collision::get_line_intersection(
        l0.0, l0.1, l1.0, l1.1,
        bl0.0, bl0.1, bl1.0, bl1.1
    ) {
        hits.push(RaycastResult { hit: RaycastHit::Billboard(HitBillboard {
                billboard_index: i,
                pos: (lx, ly),
                u: s,
                global
            }),
            light: Map::get_light_mod(lx - (dir_x * 0.05), ly - (dir_y * 0.05), map_index, last_map, maps_list),
            distance: ((lx - x).powf(2.0) + (ly - y).powf(2.0)).sqrt() + distance_travelled,
            map: map_index
        });
    }
}

impl<'a> Map<'a> {
    pub fn new(width: u32, height: u32, cell_size: f32) -> Self {
        Self {
            name: String::new(),
            width, height, cell_size,
            data: vec![CellType::default(); (width * height) as usize],
            textures: Vec::new(),
            ceiling: Color::BLACK,
            floor: Color::BLACK,
            segments: Vec::new(),
            spawnpoint: (0.0, 0.0),
            transparent: Vec::new(),
            decal_textures: Vec::new(),
            decals: Vec::new(),
            billboards: Vec::new(),
            billboard_textures: Vec::new(),
            effects: MapEffects::new(),
            lights: Vec::new(),
            adjacent_maps: AdjacentMaps::new(),
            load_adjacent: LoadAdjacent::default(),
            index: 0
        }
    }

    /// Adds a texture to the map's texture list, returns the index
    pub fn add_texture(&mut self, texture: texture::Texture<'a>) -> usize {
        self.textures.push(Rc::new(RefCell::new(texture)));
        self.textures.len() - 1
    }

    /// Set cell data
    pub fn set(&mut self, x: u32, y: u32, to: CellType) {
        self.data[(y * self.width + x) as usize] = to;
    }

    /// Get cell data
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
                print!("{}", id);
            }
            println!();
        }

        // map.regenerate_segments();

        map
    }

    // mama im a criminal
    pub fn with_wall_texture_color(&self, color: (u8, u8, u8), ix: usize) -> (u8, u8, u8) {
        let original = self.textures[ix].borrow().color_mod();
        self.textures[ix].borrow_mut().set_color_mod(color);
        original
    }

    pub fn with_decal_texture_color(&self, color: (u8, u8, u8), ix: usize) -> (u8, u8, u8) {
        let original = self.decal_textures[ix].0.borrow().color_mod();
        self.decal_textures[ix].0.borrow_mut().set_color_mod(color);
        original
    }

    pub fn with_billboard_texture_color(&self, color: (u8, u8, u8), ix: usize) -> (u8, u8, u8) {
        let original = self.billboard_textures[ix].0.borrow().color_mod();
        self.billboard_textures[ix].0.borrow_mut().set_color_mod(color);
        original
    }

    /// Return true if ray hits a wall
    pub fn raycast_hit_blocking(
        x: f32, y: f32, dir_x: f32, dir_y: f32, distance: f32,
        distance_travelled: f32, map_index: usize, maps_list: &Vec<Rc<Map>>
    ) -> bool {
        let map = &maps_list[map_index].clone();

        let (mut tile_x, dtile_x, mut dt_x, ddt_x) = raycast_helpers(map.cell_size, x, dir_x);
        let (mut tile_y, dtile_y, mut dt_y, ddt_y) = raycast_helpers(map.cell_size, y, dir_y);

        let mut t = 0.0;

        let mut cur_x: f32 = 0.0;
        let mut cur_y: f32 = 0.0;

        // Skip if direction is 0
        if dir_x.powf(2.0) + dir_y.powf(2.0) > 0.0 {

            // Check bounds
            while tile_x >= 0 && tile_x < map.width as i32 && tile_y >= 0 && tile_y < map.height as i32 {
                let tile = map.get(tile_x as u32, tile_y as u32);
                if tile != 0 {
                    if !map.transparent[tile as usize - 1] {
                        // Exit if the point is reached and a wall is hit at the same time
                        if distance - (t + distance_travelled) < EPSILON {
                            return false;
                        }

                        return true;
                    }
                }

                if dt_x < dt_y {
                    tile_x += dtile_x;
                    let dt = dt_x;
                    t += dt;
                    dt_x += ddt_x - dt;
                    dt_y  -= dt;
                } else {
                    tile_y += dtile_y;
                    let dt = dt_y;
                    t += dt;
                    dt_x -= dt;
                    dt_y += ddt_y - dt;
                }

                cur_x = x + dir_x * t;
                cur_y = y + dir_y * t;

                if (t + distance_travelled) > distance {
                    return false;
                }
            }
        } else if map.get(tile_x as u32, tile_y as u32) != 0 {
            return true;
        }

        // Handle boundary crossing
        if let Some((boundary, (move_x, move_y), tile_offset)) = map.get_boundary_cross(cur_x, cur_y) {
            let map = maps_list[boundary].clone();
            let distance_this_pass = ((cur_x - x).powf(2.0) + (cur_y - y).powf(2.0)).sqrt();
            let offset = (tile_offset.0 as f32 * map.cell_size, tile_offset.1 as f32 * map.cell_size);

            let new_x = match move_x {
                EdgeRayMove::Zero => -map.cell_size,
                EdgeRayMove::Keep => cur_x - offset.0,
                EdgeRayMove::Edge => ((map.width - 1) as f32 * map.cell_size) - 0.01
            };
            let new_y = match move_y {
                EdgeRayMove::Zero => -map.cell_size,
                EdgeRayMove::Keep => cur_y - offset.1,
                EdgeRayMove::Edge => ((map.height - 1) as f32 * map.cell_size) - 0.01
            };

            return Self::raycast_hit_blocking(
                new_x, new_y, dir_x, dir_y, distance, distance_travelled + distance_this_pass, 
                boundary, maps_list
            );
        }

        // Out of bounds
        false
    }

    // y= -(x/r)^2 + c
    fn light_falloff(range: f32, brightness: f32, distance: f32) -> f32 {
        // (((1.0 / (distance + 1.0 / 2.0).powf(2.0)) + brightness) * (range - distance)).min(brightness).max(0.0)
        (-(distance / range).powf(2.0) + brightness).max(0.0).min(1.0)
    }

    fn process_light(&self, light: &Light, x: f32, y: f32, cur_light_mod: &mut (f32, f32, f32), map_index: usize, maps_list: &Vec<Rc<Map>>) {
        let magnitude = ((light.pos.0 - x).powf(2.0) + (light.pos.1 - y).powf(2.0)).sqrt();
        let dir_x = (light.pos.0 - x) / magnitude;
        let dir_y = (light.pos.1 - y) / magnitude;

        if !Self::raycast_hit_blocking(x, y, dir_x, dir_y, magnitude, 0.0, map_index, maps_list) {
            // let distance = ((light.pos.1 - y).powf(2.0) + (light.pos.0 - x).powf(2.0)).sqrt();
            let falloff = Self::light_falloff(light.range, light.brightness, magnitude);
            cur_light_mod.0 += falloff * light.color.0;
            cur_light_mod.1 += falloff * light.color.1;
            cur_light_mod.2 += falloff * light.color.2;
        }
    }

    fn process_crossmap_light(light: &Light, x: f32, y: f32, rel_x: f32, rel_y: f32, cur_light_mod: &mut (f32, f32, f32), map_index: usize, maps_list: &Vec<Rc<Map>>) {
        let magnitude = ((rel_x - x).powf(2.0) + (rel_y - y).powf(2.0)).sqrt();

        let dir_x = (x - rel_x) / magnitude;
        let dir_y = (y - rel_y) / magnitude;

        if !Self::raycast_hit_blocking(light.pos.0, light.pos.1, dir_x, dir_y, magnitude, 0.0, map_index, maps_list) {
            let falloff = Self::light_falloff(light.range, light.brightness, magnitude);
            cur_light_mod.0 += falloff * light.color.0;
            cur_light_mod.1 += falloff * light.color.1;
            cur_light_mod.2 += falloff * light.color.2;
        }
    }

    /// Get the brightness at a position
    pub fn get_light_mod(x: f32, y: f32, map_index: usize, last_map: &Option<usize>, maps_list: &Vec<Rc<Map>>) -> (f32, f32, f32) {
        let map = maps_list[map_index].clone();
        
        let mut cur_light_mod = (0.0, 0.0, 0.0);

        for light in map.lights.iter() {
            map.process_light(light, x, y, &mut cur_light_mod, map_index, maps_list);
        }

        let mut last_processed = false;

        if let Some((map_index, offset, ..)) = map.adjacent_maps.storage.right {
            let adjacent_map = maps_list[map_index].clone();
            let cell_size = adjacent_map.cell_size;

            if let Some(last_map) = last_map {
                if *last_map == map_index {
                    last_processed = true;
                }
            }

            for light in maps_list[map_index].lights.iter() {
                let light_relative_x = light.pos.0 + (((map.width) as f32) * map.cell_size) + (offset.0 as f32 * cell_size);
                let light_relative_y = light.pos.1 + (offset.1 as f32 * cell_size);

                Self::process_crossmap_light(light, x, y, light_relative_x, light_relative_y, &mut cur_light_mod, map_index, maps_list);
            } 
        }

        if let Some((map_index, offset, ..)) = map.adjacent_maps.storage.left {
            let cell_size = maps_list[map_index].cell_size;

            if let Some(last_map) = last_map {
                if *last_map == map_index {
                    last_processed = true;
                }
            }
            
            for light in maps_list[map_index].lights.iter() {
                let other_width = ((maps_list[map_index].width) as f32) * map.cell_size;
                let light_relative_x = light.pos.0 - other_width + (offset.0 as f32 * cell_size);
                let light_relative_y = light.pos.1 + (offset.1 as f32 * cell_size);

                Self::process_crossmap_light(light, x, y, light_relative_x, light_relative_y, &mut cur_light_mod, map_index, maps_list);
            } 
        }

        if let Some((map_index, offset, ..)) = map.adjacent_maps.storage.up {
            let cell_size = maps_list[map_index].cell_size;

            if let Some(last_map) = last_map {
                if *last_map == map_index {
                    last_processed = true;
                }
            }

            for light in maps_list[map_index].lights.iter() {
                let other_height = ((maps_list[map_index].height as f32) * cell_size);
                let light_relative_x = light.pos.0 + (offset.0 as f32 * cell_size);
                let light_relative_y = light.pos.1 - other_height + (offset.1 as f32 * cell_size);

                Self::process_crossmap_light(light, x, y, light_relative_x, light_relative_y, &mut cur_light_mod, map_index, maps_list);
            }
        }

        if let Some((map_index, offset, ..)) = map.adjacent_maps.storage.down {
            let cell_size = maps_list[map_index].cell_size;

            if let Some(last_map) = last_map {
                if *last_map == map_index {
                    last_processed = true;
                }
            }

            for light in maps_list[map_index].lights.iter() {
                let self_height = ((map.height) as f32) * cell_size;
                let light_relative_x = light.pos.0 + (offset.0 as f32 * cell_size);
                let light_relative_y = light.pos.1 + self_height + (offset.1 as f32 * cell_size);

                Self::process_crossmap_light(light, x, y, light_relative_x, light_relative_y, &mut cur_light_mod, map_index, maps_list);
            }
        }

        // Only process the last map if it wasn't already processed
        if !last_processed {
            if let Some(last_map_index) = last_map {
                let last_map = maps_list[*last_map_index].clone();

                // If the last map's up is this map,
                // Cast ray from last map's light to this map's wall
                if let Some((last_adjacent, offset, ..)) = last_map.adjacent_maps.storage.up {
                    if last_adjacent == map_index {
                        let cell_size = last_map.cell_size;

                        for light in last_map.lights.iter() {
                            let self_height = ((map.height) as f32) * cell_size;
                            let light_relative_x = light.pos.0 + (-offset.0 as f32 * cell_size);
                            let light_relative_y = light.pos.1 + self_height + (-offset.1 as f32 * cell_size);

                            Self::process_crossmap_light(light, x, y, light_relative_x, light_relative_y, &mut cur_light_mod, last_map.index, maps_list);
                        }
                    }
                }

                // If the last map's down is this map,
                // Cast ray from the last map's light to this map's wall
                if let Some((last_adjacent, offset, ..)) = last_map.adjacent_maps.storage.down {
                    if last_adjacent == map_index {
                        let cell_size = last_map.cell_size;

                        for light in last_map.lights.iter() {
                            let other_height = ((last_map.height as f32) * cell_size);
                            let light_relative_x = light.pos.0 + (-offset.0 as f32 * cell_size);
                            let light_relative_y = light.pos.1 - other_height + (-offset.1 as f32 * cell_size);
                            // dbg!((light_relative_x, light_relative_y));

                            // print!("{:?}: ", cur_light_mod);
                            Self::process_crossmap_light(light, x, y, light_relative_x, light_relative_y, &mut cur_light_mod, last_map.index, maps_list);
                            // println!("{:?}", cur_light_mod);
                        }
                    }
                }
            }
        }

        cur_light_mod
    }

    pub fn get_boundary_cross(&self, x: f32, y: f32) -> Option<(usize, (EdgeRayMove, EdgeRayMove), (i32, i32))> {
        let x_0 = (x + self.cell_size).abs() < EPSILON;
        let y_0 = (y + self.cell_size).abs() < EPSILON;
        let x_edge = (x - ((self.width - 1) as f32 * self.cell_size)).abs() < EPSILON;
        let y_edge = (y - ((self.height - 1) as f32 * self.cell_size)).abs() < EPSILON;

        if x_0 {
            return self.adjacent_maps.storage.left.map(|a| (a.0, (EdgeRayMove::Edge, EdgeRayMove::Keep), a.1));
        }

        if y_0 {
            return self.adjacent_maps.storage.up.map(|a| (a.0, (EdgeRayMove::Keep, EdgeRayMove::Edge), a.1));
        }

        if x_edge {
            return self.adjacent_maps.storage.right.map(|a| (a.0, (EdgeRayMove::Zero, EdgeRayMove::Keep), a.1));
        }

        if y_edge {
            return self.adjacent_maps.storage.down.map(|a| (a.0, (EdgeRayMove::Keep, EdgeRayMove::Zero), a.1));
        }

        None
    }

    /// Cast a ray and return information about hits
    pub fn cast_ray(
        &self, x: f32, y: f32, dir_x: f32, dir_y: f32, 
        distance_travelled: f32, map_index: usize, last_map: &Option<usize>,
        maps_list: &Vec<Rc<Map>>, global_billboards: &Vec<GlobalBillboard>
    ) -> Vec<RaycastResult> {
        let (mut tile_x, dtile_x, mut dt_x, ddt_x) = raycast_helpers(self.cell_size, x, dir_x);
        let (mut tile_y, dtile_y, mut dt_y, ddt_y) = raycast_helpers(self.cell_size, y, dir_y);

        let mut t = 0.0;

        let mut cur_x: f32 = x;
        let mut cur_y: f32 = y;
        let mut was_y = dt_y < dt_x;

        let mut hits = Vec::new();
        let mut ids_hit = Vec::new();

        if dir_x.powf(2.0) + dir_y.powf(2.0) > 0.0 {
            while tile_x >= 0 && tile_x < self.width as i32 && tile_y >= 0 && tile_y < self.height as i32 {
                let tile = self.get(tile_x as u32, tile_y as u32);
                if tile != 0 {
                    let u = if was_y {
                        cur_x.abs() % self.cell_size
                    } else {
                        cur_y.abs() % self.cell_size
                    };

                    let index = (tile_y * self.width as i32 + tile_x) as usize;

                    // Ignore repeated transparent textures
                    if !ids_hit.contains(&tile) {
                        hits.push(RaycastResult { hit: RaycastHit::Wall(HitWall {
                                cell: (tile_x as u32, tile_y as u32),
                                index,
                                pos: (cur_x, cur_y),
                                u: u / self.cell_size,
                                transparent: self.transparent[tile as usize - 1]
                            }),
                            light: Self::get_light_mod(cur_x - (dir_x * 0.05), cur_y - (dir_y * 0.05), map_index, last_map, maps_list),
                            // light: (1.0, 1.0, 1.0),
                            distance: ((cur_x - x).powf(2.0) + (cur_y - y).powf(2.0)).sqrt() + distance_travelled,
                            map: map_index
                        });

                        ids_hit.push(tile);
                    }

                    if !self.transparent[tile as usize - 1] {
                        return hits;
                    }
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

                let l0 = (cur_x, cur_y);

                cur_x = x + dir_x * t;
                cur_y = y + dir_y * t;

                let l1 = (cur_x, cur_y);

                for (i, decal) in self.decals.iter().enumerate() {
                    if let Some((lx, ly, s, _)) = collision::get_line_intersection(
                        l0.0, l0.1, l1.0, l1.1,
                        decal.p0.0, decal.p0.1, decal.p1.0, decal.p1.1
                    ) {
                        hits.push(RaycastResult { hit: RaycastHit::Decal(HitDecal {
                                decal_index: i,
                                pos: (lx, ly),
                                u: s
                            }),
                            light: Self::get_light_mod(lx - (dir_x * 0.05),ly - (dir_y * 0.05), map_index, last_map, maps_list),
                            distance: ((lx - x).powf(2.0) + (ly - y).powf(2.0)).sqrt() + distance_travelled,
                            map: map_index
                        });

                    }
                }

                for (i, billboard) in self.billboards.iter().enumerate() {
                    handle_billboard(dir_x, dir_y, billboard, l0, l1, x, y, &mut hits, i, map_index, last_map, maps_list, distance_travelled, false);
                }
                for (i, billboard) in global_billboards.iter().enumerate() {
                    if billboard.map == map_index {
                        handle_billboard(dir_x, dir_y, &billboard.billboard, l0, l1, x, y, &mut hits, i, map_index, last_map, maps_list, distance_travelled, true);
                    }
                }
            }
        } else if self.get(tile_x as u32, tile_y as u32) != 0 {
            hits.push(RaycastResult { hit: RaycastHit::Wall(HitWall {
                    cell: (tile_x as u32, tile_y as u32),
                    index: (tile_y * self.width as i32 + tile_x) as usize,
                    pos: (cur_x, cur_y),
                    u: 0.5,
                    transparent: false
                }),
                light: Self::get_light_mod(cur_x - (dir_x * 0.05), cur_y - (dir_y * 0.05), map_index, last_map, maps_list),
                distance: ((cur_x - x).powf(2.0) + (cur_y - y).powf(2.0)).sqrt() + distance_travelled,
                map: map_index
            });
            
            return hits;
        }

        // Out of bounds
        if let Some((boundary, (move_x, move_y), tile_offset)) = self.get_boundary_cross(cur_x, cur_y) {
            let map = maps_list[boundary].clone();
            let distance = ((cur_x - x).powf(2.0) + (cur_y - y).powf(2.0)).sqrt();

            // TODO: consider whether to use self.cell_size or map.cell_size
            let offset = (tile_offset.0 as f32 * map.cell_size, tile_offset.1 as f32 * map.cell_size);

            let new_x = match move_x {
                EdgeRayMove::Zero => -map.cell_size,
                EdgeRayMove::Keep => cur_x - offset.0,
                EdgeRayMove::Edge => ((map.width - 1) as f32 * map.cell_size) - 0.01
            };
            let new_y = match move_y {
                EdgeRayMove::Zero => -map.cell_size,
                EdgeRayMove::Keep => cur_y - offset.1,
                EdgeRayMove::Edge => ((map.height - 1) as f32 * map.cell_size) - 0.01
            };
            hits.append(&mut map.cast_ray(new_x, new_y, dir_x, dir_y, distance_travelled + distance, boundary, &Some(map_index), maps_list, global_billboards));
        } else {
            hits.push(RaycastResult { 
                hit: RaycastHit::Edge(HitEdge {
                    pos: (cur_x, cur_y)
                }),
                light: (1.0, 1.0, 1.0),
                distance: ((cur_x - x).powf(2.0) + (cur_y - y).powf(2.0)).sqrt() + distance_travelled,
                map: map_index
            });
        }

        hits
    }

    pub fn shade(&self, distance: f32) -> f32 {
        ((1.0 / (distance * self.effects.shade_multi)) * 2.0).min(1.0)
    }

    pub fn shade_from_height(&self, height: u32, screen_height: u32) -> f32 {
        ((2.0 * height as f32) / (screen_height as f32 * self.effects.shade_multi)).min(1.0)
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
        self.segments.clear();

        for y in 0..self.height {
            for x in 0..self.width {
                if self.get(x, y) > 0 {
                    self.add_segment_square(x, y);
                }
            }
        }

        // Border
        // let off = -self.cell_size;
        // self.segments.push((Vector2::new(off, off), Vector2::new(self.width as f32 * self.cell_size + off, off)));
        // self.segments.push((Vector2::new(self.width as f32 * self.cell_size + off, off), Vector2::new(self.width as f32 * self.cell_size + off, self.height as f32 * self.cell_size + off)));
        // self.segments.push((Vector2::new(self.width as f32 * self.cell_size + off, self.height as f32 * self.cell_size + off), Vector2::new(off, self.height as f32 * self.cell_size + off)));
        // self.segments.push((Vector2::new(off, self.height as f32 * self.cell_size + off), Vector2::new(off, off)));

        let off = -self.cell_size;

        // Add top edge segments
        if let Some(up) = &self.adjacent_maps.storage.up {
            let pass_range_min = up.1.0;
            let pass_range_max = pass_range_min + up.2.0 as i32;

            for x in 0..self.width {
                if (x as i32) < pass_range_min || (x as i32) > pass_range_max {
                    let x_pos = (x as f32 - 1.0) * self.cell_size;
                    self.segments.push((
                        Vector2::new(x_pos, off),
                        Vector2::new(x_pos + self.cell_size, off)
                    ));
                }
            }
        } else {
            self.segments.push((Vector2::new(off, off), Vector2::new(self.width as f32 * self.cell_size + off, off)));
        }

        // Add bottom edge segments
        if let Some(down) = &self.adjacent_maps.storage.down {
            let pass_range_min = down.1.0;
            let pass_range_max = pass_range_min + down.2.0 as i32;
            let height = self.height as f32 * self.cell_size + off;

            for x in 0..self.width {
                if (x as i32) < pass_range_min || (x as i32) > pass_range_max {
                    let x_pos = (x as f32 - 1.0) * self.cell_size;
                    self.segments.push((
                        Vector2::new(x_pos, height),
                        Vector2::new(x_pos + self.cell_size, height)
                    ));
                }
            }
        } else {
            self.segments.push((Vector2::new(self.width as f32 * self.cell_size + off, self.height as f32 * self.cell_size + off), Vector2::new(off, self.height as f32 * self.cell_size + off)));
        }

        // Add left edge segments
        if let Some(left) = &self.adjacent_maps.storage.left {
            let pass_range_min = left.1.1;
            let pass_range_max = pass_range_min + left.2.1 as i32;

            for y in 0..self.height {
                if (y as i32) < pass_range_min || (y as i32) > pass_range_max {
                    let y_pos = (y as f32 - 1.0) * self.cell_size;
                    self.segments.push((
                        Vector2::new(off, y_pos),
                        Vector2::new(off, y_pos + self.cell_size)
                    ));
                }
            }
        } else {
            self.segments.push((Vector2::new(off, self.height as f32 * self.cell_size + off), Vector2::new(off, off)));
        }

        // Add right edge segments
        if let Some(right) = &self.adjacent_maps.storage.right {
            let pass_range_min = right.1.1;
            let pass_range_max = pass_range_min + right.2.1 as i32;
            let x_pos = self.width as f32 * self.cell_size + off;

            for y in 0..self.height {
                if (y as i32) < pass_range_min || (y as i32) > pass_range_max {
                    let y_pos = (y as f32 - 1.0) * self.cell_size;
                    self.segments.push((
                        Vector2::new(x_pos, y_pos),
                        Vector2::new(x_pos, y_pos + self.cell_size)
                    ));
                }
            }
        } else {
            self.segments.push((Vector2::new(self.width as f32 * self.cell_size + off, off), Vector2::new(self.width as f32 * self.cell_size + off, self.height as f32 * self.cell_size + off)));
        }
    }

    pub fn from_path<P: AsRef<Path>, T>(path: P, creator: &'a TextureCreator<T>) -> anyhow::Result<Self> {
        let file = fs::read_to_string(&path)?;
        let mut map = Self::from_ron(&file, creator)?;
        map.name = path.as_ref().file_name().context("failed to get path file stem")?.to_str().context("failed to convert string")?.to_string();
        anyhow::Ok(map)
    }
    
    fn from_ron<T>(src: &str, creator: &'a TextureCreator<T>) -> anyhow::Result<Self> {
        let map_file: MapFile = ron::from_str(src)?;

        let map_width = map_file.map.split("\r\n").next().unwrap().len();
        let flattened = map_file.map.replace("\r\n", "");
        let map_height = flattened.len() / map_width;

        println!("Size: {}x{}, len: {}", map_width, map_height, map_file.map.len());

        let mut map = Self::from_string(&flattened, map_width as u32, map_height as u32, map_file.cell_size);

        for texture_path in map_file.key.iter() {
            if let Some(repeat) = map_file.repeating_textures.get(texture_path) {
                map.add_texture(texture::Texture::new_repeating(format!("res/textures/{}", texture_path), creator, repeat.0, repeat.1)?);
            } else {
                map.add_texture(texture::Texture::from_file(format!("res/textures/{}", texture_path), creator)?);
            }
        }

        map.spawnpoint = ((map_file.player_spawn.0 as f32 - 0.5) * map.cell_size, (map_file.player_spawn.1 as f32 - 0.5) * map.cell_size);

        map.floor = Color::RGB(map_file.floor.0, map_file.floor.1, map_file.floor.2);
        map.ceiling = Color::RGB(map_file.ceiling.0, map_file.ceiling.1, map_file.ceiling.2);
        map.effects.shade_ceiling = map_file.shade_ceiling;
        map.effects.shade_floor = map_file.shade_floor;
        map.effects.shade_walls = map_file.shade_walls;
        map.effects.shade_multi = map_file.shade_multi;
        map.transparent = vec![false; map.textures.len()];
        for transparent_tile in map_file.transparent.iter() {
            map.transparent[transparent_tile - 1] = true;
        }
        map.effects.wall_height_multi = map_file.wall_height_multi;

        if map_file.connections.up.map.len() != 0 {
            let map_path = format!("res/maps/{}", map_file.connections.up.map);
            map.load_adjacent.up = Some((map_path, (map_file.connections.up.offset)));
        }
        if map_file.connections.down.map.len() != 0 {
            let map_path = format!("res/maps/{}", map_file.connections.down.map);
            map.load_adjacent.down = Some((map_path, (map_file.connections.down.offset)));
        }
        if map_file.connections.left.map.len() != 0 {
            let map_path = format!("res/maps/{}", map_file.connections.left.map);
            map.load_adjacent.left = Some((map_path, (map_file.connections.left.offset)));
        }
        if map_file.connections.right.map.len() != 0 {
            let map_path = format!("res/maps/{}", map_file.connections.right.map);
            map.load_adjacent.right = Some((map_path, (map_file.connections.right.offset)));
        }

        for file_decal in map_file.decals.iter() {
            let mut texture_index = 0;
            let mut found_texture = false;
            for (i, texture) in map.decal_textures.iter().enumerate() {
                if texture.1 == file_decal.texture {
                    texture_index = i;
                    found_texture = true;
                }
            }

            if !found_texture {
                map.decal_textures.push(
                    (Rc::new(RefCell::new(texture::Texture::from_file(format!("res/textures/{}", file_decal.texture), creator)?)), 
                    file_decal.texture.clone())
                );
                texture_index = map.decal_textures.len() - 1;
            }

            map.decals.push(Decal {
                height: file_decal.height,
                p0: file_decal.start,
                p1: file_decal.end,
                texture: texture_index,
                y: file_decal.y
            });
        }

        for file_billboard in map_file.billboards.iter() {
            let mut texture_index = 0;
            let mut found_texture = false;
            for (i, texture) in map.billboard_textures.iter().enumerate() {
                if texture.1 == file_billboard.texture {
                    texture_index = i;
                    found_texture = true;
                }
            }

            if !found_texture {
                map.billboard_textures.push(
                    (Rc::new(RefCell::new(texture::Texture::from_file(format!("res/textures/{}", file_billboard.texture), creator)?)), 
                    file_billboard.texture.clone())
                );
                texture_index = map.billboard_textures.len() - 1;
            }

            map.billboards.push(Billboard {
                height: file_billboard.height,
                origin: file_billboard.pos,
                texture: texture_index,
                width: file_billboard.width,
                y: file_billboard.y
            });
        }

        Ok(map)
    }
}

mod defaults {
    use std::collections::HashMap;

    pub fn shade_ceiling() -> bool { true }
    pub fn shade_floor() -> bool { true }
    pub fn shade_walls() -> bool { true }
    pub fn floor() -> (u8, u8, u8) { (0, 0, 0) }
    pub fn ceiling() -> (u8, u8, u8) { (0, 0, 0) }
    pub fn shade_multi() -> f32 { 1.0 }
    pub fn empty_vec<T>() -> Vec<T> { Vec::new() }
    pub fn repeating_textures() -> HashMap<String, (u32, u32)> { HashMap::new() }
    pub fn wall_height_multi() -> f32 { 1.0 }

    pub fn decal_y() -> f32 { 0.0 }
    pub fn decal_height() -> f32 { 1.0 }
}

#[derive(Debug, Deserialize, Serialize)]
struct MapFile {
    map: String,
    key: Vec<String>,
    cell_size: f32,
    player_spawn: (i32, i32),

    #[serde(default = "defaults::floor")]
    floor: (u8, u8, u8),

    #[serde(default = "defaults::ceiling")]
    ceiling: (u8, u8, u8),

    #[serde(default = "defaults::shade_ceiling")]
    shade_ceiling: bool,

    #[serde(default = "defaults::shade_floor")]
    shade_floor: bool,

    #[serde(default = "defaults::shade_walls")]
    shade_walls: bool,

    #[serde(default = "defaults::shade_multi")]
    shade_multi: f32,

    #[serde(default = "defaults::empty_vec")]
    transparent: Vec<usize>,

    #[serde(default = "defaults::repeating_textures")]
    repeating_textures: HashMap<String, (u32, u32)>,

    #[serde(default = "defaults::wall_height_multi")]
    wall_height_multi: f32,

    #[serde(default = "defaults::empty_vec")]
    decals: Vec<MapFileDecal>,

    #[serde(default = "defaults::empty_vec")]
    billboards: Vec<MapFileBillboard>,

    #[serde(default)]
    connections: MapConnections
}

// pub struct Decal {
//     pub texture: usize,
//     pub p0: (f32, f32),
//     pub p1: (f32, f32),
//     pub y: f32,
//     pub height: f32
// }

#[derive(Debug, Serialize, Deserialize)]
struct MapFileDecal {
    texture: String,
    start: (f32, f32),
    end: (f32, f32),

    #[serde(default = "defaults::decal_y")]
    y: f32,

    #[serde(default = "defaults::decal_height")]
    height: f32
}

// pub struct Billboard {
//     pub texture: usize,
//     pub origin: (f32, f32),
//     pub width: f32,
//     pub y: f32,
//     pub height: f32
// }

#[derive(Debug, Serialize, Deserialize)]
struct MapFileBillboard {
    texture: String,
    pos: (f32, f32),
    width: f32,

    #[serde(default = "defaults::decal_y")]
    y: f32,

    #[serde(default = "defaults::decal_height")]
    height: f32
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct MapConnections {
    #[serde(default)]
    left: MapAdjacentMap,
    #[serde(default)]
    right: MapAdjacentMap,
    #[serde(default)]
    up: MapAdjacentMap,
    #[serde(default)]
    down: MapAdjacentMap
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct MapAdjacentMap {
    map: String,
    offset: (i32, i32)
}