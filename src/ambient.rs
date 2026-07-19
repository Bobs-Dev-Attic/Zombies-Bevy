use crate::art::Art;
use crate::common::*;
use crate::hud::Cleanup;
use crate::player::Player;
use crate::world::World;
use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::TAU;

/// A fly that hovers around a home point (a corpse). Some fly lazy circles, some
/// zig-zag erratically, all with a jittery buzz.
#[derive(Component)]
pub struct Fly {
    pub home: Vec2,
    pub vel: Vec2,
    pub target: Vec2, // current spot it's drifting toward (zig-zaggers)
    pub t: f32,       // countdown to the next zig
    pub circler: bool,
    pub spin: f32,      // orbit angle (circlers)
    pub spin_rate: f32, // orbit angular speed
    pub radius: f32,    // orbit radius
}

/// A crow feeding at a corpse: it hops and pecks, and flees when disturbed.
#[derive(Component)]
pub struct Crow {
    pub home: Vec2,
    pub head: Entity,
    pub vel: Vec2,
    pub target: Vec2,
    pub t: f32,       // hop / peck timer
    pub peck: f32,    // pecking bob phase
    pub fleeing: bool,
}

/// A stray cat that skitters around the streets and bolts when the player nears.
/// Shootable, like the crows.
#[derive(Component)]
pub struct Cat {
    pub home: Vec2,
    pub vel: Vec2,
    pub target: Vec2,
    pub t: f32,
    pub fleeing: bool,
    pub tail: Entity,
    pub fur: Color,
}

/// A corpse that twitches — periodic little jerks of the body.
#[derive(Component)]
pub struct Twitch {
    pub base: f32, // resting facing angle
    pub t: f32,
    pub next: f32,
    pub jerk: f32,
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

    // ---- Corpses (some twitching, some with guts), with flies and the odd crow ----
    for _ in 0..10 {
        let Some(p) = floor_pt(world, center, &mut rng, 70.0) else { continue };
        let gutsy = rng.gen_bool(0.4);
        let twitchy = !gutsy && rng.gen_bool(0.5);
        spawn_corpse(commands, art, p, gutsy, twitchy, &mut rng);
        // A cloud of flies hovers over each corpse.
        for _ in 0..rng.gen_range(3..6) {
            spawn_fly(commands, p, &mut rng);
        }
        // Some corpses have a crow or two feeding on them.
        if rng.gen_bool(0.4) {
            for _ in 0..rng.gen_range(1..3) {
                spawn_crow(commands, p, &mut rng);
            }
        }
    }

    // ---- Stray cats roaming the streets ----
    for _ in 0..rng.gen_range(3..6) {
        if let Some(p) = floor_pt(world, center, &mut rng, 120.0) {
            spawn_cat(commands, p, &mut rng);
        }
    }
}

fn spawn_blood_pool(commands: &mut Commands, art: &Art, at: Vec2, scale: f32, rng: &mut impl Rng) {
    commands.spawn((
        soft(
            art,
            Color::srgba(0.22, 0.01, 0.02, 0.62),
            30.0 * scale,
            22.0 * scale,
            Z_DECAL + 0.5,
            at.x,
            at.y,
        ),
        crate::combat::BloodDecal,
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

/// A jointed corpse limb in body-local space (child sprites of the corpse root):
/// upper segment → joint → bent lower segment → hand/foot. Pushes the ids so the
/// caller can parent them under the corpse root.
fn corpse_limb(
    commands: &mut Commands,
    parts: &mut Vec<Entity>,
    art: &Art,
    sx: f32, sy: f32, la: f32, u: f32, l: f32, w: f32,
    seg: Color, joint: Color, ext: Color,
) {
    let (dx, dy) = (la.cos(), la.sin());
    parts.push(commands.spawn(block(seg, u, w, sx + dx * u * 0.5, sy + dy * u * 0.5, 0.02, la)).id());
    parts.push(
        commands
            .spawn((
                Sprite { image: art.circle.clone(), color: joint, custom_size: Some(Vec2::splat(w * 1.05)), ..default() },
                Transform::from_xyz(sx + dx * u, sy + dy * u, 0.025),
            ))
            .id(),
    );
    let la2 = la + 0.25;
    let (ex, ey) = (sx + dx * u, sy + dy * u);
    let (dx2, dy2) = (la2.cos(), la2.sin());
    parts.push(commands.spawn(block(seg, l, w * 0.9, ex + dx2 * l * 0.5, ey + dy2 * l * 0.5, 0.021, la2)).id());
    parts.push(
        commands
            .spawn((
                Sprite { image: art.circle.clone(), color: ext, custom_size: Some(Vec2::splat(w * 1.15)), ..default() },
                Transform::from_xyz(ex + dx2 * l, ey + dy2 * l, 0.03),
            ))
            .id(),
    );
}

fn spawn_corpse(commands: &mut Commands, art: &Art, at: Vec2, gutsy: bool, twitchy: bool, rng: &mut impl Rng) {
    let angle = rng.gen_range(0.0..TAU);
    // Blood pool under the body.
    spawn_blood_pool(commands, art, at, rng.gen_range(1.4..2.2), rng);

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

    // A drab, dead body laid out flat — bigger than a standing footprint, with
    // jointed limbs, a face, and sometimes a torn-away half-face.
    let sc = 1.4;
    let shirt = Color::srgb(rng.gen_range(0.16..0.34), rng.gen_range(0.14..0.30), rng.gen_range(0.16..0.32));
    let sh = shirt.to_srgba();
    let shirt_hi = Color::srgb((sh.red * 1.25).min(1.0), (sh.green * 1.25).min(1.0), (sh.blue * 1.25).min(1.0));
    let skin = Color::srgb(rng.gen_range(0.42..0.55), rng.gen_range(0.42..0.52), rng.gen_range(0.40..0.48));
    let sk = skin.to_srgba();
    let skin_d = Color::srgb(sk.red * 0.82, sk.green * 0.82, sk.blue * 0.82);
    let pants = Color::srgb(0.14, 0.14, 0.16);
    let pants_d = Color::srgb(0.10, 0.10, 0.12);
    let bone = Color::srgb(0.86, 0.83, 0.74);
    let eye = Color::srgb(0.07, 0.05, 0.05);
    let mut parts = Vec::new();
    // Legs, then torso, then arms.
    corpse_limb(commands, &mut parts, art, -5.0 * sc, 5.0 * sc, 2.4, 8.5 * sc, 8.0 * sc, 5.2 * sc, pants, pants_d, pants_d);
    corpse_limb(commands, &mut parts, art, -5.0 * sc, -5.0 * sc, -2.4, 8.5 * sc, 8.0 * sc, 5.2 * sc, pants, pants_d, pants_d);
    parts.push(commands.spawn((Sprite { image: art.circle.clone(), color: shirt, custom_size: Some(Vec2::new(17.0 * sc, 15.0 * sc)), ..default() }, Transform::from_xyz(0.0, 0.0, 0.02))).id());
    parts.push(commands.spawn(block(shirt_hi, 3.5 * sc, 13.0 * sc, -1.0 * sc, 0.0, 0.021, 0.0)).id());
    corpse_limb(commands, &mut parts, art, 3.0 * sc, 7.0 * sc, 1.1, 7.5 * sc, 7.0 * sc, 4.4 * sc, skin, skin_d, skin_d);
    corpse_limb(commands, &mut parts, art, 3.0 * sc, -7.0 * sc, -1.1, 7.5 * sc, 7.0 * sc, 4.4 * sc, skin, skin_d, skin_d);
    // Head + face.
    let hx = 13.5 * sc;
    parts.push(commands.spawn((Sprite { image: art.circle.clone(), color: skin, custom_size: Some(Vec2::splat(13.0 * sc)), ..default() }, Transform::from_xyz(hx, 1.0, 0.03))).id());
    parts.push(commands.spawn(block(skin_d, 2.0 * sc, 3.0 * sc, hx + 5.0 * sc, 1.5 * sc, 0.035, 0.0)).id()); // nose
    parts.push(commands.spawn((Sprite { image: art.circle.clone(), color: eye, custom_size: Some(Vec2::splat(2.6 * sc)), ..default() }, Transform::from_xyz(hx + 3.0 * sc, 1.0 + 3.2 * sc, 0.04))).id());
    if rng.gen_bool(0.4) {
        // Torn half: exposed skull + teeth.
        parts.push(commands.spawn((Sprite { image: art.circle.clone(), color: bone, custom_size: Some(Vec2::new(7.0 * sc, 8.0 * sc)), ..default() }, Transform::from_xyz(hx + 1.0 * sc, 1.0 - 3.5 * sc, 0.032))).id());
        for k in 0..3 {
            parts.push(commands.spawn(block(Color::srgb(0.9, 0.88, 0.8), 1.4 * sc, 2.0 * sc, hx + 4.2 * sc, 1.0 - 5.0 * sc + k as f32 * 2.0 * sc, 0.045, 0.0)).id());
        }
    } else {
        parts.push(commands.spawn((Sprite { image: art.circle.clone(), color: eye, custom_size: Some(Vec2::splat(2.6 * sc)), ..default() }, Transform::from_xyz(hx + 3.0 * sc, 1.0 - 3.2 * sc, 0.04))).id());
        parts.push(commands.spawn(block(Color::srgb(0.12, 0.05, 0.06), 4.8 * sc, 2.6 * sc, hx + 5.2 * sc, 1.0, 0.04, 0.2)).id()); // agape mouth
    }

    if gutsy {
        // Guts spread out of a torn belly — a mess of red/pink entrail blobs.
        for _ in 0..12 {
            let o = Vec2::new(rng.gen_range(2.0..24.0), rng.gen_range(-11.0..11.0));
            let pink = rng.gen_range(0.35..0.6);
            let g = commands
                .spawn((
                    Sprite { image: art.circle.clone(), color: Color::srgb(pink, 0.10, 0.12), custom_size: Some(Vec2::splat(rng.gen_range(3.5..8.0))), ..default() },
                    Transform::from_xyz(o.x, o.y, 0.05),
                ))
                .id();
            parts.push(g);
        }
        // A darker cavity.
        let cav = commands.spawn(block(Color::srgb(0.20, 0.02, 0.03), 9.0, 8.0, 4.0, 0.0, 0.055, 0.0)).id();
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
    let p = home + Vec2::new(rng.gen_range(-12.0..12.0), rng.gen_range(-12.0..12.0));
    commands.spawn((
        Sprite::from_color(Color::srgb(0.05, 0.05, 0.06), Vec2::splat(2.2)),
        Transform::from_xyz(p.x, p.y, Z_PARTICLE + 2.0),
        Fly {
            home,
            vel: Vec2::ZERO,
            target: p,
            t: rng.gen_range(0.0..0.4),
            circler: rng.gen_bool(0.45),
            spin: rng.gen_range(0.0..TAU),
            spin_rate: rng.gen_range(3.5..7.0) * if rng.gen_bool(0.5) { 1.0 } else { -1.0 },
            radius: rng.gen_range(6.0..16.0),
        },
        Cleanup,
    ));
}

/// Flies congregate around their corpse and buzz like real flies: some fly lazy
/// loops, others zig-zag in sharp erratic darts — all jittery. They give the
/// player a small berth without zooming clean off.
pub fn fly_system(
    time: Res<Time>,
    player_q: Query<&Transform, (With<Player>, Without<Fly>)>,
    mut q: Query<(&mut Fly, &mut Transform)>,
) {
    let dt = time.delta_secs();
    let ppos = player_q.single().ok().map(|tf| tf.translation.truncate());
    let mut rng = rand::thread_rng();
    for (mut fly, mut tf) in q.iter_mut() {
        let pos = tf.translation.truncate();
        let home = fly.home;
        if fly.circler {
            // Trace a lazy loop around the corpse, with a little wobble.
            fly.spin += fly.spin_rate * dt;
            let spin = fly.spin;
            let radius = fly.radius;
            let target = home + Vec2::new(spin.cos(), spin.sin()) * radius;
            fly.target = target;
            fly.vel += (target - pos) * 9.0 * dt;
            fly.vel += Vec2::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0)) * 22.0 * dt;
        } else {
            // Zig-zag: hold a heading briefly, then snap to a new sharp one.
            fly.t -= dt;
            if fly.t <= 0.0 {
                fly.t = rng.gen_range(0.1..0.35);
                let a = rng.gen_range(0.0..TAU);
                let r = rng.gen_range(4.0..18.0);
                let target = home + Vec2::new(a.cos(), a.sin()) * r;
                fly.target = target;
                let to = target - pos;
                let d = to.length().max(0.01);
                fly.vel += to / d * rng.gen_range(70.0..150.0); // sharp dart
            }
            let target = fly.target;
            fly.vel += (target - pos) * 2.0 * dt;
            fly.vel += Vec2::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0)) * 40.0 * dt;
        }
        // Keep a little distance from the player, gently (drift, don't flee).
        if let Some(pp) = ppos {
            let to = pos - pp;
            let d = to.length();
            if d < 60.0 && d > 0.01 {
                fly.vel += to / d * (60.0 - d) * 2.4 * dt;
            }
        }
        fly.vel *= 0.88_f32.powf(dt * 60.0);
        fly.vel = fly.vel.clamp_length_max(200.0);
        tf.translation.x += fly.vel.x * dt;
        tf.translation.y += fly.vel.y * dt;
    }
}

/// A shot crow: a burst of feathers and a splayed dead-bird decal.
pub fn kill_crow(commands: &mut Commands, pos: Vec2, rng: &mut impl Rng) {
    for _ in 0..9 {
        let a = rng.gen_range(0.0..TAU);
        feather(commands, pos, Vec2::new(a.cos(), a.sin()) * rng.gen_range(40.0..160.0), rng);
    }
    // Dead crow lying splayed on the ground.
    commands.spawn((
        Sprite::from_color(Color::srgb(0.05, 0.05, 0.07), Vec2::new(13.0, 8.0)),
        Transform {
            translation: Vec3::new(pos.x, pos.y, Z_DECAL + 2.2),
            rotation: Quat::from_rotation_z(rng.gen_range(0.0..TAU)),
            ..default()
        },
        crate::combat::Decal { life: 22.0 },
    ));
    // Splayed wings.
    for s in [-1.0f32, 1.0] {
        commands.spawn((
            Sprite::from_color(Color::srgb(0.03, 0.03, 0.05), Vec2::new(9.0, 4.0)),
            Transform {
                translation: Vec3::new(pos.x + rng.gen_range(-3.0..3.0), pos.y + s * 5.0, Z_DECAL + 2.3),
                rotation: Quat::from_rotation_z(s * rng.gen_range(0.3..0.8)),
                ..default()
            },
            crate::combat::Decal { life: 22.0 },
        ));
    }
    // A little blood.
    commands.spawn((
        Sprite {
            color: Color::srgba(0.3, 0.02, 0.03, 0.6),
            custom_size: Some(Vec2::splat(rng.gen_range(6.0..11.0))),
            ..default()
        },
        Transform::from_xyz(pos.x, pos.y, Z_DECAL + 2.1),
        crate::combat::Decal { life: 16.0 },
    ));
}

/// A drifting black feather particle.
fn feather(commands: &mut Commands, pos: Vec2, vel: Vec2, rng: &mut impl Rng) {
    let col = Color::srgb(0.05, 0.05, 0.07);
    commands.spawn((
        Sprite::from_color(col, Vec2::new(rng.gen_range(1.8..3.0), 1.4)),
        Transform {
            translation: Vec3::new(pos.x, pos.y, Z_PARTICLE + 1.0),
            rotation: Quat::from_rotation_z(rng.gen_range(0.0..TAU)),
            ..default()
        },
        crate::combat::Particle {
            vel,
            life: rng.gen_range(0.4..0.9),
            max_life: 0.9,
            drag: 0.92,
            gravity: 0.0,
            base: col,
        },
    ));
}

/// Spawn a crow feeding at a corpse: a small dark body with a beaked head.
fn spawn_crow(commands: &mut Commands, home: Vec2, rng: &mut impl Rng) {
    let p = home + Vec2::new(rng.gen_range(-16.0..16.0), rng.gen_range(-16.0..16.0));
    let body_col = Color::srgb(0.06, 0.06, 0.08);
    let head = commands
        .spawn((
            Sprite::from_color(body_col, Vec2::new(5.0, 4.5)),
            Transform::from_xyz(6.0, 0.0, 0.02),
        ))
        .id();
    let beak = commands
        .spawn((
            Sprite::from_color(Color::srgb(0.5, 0.42, 0.15), Vec2::new(3.5, 1.8)),
            Transform::from_xyz(4.0, 0.0, 0.03),
        ))
        .id();
    commands.entity(head).add_child(beak);
    let root = commands
        .spawn((
            Transform {
                translation: Vec3::new(p.x, p.y, Z_PARTICLE + 3.0),
                rotation: Quat::from_rotation_z(rng.gen_range(0.0..TAU)),
                ..default()
            },
            Visibility::default(),
            Crow {
                home,
                head,
                vel: Vec2::ZERO,
                target: p,
                t: rng.gen_range(0.4..1.4),
                peck: rng.gen_range(0.0..TAU),
                fleeing: false,
            },
            Cleanup,
        ))
        .id();
    // Body + folded wings.
    let body = commands.spawn(Sprite::from_color(body_col, Vec2::new(11.0, 7.0))).id();
    let wing = commands
        .spawn((
            Sprite::from_color(Color::srgb(0.03, 0.03, 0.05), Vec2::new(7.0, 8.5)),
            Transform::from_xyz(-2.0, 0.0, 0.01),
        ))
        .id();
    commands.entity(root).add_children(&[body, wing, head]);
}

/// Crows hop and peck at their corpse, and flap off fast when the player nears.
pub fn crow_system(
    time: Res<Time>,
    mut commands: Commands,
    player_q: Query<&Transform, (With<Player>, Without<Crow>)>,
    mut q: Query<(Entity, &mut Crow, &mut Transform)>,
    mut tf_q: Query<&mut Transform, (Without<Crow>, Without<Player>)>,
) {
    let dt = time.delta_secs();
    let ppos = player_q.single().ok().map(|tf| tf.translation.truncate());
    let mut rng = rand::thread_rng();
    for (e, mut crow, mut tf) in q.iter_mut() {
        let pos = tf.translation.truncate();
        // Disturbed → take flight.
        if let Some(pp) = ppos {
            if !crow.fleeing && (pos - pp).length() < 135.0 {
                crow.fleeing = true;
                // A couple of feathers puff off as it startles.
                for _ in 0..4 {
                    let a = rng.gen_range(0.0..TAU);
                    feather(&mut commands, pos, Vec2::new(a.cos(), a.sin()) * rng.gen_range(30.0..90.0), &mut rng);
                }
            }
        }

        if crow.fleeing {
            // Flap away from the player and off the map, then despawn.
            let away = ppos.map(|pp| (pos - pp)).unwrap_or(Vec2::new(1.0, 0.0));
            let away = away.normalize_or_zero();
            crow.vel += away * 900.0 * dt;
            crow.vel = crow.vel.clamp_length_max(360.0);
            // Flap: the wings/head bob quickly (fake by bobbing the head).
            crow.peck += dt * 22.0;
            if let Ok(mut h) = tf_q.get_mut(crow.head) {
                h.translation.y = (crow.peck).sin() * 2.0;
            }
            if (pos - crow.home).length() > 620.0 {
                commands.entity(e).despawn();
                continue;
            }
        } else {
            // Feed: hop to a new nearby spot now and then, pecking in between.
            crow.t -= dt;
            if crow.t <= 0.0 {
                crow.t = rng.gen_range(0.6..1.8);
                let a = rng.gen_range(0.0..TAU);
                let r = rng.gen_range(4.0..14.0);
                crow.target = crow.home + Vec2::new(a.cos(), a.sin()) * r;
                let to = crow.target - pos;
                crow.vel += to.normalize_or_zero() * rng.gen_range(30.0..70.0);
            }
            let target = crow.target;
            crow.vel += (target - pos) * 3.0 * dt;
            crow.vel *= 0.82_f32.powf(dt * 60.0);
            // Peck: the head dips down and back up.
            crow.peck += dt * 6.0;
            if let Ok(mut h) = tf_q.get_mut(crow.head) {
                let dip = (crow.peck.sin() * 0.5 + 0.5).powf(2.0);
                h.translation.x = 6.0 - dip * 3.0;
                h.translation.y = -dip * 1.5;
            }
        }
        // Face travel direction and move.
        if crow.vel.length_squared() > 1.0 {
            tf.rotation = Quat::from_rotation_z(crow.vel.y.atan2(crow.vel.x));
        }
        tf.translation.x += crow.vel.x * dt;
        tf.translation.y += crow.vel.y * dt;
    }
}

fn spawn_cat(commands: &mut Commands, home: Vec2, rng: &mut impl Rng) {
    let furs = [
        Color::srgb(0.08, 0.08, 0.09),  // black
        Color::srgb(0.35, 0.35, 0.38),  // grey
        Color::srgb(0.6, 0.38, 0.16),   // ginger
        Color::srgb(0.7, 0.68, 0.62),   // white/cream
    ];
    let fur = furs[rng.gen_range(0..furs.len())];
    let dark = {
        let s = fur.to_srgba();
        Color::srgb(s.red * 0.65, s.green * 0.65, s.blue * 0.65)
    };
    let p = home + Vec2::new(rng.gen_range(-12.0..12.0), rng.gen_range(-12.0..12.0));
    // Tail as a rear pivot so it can sway.
    let tail = commands
        .spawn((Transform::from_xyz(-6.0, 0.0, 0.02), Visibility::default()))
        .id();
    let tstrip = commands
        .spawn((
            Sprite::from_color(fur, Vec2::new(7.0, 2.2)),
            Transform::from_xyz(-3.5, 0.0, 0.0),
        ))
        .id();
    commands.entity(tail).add_child(tstrip);
    let root = commands
        .spawn((
            Transform {
                translation: Vec3::new(p.x, p.y, Z_PARTICLE + 3.0),
                rotation: Quat::from_rotation_z(rng.gen_range(0.0..TAU)),
                ..default()
            },
            Visibility::default(),
            Cat {
                home,
                vel: Vec2::ZERO,
                target: p,
                t: rng.gen_range(0.5..1.8),
                fleeing: false,
                tail,
                fur,
            },
            Cleanup,
        ))
        .id();
    let body = commands.spawn(Sprite::from_color(fur, Vec2::new(12.0, 6.5))).id();
    let head = commands
        .spawn((
            Sprite::from_color(fur, Vec2::new(6.0, 6.0)),
            Transform::from_xyz(7.0, 0.0, 0.03),
        ))
        .id();
    let ear1 = commands
        .spawn((
            Sprite::from_color(dark, Vec2::new(2.4, 3.0)),
            Transform::from_xyz(8.0, 2.2, 0.02),
        ))
        .id();
    let ear2 = commands
        .spawn((
            Sprite::from_color(dark, Vec2::new(2.4, 3.0)),
            Transform::from_xyz(8.0, -2.2, 0.02),
        ))
        .id();
    commands.entity(head).add_children(&[ear1, ear2]);
    commands.entity(root).add_children(&[tail, body, head]);
}

/// Fur puff + a small dead-cat decal where a shot cat drops.
pub fn kill_cat(commands: &mut Commands, pos: Vec2, fur: Color, rng: &mut impl Rng) {
    for _ in 0..5 {
        let a = rng.gen_range(0.0..TAU);
        feather(commands, pos, Vec2::new(a.cos(), a.sin()) * rng.gen_range(30.0..90.0), rng);
    }
    // A little blood.
    for _ in 0..rng.gen_range(3..6) {
        let a = rng.gen_range(0.0..TAU);
        let d = rng.gen_range(3.0..10.0);
        let o = Vec2::new(a.cos(), a.sin()) * d;
        commands.spawn(block(
            Color::srgb(0.3, 0.02, 0.03),
            rng.gen_range(2.0..4.0),
            rng.gen_range(2.0..4.0),
            pos.x + o.x,
            pos.y + o.y,
            Z_DECAL + 0.6,
            rng.gen_range(0.0..TAU),
        ));
    }
    // Splayed carcass.
    let rot = rng.gen_range(0.0..TAU);
    commands.spawn(block(fur, 12.0, 6.0, pos.x, pos.y, Z_DECAL + 0.7, rot));
    commands.spawn(block(fur, 6.0, 5.0, pos.x + rot.cos() * 8.0, pos.y + rot.sin() * 8.0, Z_DECAL + 0.72, rot));
}

/// Cats wander and sit, and bolt away fast when the player gets close.
pub fn cat_system(
    time: Res<Time>,
    world: Res<World>,
    mut commands: Commands,
    player_q: Query<&Transform, (With<Player>, Without<Cat>)>,
    mut q: Query<(Entity, &mut Cat, &mut Transform)>,
    mut tf_q: Query<&mut Transform, (Without<Cat>, Without<Player>)>,
) {
    let dt = time.delta_secs();
    let ppos = player_q.single().ok().map(|tf| tf.translation.truncate());
    let mut rng = rand::thread_rng();
    for (e, mut cat, mut tf) in q.iter_mut() {
        let pos = tf.translation.truncate();
        if let Some(pp) = ppos {
            if !cat.fleeing && (pos - pp).length() < 125.0 {
                cat.fleeing = true;
            }
        }
        if cat.fleeing {
            // Bolt directly away, skittering fast, and vanish once well clear.
            let away = ppos.map(|pp| pos - pp).unwrap_or(Vec2::new(1.0, 0.0));
            cat.vel += away.normalize_or_zero() * 1400.0 * dt;
            cat.vel = cat.vel.clamp_length_max(300.0);
            if (pos - cat.home).length() > 640.0 {
                commands.entity(e).despawn();
                continue;
            }
        } else {
            // Amble to a nearby spot, pausing (sitting) in between.
            cat.t -= dt;
            if cat.t <= 0.0 {
                cat.t = rng.gen_range(0.8..2.6);
                let a = rng.gen_range(0.0..TAU);
                let r = rng.gen_range(10.0..40.0);
                cat.target = cat.home + Vec2::new(a.cos(), a.sin()) * r;
            }
            let target = cat.target;
            cat.vel += (target - pos) * 1.6 * dt;
            cat.vel = cat.vel.clamp_length_max(70.0);
            cat.vel *= 0.9_f32.powf(dt * 60.0);
        }
        // Tail sway (faster while fleeing).
        let sway = if cat.fleeing { 16.0 } else { 4.0 };
        if let Ok(mut t) = tf_q.get_mut(cat.tail) {
            t.rotation = Quat::from_rotation_z((time.elapsed_secs() * sway).sin() * 0.5);
        }
        // Face travel, slide against walls.
        if cat.vel.length_squared() > 1.0 {
            tf.rotation = Quat::from_rotation_z(cat.vel.y.atan2(cat.vel.x));
        }
        let next = pos + cat.vel * dt;
        let resolved = world.collide(next, 4.0);
        tf.translation.x = resolved.x;
        tf.translation.y = resolved.y;
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

