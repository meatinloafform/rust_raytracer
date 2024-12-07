use std::{rc::Rc, sync::Arc};

use sdl2::{keyboard::Keycode};
use specs::{world::EntitiesRes, Component, DenseVecStorage, Join, LendJoin, Read, ReadStorage, System, VecStorage, Write, WriteStorage};

use crate::{event, input, map::{Billboard, GlobalBillboard, Map}, ui, State};

///////////////
// Resources
///////////////

#[derive(Default)]
pub struct PlayerData {
    pub x: f32,
    pub y: f32,
    pub map: usize,
    pub rad: f32
}

#[derive(Default)]
pub struct SpriteRenderData {
    pub billboards: Vec<GlobalBillboard>
}

#[derive(Default)]
pub struct Time {
    pub elapsed_time: f64
}

#[derive(Default)]
pub struct Input {
    pub input: Option<Arc<input::Input>>
}

#[derive(Default)]
pub struct EventBus {
    pub events: Vec<event::Event>
}

#[derive(Default)]
pub struct Maps {
    pub maps: Vec<MapRepr>
}

impl Maps {

    /// Load all maps into their ECS representation
    pub fn new(state: &State) -> Self {
        let mut maps = Vec::new();

        for map in state.loaded_maps.iter() {
            let mut tiles = Vec::new();

            if let Some(map) = map {
                for y in 0..map.height {
                    for x in 0..map.width {
                        let cell = map.get(x, y);

                        if cell == 0 {
                            tiles.push(TileRepr::None);
                        } else if map.transparent[(cell - 1) as usize] {
                            tiles.push(TileRepr::Transparent);
                        } else {
                            tiles.push(TileRepr::Solid);
                        }
                    }
                }

                maps.push(MapRepr {
                    tiles,
                    cell_size: map.cell_size,
                    height: map.height,
                    width: map.width
                });
            } else {
                eprintln!("Map was unloaded during creation of resource");
            }
        }

        Self {
            maps
        }
    }
}

enum TileRepr {
    None,
    Transparent,
    Solid
}

struct MapRepr {
    tiles: Vec<TileRepr>,
    width: u32,
    height: u32,
    cell_size: f32
}

///////////////
// Components
///////////////

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub map: usize
}

#[derive(Component, Debug, Default)]
pub struct Player {
    pub map: usize
}

// TODO: Radius
#[derive(Component, Debug, Default)]
pub struct NPC {
    pub max_life: f32,
    pub life: f32,
    pub behavior: usize,
    pub rad: f32
}

#[derive(Component, Debug, Default)]
pub struct Sprite {
    pub texture: usize,
    pub position: (f32, f32),
    pub width: f32,
    pub y: f32,
    pub height: f32,
    pub map: usize
}

#[derive(Debug, Component)]
pub enum AIStyle {
    Circles((f32, f32))
}

#[derive(Debug, Component)]
pub struct Interactable {
    pub key: Keycode,
    pub radius: f32,
    pub message: String,
}

#[derive(Debug, Component)]
pub struct Projectile {
    pub vel: (f32, f32),
    pub rad: f32,
    pub friendly: bool,
    pub hostile: bool
}

///////////////
// Systems
///////////////

pub struct AIStep;

impl<'a> System<'a> for AIStep {
    type SystemData = (
        Read<'a, Time>,
        ReadStorage<'a, AIStyle>,
        WriteStorage<'a, Position>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (time, ai_style, mut position) = data;

        for (ai_style, pos) in (&ai_style, &mut position).join() {
            match ai_style {
                AIStyle::Circles(from) => {
                    pos.x = from.0 + (time.elapsed_time.cos() as f32) - 0.5;
                    pos.y = from.1 + (time.elapsed_time.sin() as f32) - 0.5;
                }
            }
        }
    }
}

pub struct UpdatePlayer;

impl<'a> System<'a> for UpdatePlayer {
    type SystemData = 
    (
        Read<'a, PlayerData>,
        WriteStorage<'a, Player>,
        WriteStorage<'a, Position>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (player_data, mut player, mut pos) = data;

        for (player, pos) in (&mut player, &mut pos).join() {
            pos.x = player_data.x;
            pos.y = player_data.y;
            player.map = player_data.map;
        }
    }
}

pub struct PrepareSprites;

impl<'a> System<'a> for PrepareSprites {
    type SystemData = (
        Write<'a, SpriteRenderData>,
        ReadStorage<'a, Sprite>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut render_data, sprite) = data;

        render_data.billboards.clear();

        for sprite in sprite.join() {
            render_data.billboards.push(GlobalBillboard { 
                billboard: Billboard {
                    height: sprite.height,
                    origin: sprite.position,
                    texture: sprite.texture,
                    width: sprite.width,
                    y: sprite.y
                },
                map: sprite.map 
            });
        }
    }
}

pub struct MoveSprites;

impl<'a> System<'a> for MoveSprites {
    type SystemData = (
        ReadStorage<'a, Position>,
        WriteStorage<'a, Sprite>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (position, mut sprite) = data;

        for (pos, sprite) in (&position, &mut sprite).join() {
            sprite.position = (pos.x, pos.y)
        }
    }
}

pub struct CheckInteractions;

impl<'a> System<'a> for CheckInteractions {
    type SystemData = (
        Read<'a, Input>,
        Read<'a, PlayerData>,
        Write<'a, EventBus>,
        ReadStorage<'a, NPC>,
        ReadStorage<'a, Interactable>,
        ReadStorage<'a, Position>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (input, player, mut bus, npc, interact, position) = data;

        for (interactable, npc, position) in (&interact, &npc, &position).join() {
            if ((position.x - player.x).powf(2.0) + (position.y - player.y).powf(2.0)).sqrt() < interactable.radius {
                if input.input.as_ref().unwrap().get_just_pressed(interactable.key) {
                    bus.events.push(event::Event::DoInteraction { npc: npc.behavior });
                } else {
                    if position.map == player.map {
                        bus.events.push(
                            event::Event::ShowInteractionPrompt { 
                                message: interactable.message.clone(),
                                key: interactable.key.name()
                            }
                        );
                    }
                }
            }
        }
    }
}

pub struct MoveCollideProjectiles;

impl<'a> System<'a> for MoveCollideProjectiles {
    type SystemData = (
        Read<'a, EntitiesRes>,
        Read<'a, Maps>,
        Write<'a, PlayerData>,
        WriteStorage<'a, Projectile>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, NPC>,
    );

    // Move projectiles
    // Projectile-Bounds
    // Projectile-Wall
    // Projectile-NPC
    // Projectile-Player
    fn run(&mut self, data: Self::SystemData) {
        let (entities_res, maps, mut player, mut projectile, mut position, mut npc) = data;

        // Move projectiles
        for (proj, pos, entity) in (&projectile, &mut position, &entities_res).join() {
            pos.x += proj.vel.0;
            pos.y += proj.vel.1;

            let map = &maps.maps[pos.map];

            // TODO: Edges
            // Bounds check
            if pos.x < 0.0 {
                entities_res.delete(entity).unwrap();
            } else if pos.y < 0.0 {
                entities_res.delete(entity).unwrap();
            } else if pos.x > map.width as f32 * map.cell_size {
                entities_res.delete(entity).unwrap();
            } else if pos.y > map.height as f32 * map.cell_size {
                entities_res.delete(entity).unwrap();
            }

            let tile_pos_x = ((pos.x / map.cell_size).floor() * map.cell_size) as u32 + 1;
            let tile_pos_y = ((pos.y / map.cell_size).floor() * map.cell_size) as u32 + 1;

            if let TileRepr::Solid | TileRepr::Transparent = map.tiles[(tile_pos_y * map.width + tile_pos_x) as usize] {
                entities_res.delete(entity).unwrap();
            }
        }



        for (projectile, proj_entity) in (&projectile, &entities_res).join() {
            if let Some(proj_pos) = position.get(proj_entity) {
                for (npc, npc_entity) in (&npc, &entities_res).join() {
                    if let Some(npc_pos) = position.get(npc_entity) {
                        if projectile.friendly && (npc_pos.x - proj_pos.x).powf(2.0) + (npc_pos.y - proj_pos.y).powf(2.0) < (projectile.rad + npc.rad).powf(2.0) {
                            entities_res.delete(proj_entity).unwrap()
                        }
                    }
                }

                if projectile.hostile && (player.x - proj_pos.x).powf(2.0) + (player.y - proj_pos.y).powf(2.0) < (projectile.rad + player.rad) {
                    entities_res.delete(proj_entity).unwrap();
                }
            }
        }
    }
}