use sdl2::{keyboard::Keycode, render::TextureCreator};
use specs::{Builder, World, WorldExt};

use crate::{game, player::Player, ui::UI, State};

pub trait NPCBehavior {
    fn update(&mut self, state: &mut State, ui: &mut UI, player: &mut Player) {}
    fn interact(&mut self, state: &mut State, ui: &mut UI, player: &mut Player) {}
}

pub struct Test {
    id: u32
}

// fn state_insert(npc: Box<dyn NPCBehavior>, state: &mut State) -> usize {
//     for (i, npc_slot) in state.npcs.iter_mut().enumerate() {
//         if let None = npc_slot {
//             *npc_slot = Some(npc);
//             return i;
//         }
//     }

//     state.npcs.push(Some(npc));
//     return state.npcs.len() - 1;
// }

fn state_slot(state: &mut State) -> usize {
    for (i, npc_slot) in state.npcs.iter().enumerate() {
        if let None = npc_slot {
            return i;
        }
    }

    state.npcs.push(None);
    state.npcs.len() - 1
}

impl Test {
    pub fn create<'a, T>(state: &mut State<'a, '_, '_>, creator: &'a TextureCreator<T>, x: f32, y: f32, map: usize) {
        // let texture_id = state.entity_texture(creator, String::from("demon.png"));
        let texture_id = state.animated_entity_texture(creator, String::from("demon"), vec![
            "animated/demon/demon1.png".to_string(),
            "animated/demon/demon2.png".to_string(),
            "animated/demon/demon3.png".to_string(),
            "animated/demon/demon4.png".to_string(),
            "animated/demon/demon5.png".to_string(),
            "animated/demon/demon6.png".to_string(),
            "animated/demon/demon7.png".to_string(),
            "animated/demon/demon8.png".to_string()
        ]);

        let id = state_slot(state);

        let entity = state.world.create_entity()
            .with(game::NPC {
                behavior: id,
                rad: 0.25,
                ..Default::default()
            })
            .with(game::Position {
                x, y, map
            })
            .with(game::Sprite {
                texture: texture_id,
                position: (x, y),
                height: 1.0,
                map,
                width: 1.0,
                y: 0.0
            })
            .with(game::Interactable {
                key: Keycode::E,
                message: String::from("graaahhhh...."),
                radius: 2.0
            }).build();

        let npc = Self {
            id: entity.id()
        };
        
        state.npcs[id] = Some(Box::new(npc));
    }
}

impl NPCBehavior for Test {
    fn interact(&mut self, state: &mut State, ui: &mut UI, player: &mut Player) {
        ui.show_dialog("why did you interact with me... \n i think you probably understand what this means....".to_string());
        player.can_move = false;
    }

    fn update(&mut self, state: &mut State, ui: &mut UI, player: &mut Player) {
        
    }
}