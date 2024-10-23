use core::f32;
use core::f32::consts::PI;
use std::rc::Rc;

use nalgebra::vector;
use sdl2::keyboard::Keycode;

use crate::collision;
use crate::input::Input;
use crate::map::{Light, Map};

pub struct Player {
    pub position: (f32, f32),
    pub velocity: (f32, f32),
    pub facing: f32,
    pub forward: (f32, f32),
    pub right: (f32, f32),
    pub speed: f32,
    pub turn_speed: f32,
    pub fov: f32,
    pub radius: f32
}

impl Player {
    pub fn new(position: (f32, f32)) -> Self {
        Self {
            position,
            facing: 0.0,
            forward: (1.0, 0.0),
            fov: PI / 4.0,
            right: (0.0, 1.0),
            speed: 0.1,
            turn_speed: 0.05,
            radius: 0.25,
            velocity: (0.0, 0.0)
        }
    }

    pub fn set_facing(&mut self, facing: f32) {
        self.facing = facing;
        self.forward.0 = facing.cos();
        self.forward.1 = facing.sin();
        self.right.0 = (facing + PI / 2.0).cos();
        self.right.1 = (facing + PI / 2.0).sin();
    }

    pub fn update(&mut self, map: &Map, input: &Input) {
        self.velocity = (0.0, 0.0);
        let mut speed = self.speed;

        if input.get_pressed(Keycode::LShift) {
            speed /= 4.0;
        }

        if input.get_pressed(Keycode::Right) {
            self.set_facing(self.facing + self.turn_speed);
        }
        if input.get_pressed(Keycode::Left) {
            self.set_facing(self.facing - self.turn_speed);
        }

        if input.get_pressed(Keycode::W) {
            self.velocity.0 += self.forward.0 * speed;
            self.velocity.1 += self.forward.1 * speed;
        }
        if input.get_pressed(Keycode::S) {
            self.velocity.0 += -self.forward.0 * speed;
            self.velocity.1 += -self.forward.1 * speed;
        }
        if input.get_pressed(Keycode::D) {
            self.velocity.0 += self.right.0 * speed;
            self.velocity.1 += self.right.1 * speed;
        }
        if input.get_pressed(Keycode::A) {
            self.velocity.0 += -self.right.0 * speed;
            self.velocity.1 += -self.right.1 * speed;
        }

        let magnitude = (self.velocity.0.powf(2.0) + self.velocity.1.powf(2.0)).sqrt() / speed;
        if magnitude > 1.0 {
            self.velocity.0 /= magnitude;
            self.velocity.1 /= magnitude;
        }
        
        let new_pos = collision::slide_move(vector![self.position.0, self.position.1], self.radius, vector![self.velocity.0, self.velocity.1], &map.segments);

        self.position.0 = new_pos.x;
        self.position.1 = new_pos.y;
    }
}