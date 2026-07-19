use crate::common::*;
use crate::input::InputState;
use crate::weapons::{weapon, Ammo, WeaponKind, WEAPONS};
use crate::world::World;
use bevy::prelude::*;

/// Head slot: a soft cap (no protection), a hard helmet (protection), or bare.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HeadGear {
    Bare,
    Cap,
    Helmet,
}

/// Body slot: a plain t-shirt (starting clothes), or a plate carrier / armour.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BodyGear {
    Shirt,
    Armor,
}

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
    pub fuel: i32,
    pub grenades: i32,
    pub throw_cd: f32, // brief cooldown between grenade throws

    pub cooldown: f32,
    pub reloading: f32,
    pub reload_total: f32,
    pub exhausted: bool,

    // gear / protection
    pub head_gear: HeadGear,
    pub body_gear: BodyGear,
    pub has_backpack: bool,
    pub helmet_dura: f32,
    pub helmet_max: f32,
    pub armor_dura: f32,
    pub armor_max: f32,
    pub armor_flash: f32,  // brief flash when armour soaks a hit
    pub hurt_amount: f32,  // health damage taken this frame (drives the red vignette)
    pub dmg_mul: f32,      // incoming-damage multiplier (Super Armor cheat drops it)

    // animation / feedback
    pub walk_frame: f32,
    pub idle_t: f32,
    pub moving: bool,
    pub running: bool,
    pub hurt_flash: f32,
    pub muzzle: f32,
    pub invuln: f32,
    pub recoil: f32,
    pub stun: f32, // knocked-out/concussed timer — no control while > 0
    pub swing_t: f32,
    pub swing_dur: f32,
    pub melee_stab: bool, // alternates each melee attack: false = slash, true = stab
    pub kills: u32,
    pub blood_feet: f32,  // remaining "wetness" of blood on the soles (leaves prints)
    pub foot_side: i8,    // which foot leaves the next bloody print
    pub step_acc: f32,    // distance accumulated toward the next footprint
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
            r: 15.0, // body collider — kept wide so arms/gun don't bury into walls
            current: 2, // start with the pistol
            clip,
            rounds: 96,
            shells: 24,
            rockets: 3,
            fuel: 200,
            grenades: 4,
            throw_cd: 0.0,
            cooldown: 0.0,
            reloading: 0.0,
            reload_total: 0.0,
            exhausted: false,
            head_gear: HeadGear::Bare,
            body_gear: BodyGear::Shirt,
            has_backpack: false,
            helmet_dura: 0.0,
            helmet_max: 0.0,
            armor_dura: 0.0,
            armor_max: 0.0,
            armor_flash: 0.0,
            hurt_amount: 0.0,
            dmg_mul: 1.0,
            walk_frame: 0.0,
            idle_t: 0.0,
            moving: false,
            running: false,
            hurt_flash: 0.0,
            muzzle: 0.0,
            invuln: 0.0,
            recoil: 0.0,
            stun: 0.0,
            swing_t: 0.0,
            swing_dur: 0.22,
            melee_stab: false,
            kills: 0,
            blood_feet: 0.0,
            foot_side: 1,
            step_acc: 0.0,
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
            Ammo::Fuel => self.fuel,
            Ammo::None => 999,
        }
    }
    pub fn ammo_mut(&mut self, a: Ammo) -> Option<&mut i32> {
        match a {
            Ammo::Rounds => Some(&mut self.rounds),
            Ammo::Shells => Some(&mut self.shells),
            Ammo::Rockets => Some(&mut self.rockets),
            Ammo::Fuel => Some(&mut self.fuel),
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
            // Top sprint speed fades as stamina drops: full boost when fresh,
            // tapering toward a jog as the bar empties.
            let sf = (self.stamina / self.max_stamina).clamp(0.0, 1.0);
            f *= 1.15 + 0.45 * sf;
        }
        f
    }

    pub fn hurt(&mut self, amount: f32) {
        if self.invuln > 0.0 {
            return;
        }
        // Super Armor cuts the blow before any gear soaks it.
        let mut dmg = amount * self.dmg_mul;
        // A helmet soaks a slice of every blow until it shatters.
        if self.head_gear == HeadGear::Helmet && self.helmet_dura > 0.0 && dmg > 0.0 {
            let soak = self.helmet_dura.min(dmg * 0.4);
            self.helmet_dura -= soak;
            dmg -= soak;
            self.armor_flash = 0.18;
            if self.helmet_dura <= 0.001 {
                self.helmet_dura = 0.0;
                self.head_gear = HeadGear::Bare; // helmet destroyed
            }
        }
        // Body armour soaks the bulk of what's left, then breaks away.
        if self.body_gear == BodyGear::Armor && self.armor_dura > 0.0 && dmg > 0.0 {
            let soak = self.armor_dura.min(dmg * 0.6);
            self.armor_dura -= soak;
            dmg -= soak;
            self.armor_flash = 0.18;
            if self.armor_dura <= 0.001 {
                self.armor_dura = 0.0;
                self.body_gear = BodyGear::Shirt; // armour destroyed
            }
        }
        self.health = (self.health - dmg).clamp(0.0, self.max_health);
        self.hurt_amount += dmg; // consumed by the hurt-vignette system
        self.hurt_flash = 0.18;
        self.invuln = 0.3;
    }

    /// Equip a helmet with `dura` remaining out of `max` full durability (a
    /// freshly-picked helmet passes dura == max; a damaged, re-worn one passes
    /// its remaining dura with the full max so the wear bar reads partial).
    pub fn equip_helmet(&mut self, dura: f32, max: f32) {
        self.head_gear = HeadGear::Helmet;
        self.helmet_dura = dura;
        self.helmet_max = max;
        self.armor_flash = 0.2;
    }
    pub fn equip_armor(&mut self, dura: f32, max: f32) {
        self.body_gear = BodyGear::Armor;
        self.armor_dura = dura;
        self.armor_max = max;
        self.armor_flash = 0.2;
    }
    pub fn equip_cap(&mut self) {
        self.head_gear = HeadGear::Cap;
        self.helmet_dura = 0.0;
        self.helmet_max = 0.0;
    }
    pub fn heal_by(&mut self, amount: f32) {
        self.health = (self.health + amount).clamp(0.0, self.max_health);
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
        self.reload_total = w.reload;
        true
    }

    /// Fraction of the current reload completed (0..1); 0 when not reloading.
    pub fn reload_progress(&self) -> f32 {
        if self.reloading <= 0.0 || self.reload_total <= 0.0 {
            0.0
        } else {
            (1.0 - self.reloading / self.reload_total).clamp(0.0, 1.0)
        }
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
    driving: Res<crate::vehicle::Driving>,
    mut q: Query<(&mut Player, &mut Transform)>,
) {
    // While driving, the vehicle system owns the player's position and heading.
    if driving.active() {
        return;
    }
    let dt = time.delta_secs();
    let Ok((mut p, mut tf)) = q.single_mut() else {
        return;
    };

    p.idle_t += dt;
    p.hurt_flash = (p.hurt_flash - dt).max(0.0);
    p.muzzle = (p.muzzle - dt).max(0.0);
    p.invuln = (p.invuln - dt).max(0.0);
    p.recoil = (p.recoil - dt * 7.0).max(0.0);
    p.throw_cd = (p.throw_cd - dt).max(0.0);
    p.swing_t = (p.swing_t - dt).max(0.0);
    p.armor_flash = (p.armor_flash - dt).max(0.0);

    // Concussed / knocked out by a nearby blast: no control, but the shockwave's
    // knockback still carries the body (velocity decays), and cooldowns tick.
    if p.stun > 0.0 {
        p.stun = (p.stun - dt).max(0.0);
        p.moving = false;
        p.running = false;
        let vel = p.vel;
        p.vel = vel * (1.0 - (dt * 4.0).clamp(0.0, 1.0));
        let cur = tf.translation.truncate();
        let next = cur + p.vel * dt;
        let resolved = world.collide(next, p.r);
        tf.translation.x = resolved.x;
        tf.translation.y = resolved.y;
        tf.translation.z = depth_z(Z_CHAR, resolved.y);
        p.cooldown = (p.cooldown - dt).max(0.0);
        if p.reloading > 0.0 {
            p.reloading -= dt;
            if p.reloading <= 0.0 {
                p.reloading = 0.0;
                p.finish_reload();
            }
        }
        // Slow health regen still trickles.
        if p.health > 0.0 && p.health < p.max_health {
            p.health = (p.health + 0.6 * dt).min(p.max_health);
        }
        return;
    }

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

    // Aim toward cursor / aim point, with a slight rotational lag so the turn
    // eases into the new heading instead of snapping.
    if input.have_aim {
        let d = input.aim_world - resolved;
        if d.length_squared() > 1.0 {
            let target = d.y.atan2(d.x);
            p.angle = angle_lerp(p.angle, target, (dt * 9.0).clamp(0.0, 1.0));
        }
    } else if want_move {
        p.angle = angle_lerp(p.angle, input.move_dir.y.atan2(input.move_dir.x), (dt * 7.0).clamp(0.0, 1.0));
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

    // Auto-reload: when a firearm runs dry and there's reserve ammo, start
    // cycling a fresh mag automatically (reload time varies per weapon).
    let w = *p.weapon();
    if w.kind != WeaponKind::Melee
        && p.reloading <= 0.0
        && p.clip[p.current] <= 0
        && p.ammo_for(w.ammo) > 0
    {
        p.start_reload();
    }
}

/// Apply the Game-Settings cheats each frame while playing.
pub fn apply_cheats(settings: Res<Settings>, mut q: Query<&mut Player>) {
    let Ok(mut p) = q.single_mut() else {
        return;
    };
    // All Weapons: keep every gun topped up so any weapon is ready the instant
    // you switch to it (no reload needed on swap).
    if settings.all_weapons {
        for (i, w) in WEAPONS.iter().enumerate() {
            if w.clip > 0 && p.clip[i] < w.clip {
                p.clip[i] = w.clip;
            }
        }
    }
    // Unlimited Ammo: reserves never deplete and the current clip stays full, so
    // firing never triggers a reload.
    if settings.unlimited_ammo {
        p.rounds = 999;
        p.shells = 999;
        p.rockets = 999;
        for (i, w) in WEAPONS.iter().enumerate() {
            if w.clip > 0 {
                p.clip[i] = w.clip;
            }
        }
        if p.reloading > 0.0 {
            p.reloading = 0.0;
            p.reload_total = 0.0;
        }
    }
    // Super Stamina: the bar stays pinned full and you're never winded.
    if settings.super_stamina {
        p.stamina = p.max_stamina;
        p.exhausted = false;
    }
    // Super Armor: soak all but a tenth of every blow (1000% better protection).
    p.dmg_mul = if settings.super_armor { 0.1 } else { 1.0 };
}

/// On touch, aim assist: face the nearest zombie (with the same turn lag) so
/// the attack button just needs to be tapped. Runs after player_update.
pub fn touch_autoaim(
    time: Res<Time>,
    input: Res<InputState>,
    settings: Res<Settings>,
    mut player_q: Query<(&mut Player, &Transform), Without<crate::enemy::Zombie>>,
    zombies: Query<&Transform, With<crate::enemy::Zombie>>,
) {
    // No assist below a whisker of slider → fully manual (face move direction).
    if !input.touch_mode || settings.aim_assist < 0.02 {
        return;
    }
    let dt = time.delta_secs();
    let Ok((mut p, ptf)) = player_q.single_mut() else {
        return;
    };
    let pos = ptf.translation.truncate();
    let mut best: Option<(f32, Vec2)> = None;
    for ztf in zombies.iter() {
        let zp = ztf.translation.truncate();
        let d2 = (zp - pos).length_squared();
        if best.map_or(true, |(bd, _)| d2 < bd) {
            best = Some((d2, zp));
        }
    }
    if let Some((d2, zp)) = best {
        if d2 < 620.0 * 620.0 {
            let dv = zp - pos;
            let target = dv.y.atan2(dv.x);
            // Higher accuracy → snappier lock-on.
            let rate = 2.0 + 16.0 * settings.aim_assist;
            p.angle = angle_lerp(p.angle, target, (dt * rate).clamp(0.0, 1.0));
        }
    }
}
