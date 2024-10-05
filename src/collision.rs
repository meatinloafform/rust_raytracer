use nalgebra::Vector2;

// // https://ericleong.me/research/circle-line/#collision-response
// pub fn lines_intersect(
//     x1: f32, y1: f32, x2: f32, y2: f32,
//     x3: f32, y3: f32, x4: f32, y4: f32
// ) -> Option<Vector2<f32>> {
//     let a1 = y2 - y1;
//     let b1 = x1 - x2;
//     let c1 = a1 * x1 + b1 * y1;
//     let a2 = y4 - y3;
//     let b2 = x3 - x4;
//     let c2 = a2 * x3 + b2 * y3;
//     let det = a1 * b2 - a2 * b1;
//     if det != 0.0 {
//         let x = (b2 * c1 - b1 * c2) / det;
//         let y = (a1 * c2 - a2 * c1) / det;
//         if x >= x1.min(x2) && x <= x1.max(x2)
//         && x >= x3.min(x4) && x <= x3.max(x4)
//         && y >= y1.min(y2) && y <= y1.max(y2)
//         && y >= y3.min(y4) && y <= y3.max(y4) {
//             return Some(Vector2::new(x, y));
//         }
//     }

//     None
// }

// pub fn closest_point_on_line(
//     lx1: f32, ly1: f32, lx2: f32, ly2: f32,
//     x0: f32, y0: f32
// ) -> Vector2<f32> {
//     let a1 = ly2 - ly1;
//     let b1 = lx1 - lx2;
//     let c1 = (ly2 - ly1) * lx1 + (lx1 - lx2) * ly1;
//     let c2 = -b1 * x0 + a1 * y0;
//     let det = a1 * a1 - -b1 * b1;
//     let cx;
//     let cy;

//     if det != 0.0 {
//         cx = (a1 * c1 - b1 * c2) / det;
//         cy = (a1 * c2 - -b1 * c1) / det;
//     } else {
//         cx = y0;
//         cy = y0;
//     }

//     return Vector2::new(cx, cy);
// }

// pub fn collide_circle_segment(
//     circle: Vector2<f32>, r: f32,
//     line_0: Vector2<f32>, line_1: Vector2<f32>,
//     circle_vel: Vector2<f32> 
// ) -> Option<Vector2<f32>> {
//     let point1 = closest_point_on_line(
//         line_0.x, line_0.y, line_1.x, line_1.y,
//         circle.x, circle.y
//     );

//     let movement_intersection = lines_intersect(
//         circle.x, circle.y, circle.x + circle_vel.x, circle.y + circle_vel.y,
//         line_0.x, line_0.y, line_1.x, line_1.y
//     );

//     if let Some(movement_intersection) = movement_intersection {
//         dbg!("movement intersection");

//         // b
//         // closest point on line from endpoint of movement vector
//         let closest_movement_line = closest_point_on_line(
//             line_0.x, line_0.y, line_1.x, line_1.y,
//             circle.x + circle_vel.x, circle.y + circle_vel.y
//         );

//         // c
//         // closest point on movement vector to line_0
//         let closest_line0_movement = closest_point_on_line(
//             circle.x, circle.y, circle.x + circle_vel.x, circle.y + circle_vel.y,
//             line_0.x, line_0.y
//         );

//         // d
//         // closest point on movement vector to line_1
//         let closest_line1_movement = closest_point_on_line(
//             circle.x, circle.y, circle.x + circle_vel.x, circle.y + circle_vel.y,
//             line_1.x, line_1.y
//         );

//         // TODO: you might need more checks
//         if closest_movement_line.metric_distance(&(circle + circle_vel)) < r
//             && closest_line0_movement.metric_distance(&line_0) < r
//             && closest_line1_movement.metric_distance(&line_1) < r {
//             let stopped_position = movement_intersection - r * 
//                 ((circle - movement_intersection).magnitude() / (circle - point1).magnitude()) *
//                 circle_vel.normalize();

//             // TODO: Check for collisions with endpoints
//             return Some(stopped_position);
//         }
//     }

//     None
// }

// http://code.alaiwan.org/blog/collision-disk.html
pub struct CollisionResult {
    pub depth: f32,
    pub normal: Vector2<f32>
}

pub fn closest_point_on_segment(pos: Vector2<f32>, s0: Vector2<f32>, s1: Vector2<f32>) -> Vector2<f32> {
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

pub fn collide_circle_with_segment(circle: Vector2<f32>, r: f32, s0: Vector2<f32>, s1: Vector2<f32>) -> Option<CollisionResult> {
    let delta = circle - closest_point_on_segment(circle, s0, s1);

    // V dot V gives |v| ^ 2
    if delta.dot(&delta) > r * r {
        return None;
    }

    let dist = delta.magnitude();
    let normal = delta * (1.0 / dist);

    Some(CollisionResult {
        depth: r - dist,
        normal
    })
}

pub fn collide_with_segments(pos: Vector2<f32>, r: f32, segments: &[(Vector2<f32>, Vector2<f32>)]) -> Option<CollisionResult> {
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

const COLLISION_RESOLUTION: usize = 5;

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