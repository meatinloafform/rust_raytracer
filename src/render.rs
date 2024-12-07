use std::rc::Rc;

use sdl2::{pixels::Color, rect::Rect, render::Canvas, video::Window};
use specs::{World, WorldExt};

use crate::{game::SpriteRenderData, map::{self, GlobalBillboard, Map, RaycastHit}, player::Player, SharedEntityTextures};

impl<'a> map::Map<'a> {
    pub fn render(&self, canvas: &mut Canvas<Window>, player: &mut Player, map_index: usize, maps: &Vec<Rc<Map>>, global_billboards: &Vec<GlobalBillboard>, npc_textures: &SharedEntityTextures, world: &World, capture_frame: bool) {
        let (width, height) = canvas.window().size();

        // Ceiling and floor
        for i in 0..(height as usize) {
            let color = if (i as u32) < height / 2 {
                let shade = if self.effects.shade_ceiling {
                    self.shade_from_height(height - (2 * i as u32), height)
                } else {
                    1.0
                };
                Color::RGB((self.ceiling.r as f32 * shade) as u8, (self.ceiling.g as f32 * shade) as u8, (self.ceiling.b as f32 * shade) as u8)
            } else {
                let shade = if self.effects.shade_floor {
                    self.shade_from_height(height - (2 * (height - (i as u32))), height)
                } else {
                    1.0
                };
                Color::RGB((self.floor.r as f32 * shade) as u8, (self.floor.g as f32 * shade) as u8, (self.floor.b as f32 * shade) as u8)
            };

            canvas.set_draw_color(color);
            canvas.draw_line((0, i as i32), (width as i32, i as i32)).unwrap();
        }

        let center = (height as f32) / 2.0;

        let sprite_render_data = world.read_resource::<SpriteRenderData>();

        for col in 0..width {
            let angle = (player.facing - player.fov / 2.0) + (col as f32 / width as f32) * player.fov;

            let raycast_result = self.cast_ray(
                player.position.0, 
                player.position.1, 
                angle.cos(), 
                angle.sin(),
                0.0,
                map_index,
                &None,
                maps,
                &sprite_render_data.billboards
            );

            if col == width / 2 && capture_frame {
                for (i, hit) in raycast_result.iter().rev().enumerate() {
                    println!("Hit {}: {:?}", i, hit);
                }
            }

            for result in raycast_result.iter().rev() {
                match &result.hit {
                    RaycastHit::Wall(hit_wall) => {
                        // let distance = ((hit_wall.pos.0 - player.position.0).powf(2.0) + (hit_wall.pos.1 - player.position.1).powf(2.0)).sqrt();
                        // let shade_amt = map.shade(distance);
                        let distance = result.distance;
                        let map = maps[result.map].clone();
        
                        let mut real_line_height = 0.0;
                        let line_height = if distance == 0.0 {
                            height
                        } else {
                            let real_height = ((height as f32 / distance) * map.effects.wall_height_multi).max(0.0);
                            if real_height > height as f32 {
                                real_line_height = real_height
                            }

                            real_height.min(height as f32) as u32
                        };
                        
                        let index = map.get(hit_wall.cell.0, hit_wall.cell.1) as usize - 1;
                        //let texture = &mut self.textures[index];
                        
                        let (texture_width, texture_height, color_mod) = {
                            let texture = map.textures[index].borrow();
                            (texture.width, texture.height, texture.color_mod())
                        };

                        assert!(line_height <= height);
        
                        let (start, src);

                        if real_line_height == 0.0 {
                            start = (center - line_height as f32 / 2.0) as i32;
                            src = Rect::new((texture_width as f32 * hit_wall.u) as i32, 0, 1, texture_height);
                        } else {
                            let texture_offset = ((real_line_height - height as f32) / 2.0) * (texture_height as f32 / real_line_height);
                            let texture_height = texture_height as f32 - (2.0 * texture_offset);
                            start = 0;
                            src = Rect::new((texture_width as f32 * hit_wall.u) as i32, texture_offset as i32, 1, texture_height as u32);
                        }

                        let dst = Rect::new(col as i32, start, 1, line_height);
        
                        
                        let original = if map.effects.shade_walls {
                            Some(map.with_wall_texture_color(((color_mod.0 as f32 * result.light.0) as u8, (color_mod.1 as f32 * result.light.1) as u8, (color_mod.2 as f32 * result.light.2) as u8), index))
                        } else {
                            None
                        };

                        canvas.copy(&map.textures[index].borrow().inner, src, dst).unwrap();
                        
                        if let Some(original) = original {
                            map.with_wall_texture_color(original, index);
                        }
                    },
                    RaycastHit::Edge(hit_edge) => {
                        // let distance = ((hit_edge.pos.0 - player.position.0).powf(2.0) + (hit_edge.pos.1 - player.position.1).powf(2.0)).sqrt();
                        let distance = result.distance;

                        let line_height = if distance == 0.0 {
                            height
                        } else {
                            ((height as f32 / distance) * self.effects.wall_height_multi).max(0.0).min(height as f32) as u32
                        };

                        assert!(line_height <= height);
    
                        let start = (center - line_height as f32 / 2.0) as i32;
                        canvas.set_draw_color(Color::BLACK);
                        canvas.draw_line((col as i32, start), (col as i32, start + line_height as i32 - 1)).unwrap();
                    },
                    RaycastHit::Decal(hit_decal) => {
                        // let distance = ((hit_decal.pos.0 - player.position.0).powf(2.0) + (hit_decal.pos.1 - player.position.1).powf(2.0)).sqrt();
                        let distance = result.distance;
                        let shade_amt = self.shade(distance);

                        let mut line_height = if distance == 0.0 {
                            height
                        } else {
                            (height as f32 / distance).max(0.0).min(height as f32) as u32
                        };

                        let index = self.decals[hit_decal.decal_index].texture;
                        // let texture = &self.decal_textures[self.decals[hit_decal.decal_index].texture].0;
                        let (texture_width, texture_height, color_mod) = {
                            let texture = self.decal_textures[index].0.borrow();
                            (texture.width, texture.height, texture.color_mod())
                        };

                        assert!(line_height <= height);

                        line_height = (line_height as f32 * self.decals[hit_decal.decal_index].height) as u32;
                        let height_offset = (line_height as f32 * self.decals[hit_decal.decal_index].y) as i32;
                        let start = (center - line_height as f32 / 2.0) as i32 - height_offset;
                        let src = Rect::new((texture_width as f32 * hit_decal.u) as i32, 0, 1, texture_height);
                        let dst = Rect::new(col as i32, start, 1, line_height);

                        // let color_mod = texture.color_mod();
                        // texture.set_color_mod(((color_mod.0 as f32 * shade_amt) as u8, (color_mod.1 as f32 * shade_amt) as u8, (color_mod.2 as f32 * shade_amt) as u8));
                        // canvas.copy(&texture.inner, src, dst).unwrap();
                        // texture.set_color_mod(color_mod);

                        let original = if self.effects.shade_walls {
                            Some(self.with_decal_texture_color(((color_mod.0 as f32 * result.light.0) as u8, (color_mod.1 as f32 * result.light.1) as u8, (color_mod.2 as f32 * result.light.2) as u8), index))
                        } else {
                            None
                        };

                        canvas.copy(&self.decal_textures[index].0.borrow().inner, src, dst).unwrap();
                        
                        if let Some(original) = original {
                            self.with_decal_texture_color(original, index);
                        }
                    },
                    RaycastHit::Billboard(hit_billboard) => {
                        // let distance = ((hit_billboard.pos.0 - player.position.0).powf(2.0) + (hit_billboard.pos.1 - player.position.1).powf(2.0)).sqrt();
                        let distance = result.distance;
                        let shade_amt = self.shade(distance);

                        let mut line_height = if distance == 0.0 {
                            height
                        } else {
                            (height as f32 / distance).max(0.0).min(height as f32) as u32
                        };

                        if !hit_billboard.global {
                            let index = self.billboards[hit_billboard.billboard_index].texture;
                            let (texture_width, texture_height, color_mod) = {
                                let texture = self.billboard_textures[index].0.borrow();
                                (texture.width, texture.height, texture.color_mod())
                            };

                            assert!(line_height <= height);

                            line_height = (line_height as f32 * self.billboards[hit_billboard.billboard_index].height) as u32;
                            let start = (center - line_height as f32 / 2.0) as i32 - (line_height as f32 * self.billboards[hit_billboard.billboard_index].y) as i32;
                            let src = Rect::new((texture_width as f32 * hit_billboard.u) as i32, 0, 1, texture_height);
                            let dst = Rect::new(col as i32, start, 1, line_height);

                            // TODO: ???? this is wrong i think
                            let original = if self.effects.shade_walls {
                                Some(self.with_billboard_texture_color(((color_mod.0 as f32 * result.light.0) as u8, (color_mod.1 as f32 * result.light.1) as u8, (color_mod.2 as f32 * result.light.2) as u8), index))
                            } else {
                                None
                            };

                            canvas.copy(&self.billboard_textures[index].0.borrow().inner, src, dst).unwrap();
                            
                            if let Some(original) = original {
                                self.with_billboard_texture_color(original, index);
                            }
                        } else {
                            let index = global_billboards[hit_billboard.billboard_index].billboard.texture;
                            let mut texture_ref = npc_textures[index].0.borrow_mut();
                            let mut texture = texture_ref.take_texture();

                            line_height = (line_height as f32 * global_billboards[hit_billboard.billboard_index].billboard.height) as u32;
                            let start = (center - line_height as f32 / 2.0) as i32 - (line_height as f32 * global_billboards[hit_billboard.billboard_index].billboard.y) as i32;
                            let src = Rect::new((texture.width as f32 * hit_billboard.u) as i32, 0, 1, texture.height);
                            let dst = Rect::new(col as i32, start, 1, line_height);

                            let original = texture.color_mod();
                            texture.set_color_mod(((original.0 as f32 * result.light.0) as u8, (original.1 as f32 * result.light.1) as u8, (original.2 as f32 * result.light.2) as u8));

                            canvas.copy(&texture.inner, src, dst).unwrap();

                            texture.set_color_mod(original);
                            drop(texture_ref);

                            npc_textures[index].0.borrow_mut().return_texture(texture);
                        }
                    }
                }
            }
        }
    }
}