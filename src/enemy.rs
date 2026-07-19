use crate::common::*;
use crate::player::Player;
use crate::world::World;
use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::TAU;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ZKind {
    Walker,
    Runner,
    Crawler,
    Brute,
    Spitter,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Pattern {
    Direct,
    WanderChase,
    Ranged,
}

pub struct ZDef {
    pub hp: f32,
    pub speed: f32,
    pub r: f32,
    pub dmg: f32,
    pub score: u32,
    pub pattern: Pattern,
    pub knock_resist: f32,
    pub gore: f32,
    pub shamble: f32,
    pub lurch: f32,
}

pub fn zdef(k: ZKind) -> ZDef {
    match k {
        ZKind::Walker => ZDef { hp: 46.0, speed: 52.0, r: 12.0, dmg: 9.0, score: 10, pattern: Pattern::Direct, knock_resist: 0.0, gore: 1.0, shamble: 0.5, lurch: 0.38 },
        ZKind::Runner => ZDef { hp: 30.0, speed: 96.0, r: 11.0, dmg: 7.0, score: 16, pattern: Pattern::Direct, knock_resist: 0.1, gore: 1.0, shamble: 0.2, lurch: 0.15 },
        ZKind::Crawler => ZDef { hp: 22.0, speed: 74.0, r: 9.0, dmg: 6.0, score: 14, pattern: Pattern::WanderChase, knock_resist: 0.1, gore: 0.7, shamble: 0.42, lurch: 0.32 },
        ZKind::Brute => ZDef { hp: 180.0, speed: 40.0, r: 21.0, dmg: 22.0, score: 40, pattern: Pattern::Direct, knock_resist: 0.75, gore: 2.0, shamble: 0.26, lurch: 0.42 },
        ZKind::Spitter => ZDef { hp: 40.0, speed: 52.0, r: 12.0, dmg: 5.0, score: 24, pattern: Pattern::Ranged, knock_resist: 0.2, gore: 1.0, shamble: 0.2, lurch: 0.18 },
    }
}

/// Per-zombie cosmetic + body-configuration variety.
#[derive(Clone, Copy)]
pub struct Look {
    pub skin: Color,
    pub shirt: Color,
    pub pants: Color,
    pub hair: i8, // -1 bald, 0 short, 1 long
    pub hair_col: Color,
    pub size: f32,        // overall build multiplier (proportions like the player)
    pub missing_arm: i8,  // -1 none, 0 left, 1 right
    pub missing_leg: i8,  // -1 none, 0 left, 1 right
    pub drag_leg: i8,     // -1 none, 0 left, 1 right — a limp leg dragged behind
    pub crawler: bool,    // drags itself along the ground
    pub gash: bool,       // a bloody wound/exposed rib on the torso
    pub tatters: bool,    // torn clothing dragging behind
}

fn jitter(rng: &mut impl Rng, c: [f32; 3], amt: f32) -> Color {
    Color::srgb(
        (c[0] + rng.gen_range(-amt..amt)).clamp(0.0, 1.0),
        (c[1] + rng.gen_range(-amt..amt)).clamp(0.0, 1.0),
        (c[2] + rng.gen_range(-amt..amt)).clamp(0.0, 1.0),
    )
}

pub fn make_look(kind: ZKind, rng: &mut impl Rng) -> Look {
    let skin_base = *[[0.42, 0.50, 0.36], [0.55, 0.58, 0.44], [0.36, 0.45, 0.40]]
        .get(rng.gen_range(0..3))
        .unwrap();
    let shirts = [
        [0.55, 0.16, 0.16],
        [0.16, 0.28, 0.5],
        [0.5, 0.45, 0.2],
        [0.3, 0.3, 0.34],
        [0.2, 0.4, 0.28],
        [0.45, 0.25, 0.4],
    ];
    let shirt = shirts[rng.gen_range(0..shirts.len())];
    let hair_styles = [-1i8, 0, 0, 1];
    let hcol = [[0.1, 0.08, 0.06], [0.35, 0.22, 0.1], [0.6, 0.55, 0.5]];
    let hair = hair_styles[rng.gen_range(0..hair_styles.len())];
    let hair_pick = hcol[rng.gen_range(0..hcol.len())];

    // Body configuration. Crawlers always drag; brutes are big; otherwise size
    // and disfigurement vary per corpse.
    let crawler = kind == ZKind::Crawler || (kind != ZKind::Brute && rng.gen_bool(0.06));
    let size = match kind {
        ZKind::Brute => rng.gen_range(1.0..1.15),
        ZKind::Runner => rng.gen_range(0.82..1.0),
        _ => rng.gen_range(0.82..1.18),
    };
    // Missing limbs (a crawler often has a mangled/absent leg).
    let missing_leg = if crawler && rng.gen_bool(0.6) {
        rng.gen_range(0..2)
    } else if rng.gen_bool(0.08) {
        rng.gen_range(0..2)
    } else {
        -1
    };
    let missing_arm = if rng.gen_bool(0.12) { rng.gen_range(0..2) } else { -1 };
    // An intact-legged walker may instead drag one limp, bloodied leg behind it.
    let drag_leg = if !crawler && missing_leg < 0 && rng.gen_bool(0.16) {
        rng.gen_range(0..2)
    } else {
        -1
    };

    Look {
        skin: jitter(rng, skin_base, 0.05),
        shirt: jitter(rng, shirt, 0.05),
        pants: jitter(rng, [0.18, 0.18, 0.2], 0.04),
        hair,
        hair_col: jitter(rng, hair_pick, 0.03),
        size,
        missing_arm,
        missing_leg,
        drag_leg,
        crawler,
        gash: rng.gen_bool(0.3),
        tatters: rng.gen_bool(0.35),
    }
}

#[derive(Component)]
pub struct Zombie {
    pub kind: ZKind,
    pub hp: f32,
    pub max_hp: f32,
    pub r: f32,
    pub speed: f32,
    pub dmg: f32,
    pub score: u32,
    pub pattern: Pattern,
    pub knock_resist: f32,
    pub gore: f32,

    pub vel: Vec2,
    pub angle: f32,
    pub frame: f32,
    pub state_chase: bool,
    pub wander_angle: f32,
    pub wander_timer: f32,
    pub attack_cd: f32,
    pub spit_cd: f32,
    pub hurt_flash: f32,
    pub knock: Vec2,
    pub flank: f32,

    pub shamble_amp: f32,
    pub shamble_freq: f32,
    pub shamble_phase: f32,
    pub curve_bias: f32,
    pub lurch_depth: f32,
    pub lurch_rate: f32,
    pub lurch_phase: f32,
    pub gait_t: f32,
    pub stride_rate: f32,
    pub arm_amp: f32,    // per-zombie arm-swing amplitude
    pub arm_freq: f32,   // per-zombie arm-swing frequency
    pub arm_phase: f32,  // phase offset so arms don't move in lockstep
    pub turn_rate: f32,  // per-zombie turn responsiveness (turning radius)
    pub trail_t: f32,    // countdown to the next blood mark
    pub foot: i8,        // which foot leaves the next print
    pub reach_style: f32, // 0 = swing arms, 1 = reach out toward the player
    pub reach_l: f32,     // per-arm reach amount (left), 0 swing .. 1 reach
    pub reach_r: f32,     // per-arm reach amount (right)

    pub look: Look,
    pub dead: bool,
    pub headshot: bool,     // killed by a headshot → brain burst + ragdoll corpse
    pub sever_pending: i8,  // a limb to shoot off this frame: -1 none, 0..3 limb
    pub severed_mask: u8,   // bitmask of limbs already blown off
    pub death_t: f32,       // >0 once the ragdoll death has begun (counts up in seconds)
    pub death_spin: f32,    // which way the body topples as it falls
    pub stagger: f32,       // brief flinch/stumble timer from a solid non-lethal hit
}

impl Zombie {
    pub fn new(kind: ZKind, hp_scale: f32, rng: &mut impl Rng) -> Self {
        let d = zdef(kind);
        let look = make_look(kind, rng);
        // Radius follows the build; movement follows the body configuration so a
        // dragging crawler or one-legged limper moves slower than an intact one.
        let r = d.r * look.size;
        let mut speed = d.speed * rng.gen_range(0.9..1.12);
        if look.crawler {
            speed *= 0.5;
        }
        if look.missing_leg >= 0 {
            speed *= 0.72;
        }
        if look.drag_leg >= 0 {
            speed *= 0.7;
        }
        if look.missing_arm >= 0 {
            speed *= 0.95;
        }
        // Per-arm reach variety: a base tendency, then each arm offset so some
        // zombies swing one arm while grasping with the other.
        let base_reach: f32 = if rng.gen_bool(0.4) { rng.gen_range(0.55..1.0) } else { 0.0 };
        let reach_l = (base_reach + rng.gen_range(-0.35..0.35)).clamp(0.0, 1.0);
        let reach_r = (base_reach + rng.gen_range(-0.35..0.35)).clamp(0.0, 1.0);
        Self {
            kind,
            hp: d.hp * hp_scale,
            max_hp: d.hp * hp_scale,
            r,
            speed,
            dmg: d.dmg,
            score: d.score,
            pattern: d.pattern,
            knock_resist: d.knock_resist,
            gore: d.gore,
            vel: Vec2::ZERO,
            angle: rng.gen_range(0.0..TAU),
            frame: rng.gen_range(0.0..TAU),
            state_chase: false,
            wander_angle: rng.gen_range(0.0..TAU),
            wander_timer: rng.gen_range(0.5..2.0),
            attack_cd: 0.0,
            spit_cd: rng.gen_range(1.0..3.0),
            hurt_flash: 0.0,
            knock: Vec2::ZERO,
            flank: if rng.gen_bool(0.5) { 1.0 } else { -1.0 },
            shamble_amp: d.shamble * rng.gen_range(0.7..1.3),
            shamble_freq: rng.gen_range(1.3..3.1),
            shamble_phase: rng.gen_range(0.0..TAU),
            curve_bias: rng.gen_range(-0.13..0.13) * (d.shamble / 0.4),
            lurch_depth: d.lurch * rng.gen_range(0.7..1.2),
            lurch_rate: rng.gen_range(0.7..1.7),
            lurch_phase: rng.gen_range(0.0..TAU),
            gait_t: rng.gen_range(0.0..6.0),
            stride_rate: rng.gen_range(0.8..1.35),
            arm_amp: rng.gen_range(0.55..1.5),
            arm_freq: rng.gen_range(0.9..1.9),
            arm_phase: rng.gen_range(0.0..TAU),
            turn_rate: rng.gen_range(3.0..7.5),
            trail_t: rng.gen_range(0.0..0.5),
            foot: 1,
            // Most shamble with swinging arms; some hold their arms out reaching.
            reach_style: base_reach,
            reach_l,
            reach_r,
            look,
            dead: false,
            headshot: false,
            sever_pending: -1,
            severed_mask: 0,
            death_t: 0.0,
            death_spin: 0.0,
            stagger: 0.0,
        }
    }

    pub fn apply_knockback(&mut self, angle: f32, force: f32) {
        let f = force * (1.0 - self.knock_resist);
        self.knock.x += angle.cos() * f;
        self.knock.y += angle.sin() * f;
    }
}

/// Wave director.
#[derive(Resource)]
pub struct WaveState {
    pub wave: u32,
    pub to_spawn: u32,
    pub spawn_timer: f32,
    pub intermission: f32,
    pub active: bool,
}
impl Default for WaveState {
    fn default() -> Self {
        Self {
            wave: 0,
            to_spawn: 0,
            spawn_timer: 0.0,
            intermission: 2.0,
            active: false,
        }
    }
}

/// Event asking combat to spawn a hostile spit projectile.
#[derive(Event)]
pub struct SpitEvent {
    pub pos: Vec2,
    pub angle: f32,
}

fn floor_ring_point(world: &World, center: Vec2, rng: &mut impl Rng) -> Option<Vec2> {
    for _ in 0..24 {
        let a = rng.gen_range(0.0..TAU);
        let d = rng.gen_range(560.0..820.0);
        let p = center + Vec2::new(a.cos(), a.sin()) * d;
        if !world.blocks_point(p) {
            return Some(p);
        }
    }
    None
}

pub fn wave_system(
    time: Res<Time>,
    world: Res<World>,
    mut waves: ResMut<WaveState>,
    mut score: ResMut<Score>,
    player_q: Query<&Transform, With<Player>>,
    zombies: Query<(), With<Zombie>>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    let Ok(ptf) = player_q.single() else {
        return;
    };
    let center = ptf.translation.truncate();
    let alive = zombies.iter().count() as u32;
    let mut rng = rand::thread_rng();

    if !waves.active {
        waves.intermission -= dt;
        if waves.intermission <= 0.0 {
            waves.wave += 1;
            score.wave = waves.wave;
            waves.to_spawn = 6 + waves.wave * 3;
            waves.active = true;
            waves.spawn_timer = 0.0;
        }
        return;
    }

    if waves.to_spawn > 0 {
        waves.spawn_timer -= dt;
        if waves.spawn_timer <= 0.0 && alive < 42 {
            waves.spawn_timer = (0.7 - waves.wave as f32 * 0.03).max(0.18);
            if let Some(p) = floor_ring_point(&world, center, &mut rng) {
                let kind = pick_kind(waves.wave, &mut rng);
                let hp_scale = 1.0 + waves.wave as f32 * 0.06;
                let z = Zombie::new(kind, hp_scale, &mut rng);
                let r = z.r;
                commands.spawn((
                    z,
                    Transform::from_xyz(p.x, p.y, depth_z(Z_CHAR, p.y)),
                    Visibility::default(),
                    crate::art::NeedsRig,
                    NewZombieRadius(r),
                ));
                waves.to_spawn -= 1;
            }
        }
    } else if alive == 0 {
        // Wave cleared.
        waves.active = false;
        waves.intermission = 4.0;
    }
}

/// Temporary marker carrying radius so the rig builder knows the size.
#[derive(Component)]
pub struct NewZombieRadius(pub f32);

fn pick_kind(wave: u32, rng: &mut impl Rng) -> ZKind {
    let roll = rng.gen_range(0.0..1.0);
    match wave {
        1 => {
            if roll < 0.85 {
                ZKind::Walker
            } else {
                ZKind::Crawler
            }
        }
        2..=3 => {
            if roll < 0.55 {
                ZKind::Walker
            } else if roll < 0.78 {
                ZKind::Crawler
            } else if roll < 0.94 {
                ZKind::Runner
            } else {
                ZKind::Spitter
            }
        }
        _ => {
            if roll < 0.4 {
                ZKind::Walker
            } else if roll < 0.58 {
                ZKind::Runner
            } else if roll < 0.72 {
                ZKind::Crawler
            } else if roll < 0.86 {
                ZKind::Spitter
            } else {
                ZKind::Brute
            }
        }
    }
}

pub fn zombie_ai(
    time: Res<Time>,
    world: Res<World>,
    mut spit_ev: EventWriter<SpitEvent>,
    mut set: ParamSet<(
        Query<(&mut Player, &Transform)>,
        Query<(&mut Zombie, &mut Transform)>,
    )>,
) {
    let dt = time.delta_secs();
    let (ppos, mut player_hurt, mut player_push) = {
        let pq = set.p0();
        let Ok((_p, ptf)) = pq.single() else {
            return;
        };
        (ptf.translation.truncate(), 0.0f32, Vec2::ZERO)
    };

    let mut q = set.p1();
    for (mut z, mut tf) in q.iter_mut() {
        // A dying zombie no longer chases or attacks — the death system ragdolls
        // it and slides it along any residual knockback.
        if z.death_t > 0.0 {
            continue;
        }
        z.hurt_flash = (z.hurt_flash - dt).max(0.0);
        z.attack_cd = (z.attack_cd - dt).max(0.0);
        z.stagger = (z.stagger - dt).max(0.0);
        z.gait_t += dt;
        z.frame += dt * (2.0 + z.speed * 0.05) * z.stride_rate;

        let pos = tf.translation.truncate();
        let to = ppos - pos;
        let d = to.length().max(0.001);
        let heading = to / d;
        let hp_frac = (z.hp / z.max_hp).clamp(0.0, 1.0);
        let wound_mul = 0.55 + 0.45 * hp_frac;
        // A recent solid hit staggers them: they stumble and slow for a moment.
        let spd = z.speed * wound_mul * if z.stagger > 0.0 { 0.25 } else { 1.0 };

        let mut tvel = Vec2::ZERO;
        match z.pattern {
            Pattern::WanderChase => {
                if d < 340.0 {
                    z.state_chase = true;
                }
                if !z.state_chase {
                    z.wander_timer -= dt;
                    if z.wander_timer <= 0.0 {
                        z.wander_angle = rand::thread_rng().gen_range(0.0..TAU);
                        z.wander_timer = rand::thread_rng().gen_range(0.6..2.2);
                    }
                    tvel = Vec2::new(z.wander_angle.cos(), z.wander_angle.sin()) * spd * 0.4;
                } else {
                    tvel = shamble(&z, heading, spd);
                }
            }
            Pattern::Ranged => {
                let a = to.y.atan2(to.x);
                let tr = z.turn_rate;
                z.angle = angle_lerp(z.angle, a, (dt * tr).clamp(0.0, 1.0));
                let ideal = 190.0;
                if d > ideal + 40.0 {
                    tvel = heading * spd;
                } else if d < ideal - 40.0 {
                    tvel = -heading * spd * 0.7;
                } else {
                    let perp = Vec2::new(-heading.y, heading.x);
                    tvel = perp * spd * 0.5 * z.flank;
                }
                z.spit_cd -= dt;
                if z.spit_cd <= 0.0 && d < 340.0 {
                    z.spit_cd = rand::thread_rng().gen_range(2.2..3.6);
                    spit_ev.write(SpitEvent { pos, angle: a });
                }
            }
            Pattern::Direct => {
                tvel = shamble(&z, heading, spd);
            }
        }

        if z.pattern != Pattern::Ranged {
            let moving = tvel.length_squared() > 1.0;
            if moving {
                let ma = tvel.y.atan2(tvel.x);
                let tr = z.turn_rate;
                z.angle = angle_lerp(z.angle, ma, (dt * tr).clamp(0.0, 1.0));
            }
        }

        // Knockback decay.
        tvel += z.knock;
        let decay = 0.001f32.powf(dt);
        z.knock *= decay;
        z.vel = tvel;

        let next = pos + tvel * dt;
        let mut resolved = world.collide(next, z.r);

        // Don't occupy the player's space: keep at least (z.r + player r) away,
        // sliding the zombie to the edge of the player's body if it pushed in.
        let sep = z.r + 15.0;
        let to_p = ppos - resolved;
        let dp = to_p.length();
        if dp < sep && dp > 0.001 {
            let pushed = ppos - to_p / dp * sep;
            resolved = world.collide(pushed, z.r);
        }

        tf.translation.x = resolved.x;
        tf.translation.y = resolved.y;
        tf.translation.z = depth_z(Z_CHAR, resolved.y);

        // Melee the player.
        let pd = (ppos - resolved).length();
        if pd < z.r + 15.0 + 2.0 && z.attack_cd <= 0.0 && z.stagger <= 0.0 {
            player_hurt += z.dmg * (0.55 + 0.45 * hp_frac);
            z.attack_cd = 0.7;
            let a = (ppos - resolved).y.atan2((ppos - resolved).x);
            player_push += Vec2::new(a.cos(), a.sin()) * 60.0;
        }
    }

    // Apply accumulated damage to the player.
    if player_hurt > 0.0 || player_push != Vec2::ZERO {
        let mut pq = set.p0();
        if let Ok((mut p, _)) = pq.single_mut() {
            if player_hurt > 0.0 {
                p.hurt(player_hurt);
            }
            p.vel += player_push;
        }
    }
}

/// Keep zombies from stacking on the same spot: push overlapping pairs apart a
/// little each frame, then re-resolve against the world.
pub fn zombie_separation(
    world: Res<World>,
    mut q: Query<(Entity, &mut Transform, &Zombie)>,
) {
    // Snapshot positions/radii. Dying zombies are ragdolling on the ground and
    // don't take part in crowd separation.
    let items: Vec<(Entity, Vec2, f32)> = q
        .iter()
        .filter(|(_, _, z)| z.death_t <= 0.0)
        .map(|(e, tf, z)| (e, tf.translation.truncate(), z.r))
        .collect();
    if items.len() < 2 {
        return;
    }
    // Accumulate a push per zombie from every overlapping neighbour.
    let mut pushes: Vec<(Entity, Vec2)> = Vec::with_capacity(items.len());
    for (i, &(e, pi, ri)) in items.iter().enumerate() {
        let mut push = Vec2::ZERO;
        for (j, &(_, pj, rj)) in items.iter().enumerate() {
            if i == j {
                continue;
            }
            let d = pi - pj;
            let dist = d.length();
            let min = (ri + rj) * 0.9;
            if dist < min {
                let n = if dist > 0.001 { d / dist } else { Vec2::new(0.3, 0.1) };
                // Split the overlap; heavier work happens over several frames.
                push += n * (min - dist) * 0.5;
            }
        }
        pushes.push((e, push));
    }
    for (e, push) in pushes {
        if push == Vec2::ZERO {
            continue;
        }
        if let Ok((_, mut tf, z)) = q.get_mut(e) {
            let cur = tf.translation.truncate();
            let moved = world.collide(cur + push.clamp_length_max(6.0), z.r);
            tf.translation.x = moved.x;
            tf.translation.y = moved.y;
            tf.translation.z = depth_z(Z_CHAR, moved.y);
        }
    }
}

/// Stamp a pixelated blood mark: a little cluster of small square blocks in
/// varied dark-red shades, so trails and footprints read as chunky pixel-art.
fn pixel_blood(
    commands: &mut Commands,
    center: Vec2,
    spread: Vec2,
    blocks: u32,
    life: f32,
    rng: &mut impl Rng,
) {
    for _ in 0..blocks {
        let off = Vec2::new(
            rng.gen_range(-spread.x..spread.x),
            rng.gen_range(-spread.y..spread.y),
        );
        let shade = rng.gen_range(0.22..0.40);
        let sz: f32 = rng.gen_range(2.0..4.0);
        let sz = sz.round();
        commands.spawn((
            Sprite {
                color: Color::srgba(shade, 0.02, 0.03, rng.gen_range(0.5..0.75)),
                custom_size: Some(Vec2::splat(sz)),
                ..default()
            },
            Transform::from_xyz(center.x + off.x, center.y + off.y, Z_DECAL + rng.gen_range(0.1..0.4)),
            crate::combat::Decal { life },
        ));
    }
}

/// Bleeding zombies leave pixelated marks on the ground: crawlers and leg-
/// draggers smear a continuous trail, wounded walkers stamp bloody footprints.
pub fn zombie_gore_trail(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(&mut Zombie, &Transform)>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (mut z, tf) in q.iter_mut() {
        let moving = z.vel.length_squared() > 25.0;
        z.trail_t -= dt;
        if !moving || z.trail_t > 0.0 {
            continue;
        }
        let pos = tf.translation.truncate();
        let angle = z.angle;
        let fwd = Vec2::new(angle.cos(), angle.sin());
        let perp = Vec2::new(-angle.sin(), angle.cos());
        let wounded = z.hp < z.max_hp * 0.85;
        if z.look.crawler {
            // A dragging smear right under the belly (pixelated blocks).
            z.trail_t = rng.gen_range(0.05..0.12);
            let back = pos - fwd * z.r * 0.5;
            pixel_blood(&mut commands, back, Vec2::new(z.r * 0.6, z.r * 0.4), 5, 10.0, &mut rng);
        } else if z.look.drag_leg >= 0 {
            // The dragged leg smears a near-continuous blood streak on its side.
            z.trail_t = rng.gen_range(0.08..0.16);
            let side = if z.look.drag_leg == 0 { 1.0 } else { -1.0 };
            let at = pos + perp * z.r * 0.4 * side - fwd * z.r * 0.3;
            pixel_blood(&mut commands, at, Vec2::new(z.r * 0.35, z.r * 0.28), 4, 9.0, &mut rng);
        } else if wounded {
            // Bloody footprint, offset to the current foot's side.
            z.trail_t = rng.gen_range(0.28..0.45);
            z.foot = -z.foot;
            let at = pos + perp * z.r * 0.35 * z.foot as f32;
            pixel_blood(&mut commands, at, Vec2::new(z.r * 0.22, z.r * 0.16), 3, 8.0, &mut rng);
        } else {
            z.trail_t = rng.gen_range(0.4..0.8);
        }
    }
}

fn shamble(z: &Zombie, heading: Vec2, spd: f32) -> Vec2 {
    let lurch = 1.0 - z.lurch_depth * (0.5 + 0.5 * (z.gait_t * z.lurch_rate + z.lurch_phase).sin());
    let sway = (z.gait_t * z.shamble_freq + z.shamble_phase).sin() * z.shamble_amp + z.curve_bias;
    let perp = Vec2::new(-heading.y, heading.x);
    (heading + perp * sway) * (spd * lurch)
}
