use bevy::prelude::*;

/// World unit size of one tile.
pub const TILE: f32 = 40.0;

/// Z-layer helpers so everything sorts predictably. Top-down: things lower on
/// screen (more negative world-y) draw in front, so we fold y into z.
pub fn depth_z(base: f32, y: f32) -> f32 {
    base - y * 0.0005
}
pub const Z_FLOOR: f32 = -100.0;
pub const Z_DECAL: f32 = -90.0;
pub const Z_SHADOW: f32 = -60.0;
pub const Z_CHAR: f32 = 0.0; // characters get depth_z(Z_CHAR, y)
pub const Z_PROP: f32 = 0.0;
pub const Z_PROJECTILE: f32 = 60.0;
pub const Z_PARTICLE: f32 = 55.0;
pub const Z_FX: f32 = 70.0;

/// Top-level game flow.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Menu,
    Playing,
    GameOver,
}

/// A running tally the HUD reads.
#[derive(Resource, Default)]
pub struct Score {
    pub kills: u32,
    pub points: u32,
    pub wave: u32,
}

/// Simple screen-shake accumulator consumed by the camera.
#[derive(Resource, Default)]
pub struct Shake {
    pub trauma: f32,
}
impl Shake {
    pub fn add(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount).min(1.0);
    }
}

/// Handy angle-lerp toward a target (shortest way round).
pub fn angle_lerp(a: f32, b: f32, t: f32) -> f32 {
    let mut d = (b - a) % std::f32::consts::TAU;
    if d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    }
    if d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    a + d * t.clamp(0.0, 1.0)
}

pub fn approach(current: f32, target: f32, rate: f32) -> f32 {
    current + (target - current) * rate.clamp(0.0, 1.0)
}
