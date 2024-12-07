use sdl2::{rect::Point, render::{Canvas, RenderTarget}, sys::{KeyCode, SDL_KeyCode}};

// fn round_up_mult_8(v: i32) -> i32 {
//     (v + (8 - 1)) & -8
// }

pub fn draw_circle<T: RenderTarget>(canvas: &mut Canvas<T>, cx: i32, cy: i32, r: i32) {
    // let arr_size = round_up_mult_8(r * 8 * 35 / 49);
    let mut points: Vec<Point> = Vec::new();

    // let mut draw_count = 0;

    let diameter = r * 2;

    let mut x = r - 1;
    let mut y = 0;
    let mut tx = 1;
    let mut ty = 1;
    let mut error = tx - diameter;

    while x >= y {
        points.push(Point::new(cx + x, cy - y));
        points.push(Point::new(cx + x, cy + y));
        points.push(Point::new(cx - x, cy - y));
        points.push(Point::new(cx - x, cy + y));
        points.push(Point::new(cx + y, cy - x));
        points.push(Point::new(cx + y, cy + x));
        points.push(Point::new(cx - y, cy - x));
        points.push(Point::new(cx - y, cy + x));

        if error <= 0 {
            y += 1;
            error += ty;
            ty += 2;
        }

        if error > 0 {
            x -= 1;
            tx += 2;
            error += tx - diameter;
        }
    }

    canvas.draw_points(&points[..]).unwrap();
}