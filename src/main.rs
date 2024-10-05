use anyhow::{Context, Ok};
use sdl2::{image::InitFlag, pixels::Color, rect::Rect, render::BlendMode, sys::{SDL_Delay, SDL_GetTicks}};

mod map;
mod input;
mod player;
mod texture;
mod collision;

const TICK_INTERVAL: u32 = 16;

const MAP: &str = include_str!("map.txt");

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
    
    let map_width = MAP.split('\n').next().unwrap().len() - 1;
    let map_height = MAP.len() / map_width - 1;
    let mut map = map::Map::from_string(&MAP.replace("\r\n", ""), map_width as u32, map_height as u32, 1.0);
    let mut player = player::Player::new((2.0, 2.0));
    let mut input = input::Input::new();

    let mut events = sdl_context.event_pump().unwrap();

    let mut next_time = unsafe { SDL_GetTicks() } + TICK_INTERVAL;

    map.add_texture(texture::Texture::from_file("res/textures/radial_gradient.png", &texture_creator)?);
    map.add_texture(texture::Texture::from_file("res/textures/sky.png", &texture_creator)?);
    map.add_texture(texture::Texture::from_file("res/textures/glassh.png", &texture_creator)?);
    map.add_texture(texture::Texture::from_file("res/textures/warning.png", &texture_creator)?);

    map.floor = Color::RGB(93, 63, 211);
    map.ceiling = Color::GRAY;

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
        input.update();
        player.update(&map, &input);

        /////////////////
        // Render
        /////////////////
        canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
        canvas.clear();

        // Ceiling and floor
        for i in 0..(height as usize) {
            let color = if (i as u32) < height / 2 {
                let shade = map.shade_from_height(height - (2 * i as u32), height);
                Color::RGB((map.ceiling.r as f32 * shade) as u8, (map.ceiling.g as f32 * shade) as u8, (map.ceiling.b as f32 * shade) as u8)
            } else {
                let shade = map.shade_from_height(height - (2 * (height - (i as u32))), height);
                Color::RGB((map.floor.r as f32 * shade) as u8, (map.floor.g as f32 * shade) as u8, (map.floor.b as f32 * shade) as u8)
            };

            canvas.set_draw_color(color);
            canvas.draw_line((0, i as i32), (width as i32, i as i32)).unwrap();
        }

        // Blend mode for shading
        canvas.set_blend_mode(BlendMode::Blend);

        let center = (height as f32) / 2.0;

        // Draw walls
        for col in 0..width {
            let angle = (player.facing - player.fov / 2.0) + (col as f32 / width as f32) * player.fov;

            let raycast_result = map.cast_ray(
                player.position.0, 
                player.position.1, 
                angle.cos(), 
                angle.sin()
            );

            if let Some(result) = raycast_result {
                let distance = ((result.pos.0 - player.position.0).powf(2.0) + (result.pos.1 - player.position.1).powf(2.0)).sqrt();
                let shade_amt = map.shade(distance);

                let line_height = if distance == 0.0 {
                    height
                } else {
                    (height as f32 / distance).max(0.0).min(height as f32) as u32
                };

                let shade = (0, 0, 0, 255 - ((shade_amt * 255.0) as u8));
                let texture = &map.textures[map.get(result.cell.0, result.cell.1) as usize - 1];

                assert!(line_height <= height);

                let start = (center - line_height as f32 / 2.0) as i32;
                let src = Rect::new((texture.width as f32 * result.u) as i32, 0, 1, texture.height);
                let dst = Rect::new(col as i32, start, 1, line_height);

                canvas.copy(&texture.inner, src, dst).unwrap();
                canvas.set_draw_color(shade);
                canvas.draw_line((col as i32, start), (col as i32, start + line_height as i32 - 1)).unwrap();
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