use std::f32::consts::FRAC_1_SQRT_2;

use nalgebra::{vector, Vector2};

// http://code.alaiwan.org/blog/collision-disk.html
pub struct CollisionResult {
    pub depth: f32,
    pub normal: Vector2<f32>
}

fn closest_point_on_segment(pos: Vector2<f32>, s0: Vector2<f32>, s1: Vector2<f32>) -> Vector2<f32> {
    let tangent = s1 - s0;

    if (pos - s0).dot(&tangent) <= 0.0 {
        return s0;
    }

    if (pos - s1).dot(&tangent) >= 0.0 {
        return s1;
    }

    let t = tangent.normalize(); //tangent * (1.0 / tangent.magnitude());
    let relative_pos = pos - s0;

    s0 + t * (t.dot(&relative_pos))
}

fn collide_circle_with_segment(circle: Vector2<f32>, r: f32, s0: Vector2<f32>, s1: Vector2<f32>) -> Option<CollisionResult> {
    let delta = circle - closest_point_on_segment(circle, s0, s1);


    // V dot V gives |v| ^ 2
    if delta.dot(&delta) > r * r {
        return None;
    }

    let dist = delta.magnitude();

    if dist == 0.0 {
        // If the circle is perfectly in the center just fallback to pushing it forward in both axes ig
        Some(CollisionResult {
            depth: r,
            normal: vector![FRAC_1_SQRT_2, FRAC_1_SQRT_2]
        })
    } else {
        let normal = delta * (1.0 / dist); 

        Some(CollisionResult {
            depth: r - dist,
            normal
        })
    }
}

fn collide_with_segments(pos: Vector2<f32>, r: f32, segments: &[(Vector2<f32>, Vector2<f32>)]) -> Option<CollisionResult> {
    let mut earliest_collision: Option<CollisionResult> = None;

    for segment in segments.iter() {
        if let Some(collision) = collide_circle_with_segment(pos, r, segment.0, segment.1) {
            if let Some(ref mut earliest) = earliest_collision {
                if collision.depth > earliest.depth {
                    *earliest = collision;
                }
            } else {
                earliest_collision = Some(collision);
            }
        }
    }

    earliest_collision
}

/// How many times to repeat the collision process
const COLLISION_RESOLUTION: usize = 5;

/// Moves pos according to vel while colliding and sliding against all segments
pub fn slide_move(mut pos: Vector2<f32>, r: f32, vel: Vector2<f32>, segments: &[(Vector2<f32>, Vector2<f32>)]) -> Vector2<f32> {
    pos += vel;

    for _ in 0..COLLISION_RESOLUTION {
        if let Some(collision) = collide_with_segments(pos, r, segments) {
            pos += collision.normal * collision.depth;
        } else {
            break;
        }
    }

    pos
}

// https://stackoverflow.com/questions/563198/how-do-you-detect-where-two-line-segments-intersect
/// Returns: x, y, s, t
pub fn get_line_intersection(
    p0_x: f32, p0_y: f32, p1_x: f32, p1_y: f32, 
    p2_x: f32, p2_y: f32, p3_x: f32, p3_y: f32
) -> Option<(f32, f32, f32, f32)> {
    let s1_x = p1_x - p0_x;
    let s1_y = p1_y - p0_y;
    let s2_x = p3_x - p2_x;
    let s2_y = p3_y - p2_y;

    let s = (-s1_y * (p0_x - p2_x) + s1_x * (p0_y - p2_y)) / (-s2_x * s1_y + s1_x * s2_y);
    let t = ( s2_x * (p0_y - p2_y) - s2_y * (p0_x - p2_x)) / (-s2_x * s1_y + s1_x * s2_y);

    if s >= 0.0 && s <= 1.0 && t >= 0.0 && t <= 1.0 {
        return Some((p0_x + (t * s1_x), p0_y + (t * s1_y), s, t));
    }

    None
}