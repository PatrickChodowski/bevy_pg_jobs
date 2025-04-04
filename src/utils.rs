
use libm::{atan2f, fabsf, cosf, sinf}; 
use bevy::math::Vec2;
use rand::Rng;

pub fn get_random_range_u32(min: u32, max: u32) -> u32 {
    let mut rng = rand::rng();
    let rand: u32 = rng.random_range(min..=max);
    return rand;
}

pub fn get_direction(source_xy: &Vec2, target_xy: &Vec2) -> f32 {
    return atan2f(target_xy.y - source_xy.y, target_xy.x - source_xy.x);
}

pub fn get_distance_manhattan(source: &Vec2, target: &Vec2) -> f32 {
    return fabsf(target.x - source.x) + fabsf(target.y - source.y);
}

pub fn move_x(speed: f32, angle: f32) -> f32{
    speed * cosf(angle)
}
pub fn move_y(speed: f32, angle: f32) -> f32{
    speed * sinf(angle)
}