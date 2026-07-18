use crate::common::*;
use crate::input::InputState;
use crate::weapons::{weapon, Ammo, WeaponKind, WEAPONS};
use crate::world::World;
use bevy::prelude::*;

#[derive(Component)]
pub struct Player {
    pub health: f32,
    pub max_health: f32,
    pub stamina: f32,
    pub max_stamina: f32,
    pub base_speed: f32,
    pub angle: f32,
    pub vel: Vec2,
    pub r: f32,

    pub current: usize,
    pub clip: [i32; WEAPONS.len()],
    pub rounds: i32,
    pub shells: i32,
    pub rockets: i32,

    pub cooldown: f32,
    pub reloading: f32,
    pub exhausted: bool,

    // animation / feedback
    pub walk_frame: f32,
    pub idle_t: f32,
    pub moving: bool,
    pub running: bool,
    pub hurt_flash: f32,
    pub muzzle: f32,
    pub invuln: f32,
    pub recoil: f32,
    pub swing_t: f32,
    pub swing_dur: f32,
    pub kills: u32,
}

impl Default for Player {
    fn default() -> Self {
        let mut clip = [0; WEAPONS.len()];
        for (i, w) in WEAPONS.iter().enumerate() {
            clip[i] = w.clip;
        }
        Self {
            health: 100.0,
            max_health: 100.0,
            stamina: 140.0,
            max_stamina: 140.0,
            base_speed: 205.0,
            angle: 0.0,
            vel: Vec2::ZERO,
            r: 11.0,
            current: 2, // start with the pistol
            clip,
            rounds: 96,
            shells: 24,
            rockets: 3,
            cooldown: 0.0,
            reloading: 0.0,
            exhausted: false,
            walk_frame: 0.0,
            idle_t: 0.0,
            moving: false,
            running: false,
            hurt_flash: 0.0,
            muzzle: 0.0,
            invuln: 0.0,
            recoil: 0.0,
            swing_t: 0.0,
            swing_dur: 0.22,
            kills: 0,
        }
    }
}

impl Player {
    pub fn weapon(&self) -> &'static crate::weapons::Weapon {
        weapon(self.current)
    }

    pub fn ammo_for(&self, a: Ammo) -> i32 {
        match a {
            Ammo::Rounds => self.rounds,
            Ammo::Shells => self.shells,
            Ammo::Rockets => self.rockets,
            Ammo::None => 999,
        }
    }
    pub fn ammo_mut(&mut self, a: Ammo) -> Option<&mut i32> {
        match a {
            Ammo::Rounds => Some(&mut self.rounds),
            Ammo::Shells => Some(&mut self.shells),
            Ammo::Rockets => Some(&mut self.rockets),
            Ammo::None => None,
        }
    }

    pub fn speed_factor(&self, sprinting: bool) -> f32 {
        let mut f = 1.0;
        let hp = self.health / self.max_health;
        if hp < 0.25 {
            f *= 0.62;
        } else if hp < 0.5 {
            f *= 0.82;
        }
        if self.exhausted {
            f *= 0.6;
        }
        if sprinting && !self.exhausted && self.stamina > 0.0 {
            f *= 1.5;
        }
        f
    }

    pub fn hurt(&mut self, amount: f32) {
        if self.invuln > 0.0 {
            return;
        }
        self.health = (self.health - amount).clamp(0.0, self.max_health);
        self.hurt_flash = 0.18;
        self.invuln = 0.3;
    }

    pub fn start_reload(&mut self) -> bool {
        let w = self.weapon();
        if w.kind == WeaponKind::Melee || self.reloading > 0.0 {
            return false;
        }
        let clip = self.clip[self.current];
        if clip >= w.clip {
            return false;
        }
        if self.ammo_for(w.ammo) <= 0 {
            return false;
        }
        self.reloading = w.reload;
        true
    }

    fn finish_reload(&mut self) {
        let w = *self.weapon();
        let clip = self.clip[self.current];
        let need = w.clip - clip;
        let have = self.ammo_for(w.ammo);
        let take = need.min(have);
        self.clip[self.current] = clip + take;
        if let Some(a) = self.ammo_mut(w.ammo) {
            *a -= take;
        }
    }

    pub fn can_fire(&self) -> bool {
        if self.cooldown > 0.0 || self.reloading > 0.0 {
            return false;
        }
        let w = self.weapon();
        if w.kind == WeaponKind::Melee {
            return true;
        }
        self.clip[self.current] > 0
    }
}

pub fn player_update(
    time: Res<Time>,
    input: Res<InputState>,
    world: Res<World>,
    mut q: Query<(&mut Player, &mut Transform)>,
) {
    let dt = time.delta_secs();
    let Ok((mut p, mut tf)) = q.single_mut() else {
        return;
    };

    p.idle_t += dt;
    p.hurt_flash = (p.hurt_flash - dt).max(0.0);
    p.muzzle = (p.muzzle - dt).max(0.0);
    p.invuln = (p.invuln - dt).max(0.0);
    p.recoil = (p.recoil - dt * 7.0).max(0.0);
    p.swing_t = (p.swing_t - dt).max(0.0);

    // Weapon switching.
    // (handled here so state is centralized)
    if let Some(slot) = input.weapon_slot {
        if slot < WEAPONS.len() {
            p.current = slot;
        }
    }
    if input.next_weapon {
        p.current = (p.current + 1) % WEAPONS.len();
    }
    if input.prev_weapon {
        p.current = (p.current + WEAPONS.len() - 1) % WEAPONS.len();
    }
    if input.reload {
        p.start_reload();
    }

    let want_move = input.move_mag > 0.08;
    let sprinting = input.move_mag > 0.92 && want_move;

    // Stamina.
    let mut drain_mul = 1.0 + (1.0 - p.health / p.max_health) * 0.5;
    let _ = &mut drain_mul;
    if sprinting && !p.exhausted && p.stamina > 0.0 {
        p.stamina = (p.stamina - 24.0 * dt * drain_mul).clamp(0.0, p.max_stamina);
        if p.stamina <= 0.0 {
            p.exhausted = true;
        }
    } else if !want_move {
        p.stamina = (p.stamina + 26.0 * dt).clamp(0.0, p.max_stamina);
    } else {
        p.stamina = (p.stamina + 12.0 * dt).clamp(0.0, p.max_stamina);
    }
    if p.exhausted && p.stamina > p.max_stamina * 0.25 {
        p.exhausted = false;
    }

    p.moving = want_move;
    p.running = sprinting && !p.exhausted && p.stamina > 0.0;

    let spd = p.base_speed * p.speed_factor(sprinting) * (input.move_mag / 0.9).clamp(0.0, 1.0);
    let target = if want_move {
        input.move_dir * spd
    } else {
        Vec2::ZERO
    };
    let vel = p.vel;
    p.vel = vel + (target - vel) * (dt * 12.0).clamp(0.0, 1.0);

    let cur = tf.translation.truncate();
    let next = cur + p.vel * dt;
    let resolved = world.collide(next, p.r);
    tf.translation.x = resolved.x;
    tf.translation.y = resolved.y;
    tf.translation.z = depth_z(Z_CHAR, resolved.y);

    if want_move {
        p.walk_frame += dt * if sprinting { 16.0 } else { 10.0 };
    }

    // Aim toward cursor / aim point.
    if input.have_aim {
        let d = input.aim_world - resolved;
        if d.length_squared() > 1.0 {
            let target = d.y.atan2(d.x);
            p.angle = angle_lerp(p.angle, target, (dt * 16.0).clamp(0.0, 1.0));
        }
    } else if want_move {
        p.angle = angle_lerp(p.angle, input.move_dir.y.atan2(input.move_dir.x), (dt * 10.0).clamp(0.0, 1.0));
    }

    // Passive regen.
    if p.health > 0.0 && p.health < p.max_health {
        p.health = (p.health + 1.2 * dt).min(p.max_health);
    }

    // Cooldowns.
    p.cooldown = (p.cooldown - dt).max(0.0);
    if p.reloading > 0.0 {
        p.reloading -= dt;
        if p.reloading <= 0.0 {
            p.reloading = 0.0;
            p.finish_reload();
        }
    }
}
