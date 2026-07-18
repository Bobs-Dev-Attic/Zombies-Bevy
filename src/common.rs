use bevy::prelude::*;

/// Game version, sourced from Cargo.toml so there's a single source of truth.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

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
    Options,
    Playing,
    GameOver,
}

/// Cheat / assist toggles listed under Options → Game Settings.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Cheat {
    AllWeapons,
    UnlimitedAmmo,
    SuperStamina,
    SuperArmor,
}
pub const CHEATS: [Cheat; 4] = [
    Cheat::AllWeapons,
    Cheat::UnlimitedAmmo,
    Cheat::SuperStamina,
    Cheat::SuperArmor,
];
impl Cheat {
    pub fn label(self) -> &'static str {
        match self {
            Cheat::AllWeapons => "All Weapons",
            Cheat::UnlimitedAmmo => "Unlimited Ammo",
            Cheat::SuperStamina => "Super Stamina",
            Cheat::SuperArmor => "Super Armor",
        }
    }
    pub fn hint(self) -> &'static str {
        match self {
            Cheat::AllWeapons => "every gun kept loaded and ready",
            Cheat::UnlimitedAmmo => "reserves never run dry",
            Cheat::SuperStamina => "never tire, sprint forever",
            Cheat::SuperArmor => "1000% better protection",
        }
    }
}

/// Persistent player settings.
#[derive(Resource)]
pub struct Settings {
    /// 0 = fully manual aiming, 1 = snap instantly to the nearest zombie.
    pub aim_assist: f32,
    pub all_weapons: bool,
    pub unlimited_ammo: bool,
    pub super_stamina: bool,
    pub super_armor: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            aim_assist: 0.6,
            all_weapons: false,
            unlimited_ammo: false,
            super_stamina: false,
            super_armor: false,
        }
    }
}
impl Settings {
    pub fn cheat(&self, c: Cheat) -> bool {
        match c {
            Cheat::AllWeapons => self.all_weapons,
            Cheat::UnlimitedAmmo => self.unlimited_ammo,
            Cheat::SuperStamina => self.super_stamina,
            Cheat::SuperArmor => self.super_armor,
        }
    }
    pub fn toggle_cheat(&mut self, c: Cheat) {
        match c {
            Cheat::AllWeapons => self.all_weapons = !self.all_weapons,
            Cheat::UnlimitedAmmo => self.unlimited_ammo = !self.unlimited_ammo,
            Cheat::SuperStamina => self.super_stamina = !self.super_stamina,
            Cheat::SuperArmor => self.super_armor = !self.super_armor,
        }
    }
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
