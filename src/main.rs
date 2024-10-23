use std::{f32::consts::PI, fmt::Debug, path::{Path, PathBuf}, rc::Rc, result};

use anyhow::{Context, Ok};
use map::{Light, Map, RaycastHit};
use player::Player;
use sdl2::{image::InitFlag, keyboard::Keycode, pixels::Color, rect::Rect, render::{BlendMode, Canvas, RenderTarget, TextureCreator}, sys::{SDL_Delay, SDL_GetTicks}, video::Window};
use ui::UI;

mod ui;
mod map;
mod util;
mod input;
mod player;
mod render;
mod texture;
mod collision;

/// Milliseconds per frame
const TICK_INTERVAL: u32 = 16;

/// Minimap width (px)
const MINIMAP_WIDTH: u32 = 200;

/// Minimap height (px)
const MINIMAP_HEIGHT: u32 = 200;

/// Pixels per cell on minimap
const MINIMAP_SCALE: f32 = 25.0;

fn find_sdl_gl_driver() -> Option<u32> {
    for (index, item) in sdl2::render::drivers().enumerate() {
        if item.name == "opengl" {
            return Some(index as u32);
        }
    }

    None
}

fn main() -> anyhow::Result<()> {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let _image_context = sdl2::image::init(InitFlag::PNG | InitFlag::JPG);
    let window = video_subsystem
        .window("ion know", 640, 480)
        .opengl()
        .position_centered()
        .build()?;

    let mut canvas = window
        .into_canvas()
        .index(find_sdl_gl_driver().context("sdl gl driver")?)
        .target_texture()
        .build()?;

    let (width, height) = canvas.window().size();

    let texture_creator = canvas.texture_creator();

    let mut state = State::new();
    let mut ui = UI::new(&texture_creator);
    state.load_map("res/maps/spiral/spiral0.ron", &texture_creator)?;

    let mut player = player::Player::new(state.spawnpoint);
    let mut input = input::Input::new();

    let mut events = sdl_context.event_pump().unwrap();

    // Time of next frame
    let mut next_time = unsafe { SDL_GetTicks() } + TICK_INTERVAL;

    'mainloop: loop {
        for event in events.poll_iter() {
            use sdl2::event::Event;
            match event {
                Event::Quit { .. } => break 'mainloop,
                Event::KeyDown { keycode: Some(keycode), repeat, .. } => {
                    if !repeat {
                        input.pressed(keycode);
                    }
                },
                Event::KeyUp { keycode: Some(keycode), .. } => {
                    input.released(keycode);
                }
                _ => ()
            }
        }

        /////////////////
        // Update
        /////////////////

        // Player input
        player.update(state.loaded_maps[state.current_map].as_ref().unwrap(), &input);

        // Map update
        state.update(&mut player);

        // Toggle minimap
        if input.get_just_pressed(Keycode::M) {
            state.minimap_enabled = !state.minimap_enabled;
        }

        input.update();

        /////////////////
        // Render
        /////////////////
        canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
        canvas.clear();

        // Draw maps
        state.render(&mut canvas, &mut player);

        // Draw minimap
        {
            let map = state.loaded_maps[state.current_map].as_ref().unwrap();

            if state.minimap_enabled {
                let minimap_x = width - MINIMAP_WIDTH - 10;
                let minimap_y = height - MINIMAP_HEIGHT - 10;
                let mut player_offset_x = 0.0;
                let mut player_offset_y = 0.0;
                let mut minimap_offset_x = player.position.0 * MINIMAP_SCALE - (MINIMAP_WIDTH as f32 / 2.0) + MINIMAP_SCALE * map.cell_size - 1.0;
                let mut minimap_offset_y = player.position.1 * MINIMAP_SCALE - (MINIMAP_HEIGHT as f32 / 2.0) + MINIMAP_SCALE * map.cell_size - 1.0;
                let minimap_rect = Rect::new(minimap_x as i32, minimap_y as i32, MINIMAP_WIDTH, MINIMAP_HEIGHT);
                
                // Clamp minimap drawing
                if minimap_offset_x < 0.0 {
                    player_offset_x = minimap_offset_x;
                    minimap_offset_x = 0.0;
                }

                if minimap_offset_y < 0.0 {
                    player_offset_y = minimap_offset_y;
                    minimap_offset_y = 0.0;
                }

                let max_x = map.width as f32 * map.cell_size * MINIMAP_SCALE;
                if minimap_offset_x + MINIMAP_WIDTH as f32 > max_x {
                    player_offset_x = (minimap_offset_x + MINIMAP_WIDTH as f32) - max_x;
                    minimap_offset_x = max_x - MINIMAP_WIDTH as f32;
                }

                let max_y = map.height as f32 * map.cell_size * MINIMAP_SCALE;
                if minimap_offset_y + MINIMAP_HEIGHT as f32 > max_y {
                    player_offset_y = (minimap_offset_y + MINIMAP_HEIGHT as f32) - max_y;
                    minimap_offset_y = max_y - MINIMAP_HEIGHT as f32;
                }

                canvas.set_draw_color(map.floor);
                canvas.fill_rect(minimap_rect).unwrap();

                canvas.set_clip_rect(Rect::new(
                    minimap_rect.x + 1, minimap_rect.y + 1,
                    minimap_rect.width() - 1, minimap_rect.height() - 1
                ));
                // canvas.set_draw_color(Color::RGB(0, 128, 0));
                canvas.set_draw_color(Color::WHITE);

                for y in 0..map.height {
                    for x in 0..map.width {
                        let cell = map.get(x, y);
                        if cell == 0 { continue; }

                        let mut top_left_x = (x as f32 * MINIMAP_SCALE * map.cell_size) - minimap_offset_x as f32;
                        let mut top_left_y = (y as f32 * MINIMAP_SCALE * map.cell_size) - minimap_offset_y as f32;
                        let mut bottom_right_x = top_left_x + MINIMAP_SCALE * map.cell_size;
                        let mut bottom_right_y = top_left_y + MINIMAP_SCALE * map.cell_size;

                        if !(top_left_x > MINIMAP_WIDTH as f32 || top_left_y > MINIMAP_HEIGHT as f32 || bottom_right_x < 0.0 || bottom_right_y < 0.0) {
                            top_left_x = top_left_x.max(0.0);
                            top_left_y = top_left_y.max(0.0);
                            bottom_right_x = bottom_right_x.min(MINIMAP_WIDTH as f32);
                            bottom_right_y = bottom_right_y.min(MINIMAP_HEIGHT as f32);

                            canvas.copy(&map.textures[(cell - 1) as usize].borrow().inner, None, Rect::new(
                                (top_left_x + minimap_x as f32) as i32 - 1,
                                (top_left_y + minimap_y as f32) as i32 - 1,
                                (bottom_right_x - top_left_x) as u32 + 1,
                                (bottom_right_y - top_left_y) as u32 + 1
                            )).unwrap();
                        }
                    }
                }

                // Draw player in minimap
                let cx = (minimap_x + (MINIMAP_WIDTH / 2)) as i32 + player_offset_x as i32;
                let cy = (minimap_y + (MINIMAP_HEIGHT / 2)) as i32 + player_offset_y as i32;
                util::draw_circle(&mut canvas, cx, cy, (player.radius * MINIMAP_SCALE) as i32);
                util::draw_circle(&mut canvas, cx, cy, (player.radius * MINIMAP_SCALE) as i32 + 1);
                canvas.draw_line(
                    (cx, cy),
                    (cx + (player.forward.0 * MINIMAP_SCALE * player.radius) as i32, cy + (player.forward.1 * MINIMAP_SCALE * player.radius) as i32)   
                ).unwrap();

                canvas.set_clip_rect(None);

                canvas.set_draw_color(Color::BLACK);
                canvas.draw_rect(minimap_rect).unwrap();
            }

        }

        canvas.present();

        // Wait until the next frame
        unsafe {
            let time = time_left(next_time);
            SDL_Delay(time);
            next_time += TICK_INTERVAL;
        }
    }

    Ok(())
}

unsafe fn time_left(next_time: u32) -> u32 {
    let now = SDL_GetTicks();
    if next_time <= now {
        0
    } else {
        next_time - now
    }
}

impl<'a> State<'a> {
    pub fn new() -> Self {
        Self {
            minimap_enabled: false,
            loaded_maps: Vec::new(),
            current_map: 0,
            spawnpoint: (0.0, 0.0),
            player_light: 0
        }
    }

    pub fn load_map<P: AsRef<Path> + Debug, T>(&mut self, path: P, creator: &'a TextureCreator<T>) -> anyhow::Result<()> {
        let mut new_vec = Vec::new();

        self._load_map(path, &mut new_vec, creator)?;

        self.loaded_maps = new_vec.into_iter().map(|a| Some(a)).collect();

        self.loaded_maps[0].as_mut().unwrap().lights.push(Light::player());
        self.player_light = self.loaded_maps[0].as_ref().unwrap().lights.len() - 1;

        for map in self.loaded_maps.iter_mut() {
            map.as_mut().unwrap().regenerate_segments();
        }

        anyhow::Ok(())
    }

    fn _load_map<P: AsRef<Path> + Debug, T>(&mut self, path: P, loaded_maps: &mut Vec<Map<'a>>, creator: &'a TextureCreator<T>) -> anyhow::Result<usize> {
        println!("Loading: {:?}", &path);

        let map_index = loaded_maps.len();
        let mut map = map::Map::from_path(path, creator)?;
        map.index = loaded_maps.len(); 
        loaded_maps.push(map);

        if loaded_maps[map_index].load_adjacent.up.is_some() {
            let path = loaded_maps[map_index].load_adjacent.up.take().unwrap();

            let index = self.load_or_get_index(path.0, loaded_maps, creator)?;

            loaded_maps[map_index].adjacent_maps.storage.up = Some((
                index, path.1, (loaded_maps[index].width, loaded_maps[index].height)
            ));
        }

        if loaded_maps[map_index].load_adjacent.down.is_some() {
            let path = loaded_maps[map_index].load_adjacent.down.take().unwrap();

            let index = self.load_or_get_index(path.0, loaded_maps, creator)?;

            loaded_maps[map_index].adjacent_maps.storage.down = Some((
                index, path.1, (loaded_maps[index].width, loaded_maps[index].height)
            ));
        }

        if loaded_maps[map_index].load_adjacent.left.is_some() {
            let path = loaded_maps[map_index].load_adjacent.left.take().unwrap();

            let index = self.load_or_get_index(path.0, loaded_maps, creator)?;

            loaded_maps[map_index].adjacent_maps.storage.left = Some((
                index, path.1, (loaded_maps[index].width, loaded_maps[index].height)
            ));
        }

        if loaded_maps[map_index].load_adjacent.right.is_some() {
            let path = loaded_maps[map_index].load_adjacent.right.take().unwrap();

            let index = self.load_or_get_index(path.0, loaded_maps, creator)?;

            loaded_maps[map_index].adjacent_maps.storage.right = Some((
                index, path.1, (loaded_maps[index].width, loaded_maps[index].height)
            ));
        }


        anyhow::Ok(map_index)
    }

    fn load_or_get_index<P: AsRef<Path> + Debug, T>(&mut self, path: P, loaded_maps: &mut Vec<Map<'a>>, creator: &'a TextureCreator<T>) -> anyhow::Result<usize> {
        let target_name = path.as_ref().file_name().context("failed to get file name")?.to_str().context("failed to convert string")?.to_string();

        for i in 0..loaded_maps.len() {
            if loaded_maps[i].name == target_name {
                return anyhow::Ok(i);
            }
        }

        let index = self._load_map(&path, loaded_maps, creator)?;

        anyhow::Ok(index)
    }

    pub fn update(&mut self, player: &mut Player) {
        self.loaded_maps[self.current_map].as_mut().unwrap().lights[self.player_light].pos = player.position;

        let (width, height, cell_size) = {
            let map = self.loaded_maps[self.current_map].as_ref().unwrap();
            (map.width as f32 * map.cell_size, map.height as f32 * map.cell_size, map.cell_size)
        };

        if player.position.0 < -cell_size && self.loaded_maps[self.current_map].as_ref().unwrap().adjacent_maps.storage.left.is_some() {
            let adj = self.loaded_maps[self.current_map].as_ref().unwrap().adjacent_maps.storage.left.unwrap();
            let cell_size = self.loaded_maps[adj.0].as_ref().unwrap().cell_size;
            let offset = (adj.1.0 as f32 * cell_size, adj.1.1 as f32 * cell_size);
            let new_width = adj.2.0 as f32 * cell_size;
            self.switch_maps(player, adj.0, (new_width + player.position.0 - offset.0, player.position.1 - offset.1));
        } else if player.position.0 > (width - cell_size) && self.loaded_maps[self.current_map].as_ref().unwrap().adjacent_maps.storage.right.is_some() {
            let adj = self.loaded_maps[self.current_map].as_ref().unwrap().adjacent_maps.storage.right.unwrap();
            let cell_size = self.loaded_maps[adj.0].as_ref().unwrap().cell_size;
            let offset = (adj.1.0 as f32 * cell_size, adj.1.1 as f32 * cell_size);
            self.switch_maps(player, adj.0, (player.position.0 - width - offset.0, player.position.1 - offset.1));
        } else if player.position.1 < -cell_size && self.loaded_maps[self.current_map].as_ref().unwrap().adjacent_maps.storage.up.is_some() {
            let adj = self.loaded_maps[self.current_map].as_ref().unwrap().adjacent_maps.storage.up.unwrap();
            let cell_size = self.loaded_maps[adj.0].as_ref().unwrap().cell_size;
            let offset = (adj.1.0 as f32 * cell_size, adj.1.1 as f32 * cell_size);
            let new_height = adj.2.1 as f32 * cell_size;
            self.switch_maps(player, adj.0, (player.position.0 - offset.0, new_height + player.position.1 - offset.1));
        } else if player.position.1 > (height - cell_size) && self.loaded_maps[self.current_map].as_ref().unwrap().adjacent_maps.storage.down.is_some() {
            let adj = self.loaded_maps[self.current_map].as_ref().unwrap().adjacent_maps.storage.down.unwrap();
            let cell_size = self.loaded_maps[adj.0].as_ref().unwrap().cell_size;
            let offset = (adj.1.0 as f32 * cell_size, adj.1.1 as f32 * cell_size);
            self.switch_maps(player, adj.0, (player.position.0 - offset.0, player.position.1 - height - offset.1));
        }
    }

    pub fn switch_maps(&mut self, player: &mut Player, new_index: usize, new_position: (f32, f32)) {
        let mut player_light = self.loaded_maps[self.current_map].as_mut().unwrap().lights.remove(self.player_light);
        self.current_map = new_index;
        player.position = new_position;
        player_light.pos = new_position;
        self.loaded_maps[self.current_map].as_mut().unwrap().lights.push(player_light);
        self.player_light = self.loaded_maps[self.current_map].as_ref().unwrap().lights.len() - 1;
    }

    pub fn render(&mut self, canvas: &mut Canvas<Window>, player: &mut Player) {
        let mut map_rcs = self.loaded_maps.iter_mut().map(|a| Rc::new(a.take().unwrap())).collect::<Vec<_>>();

        {
            let base_map = map_rcs[self.current_map].clone();

            // TODO: Move the render code either to a function or to the implementation of Map
            base_map.render(canvas, player, self.current_map, &map_rcs);
        }

        for (i, rc) in map_rcs.drain(..).enumerate() {
            if let result::Result::Ok(map) = Rc::try_unwrap(rc) {
                self.loaded_maps[i] = Some(map);
            } else {
                panic!("outlying references to map still existed at the end of render");
            }
        }
    }

    // pub fn add_light(&mut self, light: Light) -> usize {
    //     for (i, light_slot) in self.lights.iter_mut().enumerate() {
    //         if light_slot.is_none() {
    //             *light_slot = Some(light);
    //             return i;
    //         }
    //     }

    //     self.lights.push(Some(light));
    //     return self.lights.len() - 1;
    // }
}

struct State<'a> {
    pub minimap_enabled: bool,
    pub loaded_maps: Vec<Option<Map<'a>>>,
    pub current_map: usize,
    pub spawnpoint: (f32, f32),
    pub player_light: usize
}

//////////////
// TODO
//////////////
// 
// Cross-map lighting
// right now it only works on maps immediately adjacent
//
// Fix minimap for small maps
// Address minimap during crossmap changes