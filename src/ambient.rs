use crate::art::Art;
use crate::common::*;
use crate::hud::Cleanup;
use crate::player::Player;
use crate::world::World;
use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::TAU;

/// A fly that buzzes around a home point and scatters when the player nears.
#[derive(Component)]
pub struct Fly {
    pub home: Vec2,
    pub vel: Vec2,
    pub phase: f32,
    pub flee: f32, // scatter timer
}

/// A corpse that twitches — periodic little jerks of the body.
#[derive(Component)]
pub struct Twitch {
    pub base: f32, // resting facing angle
    pub t: f32,
    pub next: f32,
    pub jerk: f32,
}

/// A flickering light pool on the ground (alpha wavers like a dying bulb).
#[derive(Component)]
pub struct Flicker {
    pub base: f32,
    pub phase: f32,
    pub rate: f32,
}

fn soft(art: &Art, color: Color, w: f32, h: f32, z: f32, x: f32, y: f32) -> impl Bundle {
    (
        Sprite {
            image: art.soft.clone(),
            color,
            custom_size: Some(Vec2::new(w, h)),
            ..default()
        },
        Transform::from_xyz(x, y, z),
        Cleanup,
    )
}

fn block(color: Color, w: f32, h: f32, x: f32, y: f32, z: f32, rot: f32) -> impl Bundle {
    (
        Sprite::from_color(color, Vec2::new(w, h)),
        Transform {
            translation: Vec3::new(x, y, z),
            rotation: Quat::from_rotation_z(rot),
            ..default()
        },
        Cleanup,
    )
}

fn floor_pt(world: &World, avoid: Vec2, rng: &mut impl Rng, min_d: f32) -> Option<Vec2> {
    for _ in 0..30 {
        let a = rng.gen_range(0.0..TAU);
        let d = rng.gen_range(min_d..900.0);
        let p = avoid + Vec2::new(a.cos(), a.sin()) * d;
        if !world.blocks_point(p) {
            return Some(p);
        }
    }
    None
}

/// Scatter atmosphere across the arena: debris/garbage, blood pools, corpses
/// (some twitching, some with their guts spread out), flickering lights, and
/// flies that hover over the mess. Called once from start_game.
pub fn scatter_ambient(commands: &mut Commands, art: &Art, world: &World, center: Vec2) {
    let mut rng = rand::thread_rng();

    // ---- Debris & garbage (static clutter) ----
    for _ in 0..70 {
        let Some(p) = floor_pt(world, center, &mut rng, 40.0) else { continue };
        match rng.gen_range(0..5) {
            0 => {
                // crushed can
                let c = Color::srgb(rng.gen_range(0.4..0.6), rng.gen_range(0.4..0.55), 0.5);
                commands.spawn(block(c, rng.gen_range(4.0..7.0), rng.gen_range(3.0..5.0), p.x, p.y, Z_DECAL + 0.4, rng.gen_range(0.0..TAU)));
            }
            1 => {
                // scrap paper / trash
                let g = rng.gen_range(0.5..0.72);
                commands.spawn(block(Color::srgb(g, g, g * 0.95), rng.gen_range(5.0..9.0), rng.gen_range(4.0..7.0), p.x, p.y, Z_DECAL + 0.4, rng.gen_range(0.0..TAU)));
            }
            2 => {
                // rubble chunk
                let d = rng.gen_range(0.14..0.24);
                commands.spawn(block(Color::srgb(d, d, d + 0.02), rng.gen_range(5.0..10.0), rng.gen_range(5.0..9.0), p.x, p.y, Z_DECAL + 0.4, rng.gen_range(0.0..TAU)));
            }
            3 => {
                // scattered gravel cluster
                for _ in 0..4 {
                    let o = Vec2::new(rng.gen_range(-8.0..8.0), rng.gen_range(-8.0..8.0));
                    let d = rng.gen_range(0.16..0.26);
                    commands.spawn(block(Color::srgb(d, d, d), rng.gen_range(2.0..4.0), rng.gen_range(2.0..4.0), p.x + o.x, p.y + o.y, Z_DECAL + 0.35, rng.gen_range(0.0..TAU)));
                }
            }
            _ => {
                // dark oil/grime stain
                commands.spawn(soft(art, Color::srgba(0.02, 0.02, 0.03, 0.5), rng.gen_range(16.0..30.0), rng.gen_range(12.0..22.0), Z_DECAL + 0.2, p.x, p.y));
            }
        }
    }

    // ---- Blood pools ----
    for _ in 0..14 {
        let Some(p) = floor_pt(world, center, &mut rng, 60.0) else { continue };
        spawn_blood_pool(commands, art, p, rng.gen_range(0.8..1.6), &mut rng);
    }

    // ---- Corpses (some twitching, some with guts) ----
    for _ in 0..10 {
        let Some(p) = floor_pt(world, center, &mut rng, 70.0) else { continue };
        let gutsy = rng.gen_bool(0.4);
        let twitchy = !gutsy && rng.gen_bool(0.5);
        spawn_corpse(commands, art, p, gutsy, twitchy, &mut rng);
        // A couple of flies hover over each corpse.
        for _ in 0..rng.gen_range(2..5) {
            spawn_fly(commands, p, &mut rng);
        }
    }

    // ---- Flickering street lights ----
    for _ in 0..7 {
        let Some(p) = floor_pt(world, center, &mut rng, 120.0) else { continue };
        let base = rng.gen_range(0.10..0.20);
        commands.spawn((
            Sprite {
                image: art.soft.clone(),
                color: Color::srgba(1.0, 0.86, 0.55, base),
                custom_size: Some(Vec2::splat(rng.gen_range(150.0..240.0))),
                ..default()
            },
            Transform::from_xyz(p.x, p.y, Z_DECAL + 3.0),
            Flicker { base, phase: rng.gen_range(0.0..TAU), rate: rng.gen_range(6.0..16.0) },
            Cleanup,
        ));
    }
}

fn spawn_blood_pool(commands: &mut Commands, art: &Art, at: Vec2, scale: f32, rng: &mut impl Rng) {
    commands.spawn(soft(
        art,
        Color::srgba(0.22, 0.01, 0.02, 0.62),
        30.0 * scale,
        22.0 * scale,
        Z_DECAL + 0.5,
        at.x,
        at.y,
    ));
    // Pixelated clots around the rim.
    for _ in 0..(6.0 * scale) as i32 {
        let a = rng.gen_range(0.0..TAU);
        let d = rng.gen_range(6.0..16.0) * scale;
        let o = Vec2::new(a.cos(), a.sin()) * d;
        let sh = rng.gen_range(0.22..0.36);
        commands.spawn(block(
            Color::srgb(sh, 0.02, 0.03),
            rng.gen_range(2.0..4.0),
            rng.gen_range(2.0..4.0),
            at.x + o.x,
            at.y + o.y,
            Z_DECAL + 0.55,
            rng.gen_range(0.0..TAU),
        ));
    }
}

fn spawn_corpse(commands: &mut Commands, art: &Art, at: Vec2, gutsy: bool, twitchy: bool, rng: &mut impl Rng) {
    let angle = rng.gen_range(0.0..TAU);
    // Blood pool under the body.
    spawn_blood_pool(commands, art, at, rng.gen_range(1.0..1.7), rng);

    let root = commands
        .spawn((
            Transform {
                translation: Vec3::new(at.x, at.y, Z_DECAL + 1.5),
                rotation: Quat::from_rotation_z(angle),
                ..default()
            },
            Visibility::default(),
            Cleanup,
        ))
        .id();

    // A drab, dead body laid out flat (dark shirt, pale skin, splayed limbs).
    let shirt = Color::srgb(rng.gen_range(0.16..0.34), rng.gen_range(0.14..0.30), rng.gen_range(0.16..0.32));
    let skin = Color::srgb(rng.gen_range(0.42..0.55), rng.gen_range(0.42..0.52), rng.gen_range(0.40..0.48));
    let pants = Color::srgb(0.14, 0.14, 0.16);
    let mut parts = Vec::new();
    let torso = commands.spawn(block(shirt, 15.0, 13.0, 0.0, 0.0, 0.02, 0.0)).id();
    let head = commands
        .spawn((
            Sprite { image: art.circle.clone(), color: skin, custom_size: Some(Vec2::splat(11.0)), ..default() },
            Transform::from_xyz(11.0, 1.0, 0.03),
            Cleanup,
        ))
        .id();
    let arm1 = commands.spawn(block(skin, 12.0, 4.0, -1.0, 9.0, 0.02, rng.gen_range(0.4..1.0))).id();
    let arm2 = commands.spawn(block(skin, 12.0, 4.0, -1.0, -9.0, 0.02, -rng.gen_range(0.4..1.0))).id();
    let leg1 = commands.spawn(block(pants, 13.0, 5.0, -12.0, 4.0, 0.01, rng.gen_range(-0.3..0.3))).id();
    let leg2 = commands.spawn(block(pants, 13.0, 5.0, -12.0, -4.0, 0.01, rng.gen_range(-0.3..0.3))).id();
    parts.extend([torso, head, arm1, arm2, leg1, leg2]);

    if gutsy {
        // Guts spread out of a torn belly — a mess of red/pink entrail blobs.
        for _ in 0..10 {
            let o = Vec2::new(rng.gen_range(2.0..20.0), rng.gen_range(-9.0..9.0));
            let pink = rng.gen_range(0.35..0.6);
            let g = commands
                .spawn((
                    Sprite { image: art.circle.clone(), color: Color::srgb(pink, 0.10, 0.12), custom_size: Some(Vec2::splat(rng.gen_range(3.0..7.0))), ..default() },
                    Transform::from_xyz(o.x, o.y, 0.04),
                    Cleanup,
                ))
                .id();
            parts.push(g);
        }
        // A darker cavity.
        let cav = commands.spawn(block(Color::srgb(0.20, 0.02, 0.03), 8.0, 7.0, 4.0, 0.0, 0.05, 0.0)).id();
        parts.push(cav);
    }

    commands.entity(root).add_children(&parts);
    if twitchy {
        commands.entity(root).insert(Twitch {
            base: angle,
            t: 0.0,
            next: rng.gen_range(0.6..2.2),
            jerk: 0.0,
        });
    }
}

fn spawn_fly(commands: &mut Commands, home: Vec2, rng: &mut impl Rng) {
    let p = home + Vec2::new(rng.gen_range(-14.0..14.0), rng.gen_range(-14.0..14.0));
    commands.spawn((
        Sprite::from_color(Color::srgb(0.05, 0.05, 0.06), Vec2::splat(2.2)),
        Transform::from_xyz(p.x, p.y, Z_PARTICLE + 2.0),
        Fly { home, vel: Vec2::ZERO, phase: rng.gen_range(0.0..TAU), flee: 0.0 },
        Cleanup,
    ));
}

/// Flies buzz erratically around their home and scatter from the player.
pub fn fly_system(
    time: Res<Time>,
    player_q: Query<&Transform, (With<Player>, Without<Fly>)>,
    mut q: Query<(&mut Fly, &mut Transform)>,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    let ppos = player_q.single().ok().map(|tf| tf.translation.truncate());
    let mut rng = rand::thread_rng();
    for (mut fly, mut tf) in q.iter_mut() {
        let pos = tf.translation.truncate();
        fly.flee = (fly.flee - dt).max(0.0);
        // Scatter when the player gets close.
        if let Some(pp) = ppos {
            let to = pos - pp;
            let d = to.length();
            if d < 95.0 {
                fly.flee = 0.6;
                let away = if d > 0.01 { to / d } else { Vec2::new(1.0, 0.0) };
                fly.vel += away * 900.0 * dt;
            }
        }
        // Buzz: jittery orbit around home unless fleeing.
        if fly.flee <= 0.0 {
            let target = fly.home
                + Vec2::new((t * 3.1 + fly.phase).sin(), (t * 2.7 + fly.phase * 1.7).cos()) * 12.0;
            let to = target - pos;
            fly.vel += to * 6.0 * dt;
            fly.vel += Vec2::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0)) * 140.0 * dt;
        }
        fly.vel *= 0.90_f32.powf(dt * 60.0);
        fly.vel = fly.vel.clamp_length_max(340.0);
        tf.translation.x += fly.vel.x * dt;
        tf.translation.y += fly.vel.y * dt;
    }
}

/// Corpses jerk now and then.
pub fn twitch_system(time: Res<Time>, mut q: Query<(&mut Twitch, &mut Transform)>) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (mut tw, mut tf) in q.iter_mut() {
        tw.t += dt;
        // Decay the current jerk back toward rest.
        tw.jerk *= 0.86_f32.powf(dt * 60.0);
        if tw.t >= tw.next {
            tw.t = 0.0;
            tw.next = rng.gen_range(0.7..2.6);
            tw.jerk = rng.gen_range(-0.16..0.16);
        }
        tf.rotation = Quat::from_rotation_z(tw.base + tw.jerk);
    }
}

/// Flickering lights waver in intensity like failing bulbs.
pub fn flicker_system(time: Res<Time>, mut q: Query<(&Flicker, &mut Sprite)>) {
    let t = time.elapsed_secs();
    let mut rng = rand::thread_rng();
    for (fl, mut sprite) in q.iter_mut() {
        // A couple of sine terms plus noise → an uneven, buzzing flicker.
        let s = (t * fl.rate + fl.phase).sin() * 0.5 + (t * fl.rate * 2.7 + fl.phase).sin() * 0.25;
        let noise = rng.gen_range(-0.12..0.12);
        let a = (fl.base * (1.0 + s * 0.6 + noise)).clamp(0.02, 0.35);
        sprite.color = Color::srgba(1.0, 0.86, 0.55, a);
    }
}
