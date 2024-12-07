use std::{cell::RefCell, collections::HashMap, f32::consts::PI, fmt::Debug, path::{Path, PathBuf}, rc::Rc, result, sync::Arc};

use anyhow::{Context, Ok};
use input::Input;
use map::{Light, Map, RaycastHit};
use npc::NPCBehavior;
use player::Player;
use sdl2::{image::InitFlag, keyboard::Keycode, mouse::MouseButton, pixels::Color, rect::Rect, render::{BlendMode, Canvas, RenderTarget, TextureCreator}, sys::{SDL_Delay, SDL_GetTicks}, video::Window};
use specs::{Builder, Dispatcher, DispatcherBuilder, World, WorldExt};
use texture::{AnimationInfo, EntityTexture};
use ui::UI;

mod ui;
mod map;
mod npc;
mod item;
mod game;
mod util;
mod event;
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
    let mut ui = UI::new(&texture_creator)?;
    state.load_map("res/maps/spiral/spiral0.ron", &texture_creator)?;
    npc::Test::create(&mut state, &texture_creator, 2.5, 2.5, 0);
    // state.add_npc("demon.png".to_string(), &texture_creator, (2.5, 2.5), 0);

    let mut player = player::Player::new(state.spawnpoint);
    // player.position = (10.0, 10.0);
    let fireball_texture = state.entity_texture(&texture_creator, "projectiles/fireball.png".to_string());
    player.projectile = fireball_texture;
    // let mut input = input::Input::new();

    let mut events = sdl_context.event_pump().unwrap();

    // Time of next frame
    let mut next_time = unsafe { SDL_GetTicks() } + TICK_INTERVAL;

    'mainloop: loop {
        {
            let input = Arc::get_mut(&mut state.input).expect("hanging reference to input");
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
                    },
                    Event::MouseMotion { x, y, xrel, yrel, .. } => {
                        input.mouse_moved((x.max(0) as u32, y.max(0) as u32), (xrel, yrel));
                    },
                    Event::MouseButtonDown { mouse_btn, clicks, x, y, .. } => {
                        match mouse_btn {
                            MouseButton::Left => input.left_mouse_pressed(),
                            MouseButton::Middle => input.middle_mouse_pressed(),
                            MouseButton::Right => input.right_mouse_pressed(),
                            _ => ()
                        }
                    },
                    Event::MouseButtonUp { mouse_btn, clicks, x, y, .. } => {
                        match mouse_btn {
                            MouseButton::Left => input.left_mouse_released(),
                            MouseButton::Middle => input.middle_mouse_released(),
                            MouseButton::Right => input.right_mouse_released(),
                            _ => ()
                        }
                    }
                    _ => ()
                }
            }
        }

        /////////////////
        // Update
        /////////////////

        // Player input
        player.update(&mut state);

        ui.update(&player, &state.input);
        state.update(&mut player, &mut ui);
        
        if ui.dialog.unfreeze_player {
            ui.dialog.unfreeze_player = false;
            player.can_move = true;
            state.allow_interaction = true;
        }

        // Toggle minimap
        if state.input.get_just_pressed(Keycode::M) {
            state.minimap_enabled = !state.minimap_enabled;
        }

        // Capture frame
        state.capture_frame = false;
        if state.input.get_just_pressed(Keycode::C) {
            state.capture_frame = true;
        }

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

        /////////////////
        // UI
        /////////////////
        ui.draw(&state.input, &player, &mut canvas);

        if let Some(input) = Arc::get_mut(&mut state.input) {
            input.update();
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

impl<'a, 'b, 'c> State<'a, 'b, 'c> {
    pub fn new() -> Self {
        let mut world = World::new();
        world.register::<game::Position>();
        world.register::<game::Player>();
        world.register::<game::NPC>();
        world.register::<game::Sprite>();
        world.register::<game::AIStyle>();
        world.register::<game::Interactable>();
        world.register::<game::Projectile>();

        world.create_entity()
            .with(game::Position {
                x: 0.0, y: 0.0, map: 0
            }).with(game::Player {
                map: 0
            })
            .build();

        world.insert(game::PlayerData {
            ..Default::default()
        });

        world.insert(game::SpriteRenderData {
            ..Default::default()
        });

        world.insert(game::Time {
            ..Default::default()
        });

        world.insert(game::EventBus {
            events: Vec::new()
        });

        world.insert(game::Input {
            input: None
        });

        let mut dispatcher = DispatcherBuilder::new()
            .with(game::UpdatePlayer, "update_player", &[])
            .with(game::AIStep, "ai_step", &[])
            .with(game::CheckInteractions, "check_interactions", &["update_player", "ai_step"])
            .with(game::MoveSprites, "move_sprites", &["ai_step"])
            .with(game::PrepareSprites, "prepare_sprites", &["move_sprites"])
            .with(game::MoveCollideProjectiles, "move_collide_projectiles", &[])
            .build();

        let mut state = Self {
            minimap_enabled: false,
            loaded_maps: Vec::new(),
            current_map: 0,
            spawnpoint: (0.0, 0.0),
            player_light: 0,
            world,
            dispatcher,
            entity_textures: Vec::new(),
            input: Arc::new(Input::new()),
            npcs: Vec::new(),
            allow_interaction: true,
            capture_frame: false
        };

        state
    }

    fn post_load(&mut self) {
        self.world.insert(game::Maps::new(self));
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

        self.post_load();

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

    pub fn update(&mut self, player: &mut Player, ui: &mut UI) {
        if ui.dialog.active {
            self.allow_interaction = false;
        }

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

        {
            let mut player_data = self.world.write_resource::<game::PlayerData>();
            player_data.x = player.position.0;
            player_data.y = player.position.1;
            player_data.map = self.current_map;
        }

        let tick = unsafe {
            SDL_GetTicks()
        } as f64 / 1000.0;

        {
            let mut time = self.world.write_resource::<game::Time>();
            time.elapsed_time = tick;
        }

        for i in 0..self.npcs.len() {
            if self.npcs[i].is_some() {
                let mut npc = self.npcs[i].take().unwrap();
                npc.update(self, ui, player);
                self.npcs[i] = Some(npc);
            }
        }

        {
            let mut input_container = self.world.write_resource::<game::Input>();
            input_container.input = Some(self.input.clone())
        }

        self.dispatcher.dispatch(&mut self.world);
        self.world.maintain();

        {
            let mut input_container = self.world.write_resource::<game::Input>();
            input_container.input = None;
        }

        let events = {
            self.world.write_resource::<game::EventBus>().events.drain(..).collect::<Vec<_>>()
        };

        for event in events.into_iter() {
            use event::Event::*;
            match event {
                ShowInteractionPrompt { message, key } => {
                    ui.interaction_prompt_message = Some((message, key));
                },
                Print { message } => {
                    println!("{}", message);
                },
                DoInteraction { npc } => {
                    if self.allow_interaction {
                        let mut npc_obj = self.npcs[npc].take().unwrap();
                        npc_obj.interact(self, ui, player);
                        self.npcs[npc] = Some(npc_obj);
                    }
                },
                ShowDialog { text } => {
                    ui.show_dialog(text);
                    player.can_move = false
                }
            }
        }

        for texture in self.entity_textures.iter() {
            texture.0.borrow_mut().try_animate();
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
            let global_billboards = self.world.fetch::<game::SpriteRenderData>();

            base_map.render(canvas, player, self.current_map, &map_rcs, &global_billboards.billboards, &self.entity_textures, &self.world, self.capture_frame);
        }

        for (i, rc) in map_rcs.drain(..).enumerate() {
            if let result::Result::Ok(map) = Rc::try_unwrap(rc) {
                self.loaded_maps[i] = Some(map);
            } else {
                panic!("outlying references to map still existed at the end of render");
            }
        }
    }

    pub fn entity_texture<T>(&mut self, creator: &'a TextureCreator<T>, name: String) -> usize {
        for (i, texture) in self.entity_textures.iter().enumerate() {
            if texture.1 == name {
                return i;
            }
        }

        let path = PathBuf::from("res/textures/").join(&name);

        self.entity_textures.push((
            Rc::new(RefCell::new(EntityTexture::Static(Some(texture::Texture::from_file(path, creator).unwrap())))),
            name
        ));

        self.entity_textures.len() - 1
    }

    pub fn animated_entity_texture<T>(&mut self, creator: &'a TextureCreator<T>, name: String, textures: Vec<String>) -> usize {
        for (i, texture) in self.entity_textures.iter().enumerate() {
            if texture.1 == name {
                return i;
            }
        }

        let mut loaded_textures = Vec::new();

        for texture in textures.iter() {
            let path = PathBuf::from("res/textures/").join(texture);
            
            loaded_textures.push(
                Some(texture::Texture::from_file(path, creator).unwrap())
            );
        }

        self.entity_textures.push((
            Rc::new(RefCell::new(EntityTexture::Animated(loaded_textures, AnimationInfo {
                frame: 0,
                frame_count: textures.len()
            }))),
            name
        ));

        self.entity_textures.len() - 1
    }
}

// type NpcTextures<'a> = Vec<(Rc<RefCell<texture::Texture<'a>>>, String)>;
type SharedEntityTextures<'a> = Vec<(Rc<RefCell<texture::EntityTexture<'a>>>, String)>;

struct State<'a, 'b, 'c> {
    pub minimap_enabled: bool,
    pub loaded_maps: Vec<Option<Map<'a>>>,
    pub current_map: usize,
    pub spawnpoint: (f32, f32),
    pub player_light: usize,
    pub world: World,
    pub dispatcher: Dispatcher<'b, 'c>,
    // pub npc_textures: NpcTextures<'a>,
    pub entity_textures: SharedEntityTextures<'a>,
    pub input: Arc<Input>,
    pub npcs: Vec<Option<Box<dyn NPCBehavior>>>,
    pub allow_interaction: bool,
    pub capture_frame: bool
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