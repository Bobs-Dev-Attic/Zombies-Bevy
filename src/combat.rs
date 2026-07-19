use crate::common::*;
use crate::enemy::{SpitEvent, Zombie};
use crate::input::InputState;
use crate::player::Player;
use crate::weapons::WeaponKind;
use crate::world::World;
use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::TAU;

#[derive(Component)]
pub struct Projectile {
    pub vel: Vec2,
    pub damage: f32,
    pub range: f32,
    pub traveled: f32,
    pub knockback: f32,
    pub sever: f32,
    pub explosive: f32,
    pub pierce: i32,
    pub ricochet: u32,
    pub wall_pierce: u32,
    pub falloff: f32,
    pub hostile: bool,
    pub hit: Vec<Entity>,
}

#[derive(Component)]
pub struct Particle {
    pub vel: Vec2,
    pub life: f32,
    pub max_life: f32,
    pub drag: f32,
    pub gravity: f32,
    pub base: Color,
}

#[derive(Component)]
pub struct Decal {
    pub life: f32,
}

/// Marks a blood pool the player can track through, leaving bloody footprints.
#[derive(Component)]
pub struct BloodDecal;

/// If the player is standing in blood, track fading bloody footprints behind them
/// for a short while.
pub fn player_footprints(
    time: Res<Time>,
    mut commands: Commands,
    mut player_q: Query<(&mut Player, &Transform)>,
    blood_q: Query<&Transform, (With<BloodDecal>, Without<Player>)>,
) {
    let dt = time.delta_secs();
    let Ok((mut p, tf)) = player_q.single_mut() else {
        return;
    };
    let pos = tf.translation.truncate();
    // Standing on (or right next to) blood re-wets the soles.
    for btf in blood_q.iter() {
        if (btf.translation.truncate() - pos).length() < 20.0 {
            p.blood_feet = 3.5;
            break;
        }
    }
    p.blood_feet = (p.blood_feet - dt).max(0.0);
    if p.blood_feet <= 0.0 || !p.moving {
        return;
    }
    // Stamp a print every stride, alternating feet, fainter as the blood wears off.
    p.step_acc += p.vel.length() * dt;
    if p.step_acc >= 15.0 {
        p.step_acc = 0.0;
        p.foot_side = -p.foot_side;
        let ang = p.angle;
        let side = Vec2::new(-ang.sin(), ang.cos());
        let at = pos + side * 5.0 * p.foot_side as f32;
        let a = (p.blood_feet / 3.5).clamp(0.0, 1.0) * 0.6;
        commands.spawn((
            Sprite {
                color: Color::srgba(0.3, 0.02, 0.03, a),
                custom_size: Some(Vec2::new(6.5, 3.6)),
                ..default()
            },
            Transform {
                translation: Vec3::new(at.x, at.y, Z_DECAL + 0.7),
                rotation: Quat::from_rotation_z(ang),
                ..default()
            },
            Decal { life: 6.0 },
        ));
    }
}

/// An ejected shell/casing that tumbles across the floor, bounces off walls, and
/// settles before fading out.
#[derive(Component)]
pub struct Casing {
    pub vel: Vec2,
    pub life: f32,
    pub spin: f32,
}

/// Move ejected casings: friction settles them, walls bounce them (losing energy),
/// and they fade over the last part of their life.
pub fn casing_system(
    time: Res<Time>,
    world: Res<World>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Casing, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (e, mut c, mut tf, mut sprite) in q.iter_mut() {
        c.life -= dt;
        if c.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        // Floor friction brings it to rest.
        let fr = (1.0 - dt * 3.5).clamp(0.0, 1.0);
        c.vel *= fr;
        let prev = tf.translation.truncate();
        let next = prev + c.vel * dt;
        if world.blocks_point(next) {
            // Reflect off whichever wall face it crossed, losing half its speed.
            let hit_x = world.blocks_point(Vec2::new(next.x, prev.y));
            let hit_y = world.blocks_point(Vec2::new(prev.x, next.y));
            if hit_x && !hit_y {
                c.vel.x = -c.vel.x * 0.5;
            } else if hit_y && !hit_x {
                c.vel.y = -c.vel.y * 0.5;
            } else {
                c.vel = -c.vel * 0.5;
            }
            c.spin = -c.spin * 0.6;
            // Stay at the previous spot this frame (backed out of the wall).
        } else {
            tf.translation.x = next.x;
            tf.translation.y = next.y;
        }
        tf.rotation *= Quat::from_rotation_z(c.spin * dt);
        // Fade out over the final stretch.
        let a = (c.life / 0.5).clamp(0.0, 1.0);
        let col = sprite.color.to_srgba();
        sprite.color = Color::srgba(col.red, col.green, col.blue, a);
    }
}

/// A single crisp square of the pixelated muzzle flash. Snaps through a couple
/// of frames then vanishes (no fade to a gradient — it "flashes").
#[derive(Component)]
pub struct MuzzleFlash {
    pub life: f32,
    pub max: f32,
}

/// Spawn a blocky star of squares bursting forward from the barrel tip. `scale`
/// sizes the burst (bigger for the rifle and explosives).
pub fn spawn_muzzle_flash(
    commands: &mut Commands,
    muzzle: Vec2,
    angle: f32,
    scale: f32,
    rng: &mut impl Rng,
) {
    let fwd = Vec2::new(angle.cos(), angle.sin());
    let perp = Vec2::new(-angle.sin(), angle.cos());
    let big = scale >= 1.4;
    let core = Color::srgb(1.0, 0.98, 0.85);
    let mid = Color::srgb(1.0, 0.82, 0.35);
    let outer = Color::srgb(1.0, 0.55, 0.18);
    let life = 0.07;

    // Star pattern: (forward offset, side offset, square size, colour).
    let blocks: [(f32, f32, f32, Color); 9] = [
        (1.0, 0.0, 8.0, core),
        (7.0, 0.0, 6.0, core),
        (12.0, 0.0, 5.0, mid),
        (17.0, 0.0, 3.5, outer),
        (5.0, 5.0, 4.5, mid),
        (5.0, -5.0, 4.5, mid),
        (10.0, 4.0, 3.0, outer),
        (10.0, -4.0, 3.0, outer),
        (3.0, 0.0, 11.0, mid),
    ];
    for (f, s, sz, col) in blocks {
        let p = muzzle + fwd * (f * scale) + perp * (s * scale);
        let jit = life * rng.gen_range(0.8..1.15);
        commands.spawn((
            Sprite::from_color(col, Vec2::splat(sz * scale)),
            Transform {
                translation: Vec3::new(p.x, p.y, Z_FX - 1.0),
                rotation: Quat::from_rotation_z(angle),
                ..default()
            },
            MuzzleFlash { life: jit, max: jit },
        ));
    }
    // A few forward sparks flung from the barrel.
    for _ in 0..(if big { 8 } else { 5 }) {
        let a = angle + rng.gen_range(-0.35..0.35);
        let sp = rng.gen_range(220.0..460.0) * scale;
        let tip = muzzle + fwd * 6.0;
        commands.spawn((
            Sprite::from_color(
                if rng.gen_bool(0.5) { mid } else { outer },
                Vec2::splat(rng.gen_range(2.0..3.5)),
            ),
            Transform::from_xyz(tip.x, tip.y, Z_FX - 1.0),
            Particle {
                vel: Vec2::new(a.cos(), a.sin()) * sp,
                life: rng.gen_range(0.08..0.16),
                max_life: 0.16,
                drag: 0.9,
                gravity: 0.0,
                base: outer,
            },
        ));
    }
}

pub fn muzzle_flash_system(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut MuzzleFlash, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (e, mut m, mut sprite) in q.iter_mut() {
        m.life -= dt;
        if m.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        // Two-step brightness so it reads as a hard flash, not a fade.
        let t = m.life / m.max;
        let a = if t > 0.5 { 1.0 } else { 0.7 };
        let c = sprite.color.to_srgba();
        sprite.color = Color::srgba(c.red, c.green, c.blue, a);
    }
}

#[derive(Resource, Default)]
pub struct FireLatch(pub bool);

#[derive(Event)]
pub struct Explosion {
    pub pos: Vec2,
    pub radius: f32,
    pub damage: f32,
    pub knockback: f32,
    pub sever: f32,
}

/// A thrown grenade: flies out, slows to a roll, and detonates when the fuse
/// burns down (or immediately if it rolls onto the player's foe pile — no, just
/// the fuse). It blinks faster as the fuse runs low.
#[derive(Component)]
pub struct Grenade {
    pub vel: Vec2,
    pub fuse: f32,
    pub spin: f32,
}

/// Throw grenades (G / F) and drive the ones in flight; on a spent fuse they
/// emit an Explosion, reusing the same blast path as the bazooka.
pub fn grenade_system(
    time: Res<Time>,
    input: Res<InputState>,
    world: Res<World>,
    mut commands: Commands,
    mut explosions: EventWriter<Explosion>,
    mut player_q: Query<(&mut Player, &Transform)>,
    mut grenades: Query<(Entity, &mut Grenade, &mut Transform), Without<Player>>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();

    // Throw.
    if let Ok((mut p, ptf)) = player_q.single_mut() {
        if input.throw && p.stun <= 0.0 && p.throw_cd <= 0.0 && p.grenades > 0 {
            p.grenades -= 1;
            p.throw_cd = 0.5;
            let pos = ptf.translation.truncate();
            let angle = p.angle;
            let fwd = Vec2::new(angle.cos(), angle.sin());
            let start = pos + fwd * 18.0;
            let body = Color::srgb(0.16, 0.24, 0.14); // dark olive pineapple
            let grenade = commands
                .spawn((
                    Sprite::from_color(body, Vec2::splat(6.5)),
                    Transform::from_xyz(start.x, start.y, Z_PROJECTILE),
                    Grenade {
                        vel: fwd * 360.0,
                        fuse: 1.15,
                        spin: rng.gen_range(-8.0..8.0),
                    },
                ))
                .id();
            // A little top nub so it reads as a grenade, not a pebble.
            let nub = commands
                .spawn((
                    Sprite::from_color(Color::srgb(0.3, 0.3, 0.32), Vec2::new(2.5, 2.0)),
                    Transform::from_xyz(0.0, 3.5, 0.05),
                ))
                .id();
            commands.entity(grenade).add_child(nub);
        }
    }

    // Update in-flight grenades.
    for (e, mut g, mut tf) in grenades.iter_mut() {
        g.fuse -= dt;
        // Friction: it slows to a roll after the throw.
        let vel = g.vel * (1.0 - (dt * 2.4).clamp(0.0, 1.0));
        g.vel = vel;
        let cur = tf.translation.truncate();
        let resolved = world.collide(cur + vel * dt, 3.0);
        tf.translation.x = resolved.x;
        tf.translation.y = resolved.y;
        tf.rotation *= Quat::from_rotation_z(g.spin * dt);
        if g.fuse <= 0.0 {
            explosions.write(Explosion {
                pos: resolved,
                radius: 82.0,
                damage: 95.0,
                knockback: 260.0,
                sever: 0.5,
            });
            commands.entity(e).despawn();
        }
    }
}


fn spawn_particle(commands: &mut Commands, pos: Vec2, vel: Vec2, color: Color, size: f32, life: f32, gravity: f32) {
    commands.spawn((
        Sprite::from_color(color, Vec2::splat(size)),
        Transform::from_xyz(pos.x, pos.y, Z_PARTICLE),
        Particle {
            vel,
            life,
            max_life: life,
            drag: 0.9,
            gravity,
            base: color,
        },
    ));
}

fn blood_burst(commands: &mut Commands, pos: Vec2, dir: f32, amount: u32) {
    let mut rng = rand::thread_rng();
    for _ in 0..amount {
        let a = dir + rng.gen_range(-0.7..0.7);
        let sp = rng.gen_range(40.0..220.0);
        let v = Vec2::new(a.cos(), a.sin()) * sp;
        let shade = rng.gen_range(0.45..0.72);
        spawn_particle(
            commands,
            pos,
            v,
            Color::srgb(shade, 0.05, 0.06),
            rng.gen_range(2.0..4.5),
            rng.gen_range(0.3..0.7),
            0.0,
        );
    }
    // A lingering ground stain, built from many small blobs so it reads as a
    // splatter of pooling blood rather than one flat square.
    let fwd = Vec2::new(dir.cos(), dir.sin());
    // 1) A central irregular pool: a few overlapping dark blobs.
    let pool = 2 + amount / 4;
    for _ in 0..pool {
        let o = Vec2::new(rng.gen_range(-6.0..6.0), rng.gen_range(-6.0..6.0));
        let d: f32 = rng.gen_range(0.16..0.34);
        commands.spawn((
            Sprite {
                color: Color::srgba(0.24 + d * 0.4, 0.015, 0.02, rng.gen_range(0.6..0.82)),
                custom_size: Some(Vec2::new(rng.gen_range(7.0..15.0), rng.gen_range(6.0..13.0))),
                ..default()
            },
            Transform::from_xyz(pos.x + o.x, pos.y + o.y, Z_DECAL + rng.gen_range(0.0..1.0))
                .with_rotation(Quat::from_rotation_z(rng.gen_range(0.0..std::f32::consts::TAU))),
            Decal { life: rng.gen_range(16.0..30.0) },
            BloodDecal,
        ));
    }
    // 2) A directional cast-off spray: small droplets thrown along `dir`, getting
    //    finer and more spread the farther out they land.
    let drops = 6 + amount;
    for _ in 0..drops {
        let d = rng.gen_range(4.0..46.0);
        let spread = 0.15 + d * 0.012;
        let a = dir + rng.gen_range(-spread..spread);
        let off = Vec2::new(a.cos(), a.sin()) * d
            + fwd * rng.gen_range(0.0..8.0);
        let sz: f32 = rng.gen_range(1.5..5.0) * (1.0 - d / 70.0).max(0.35);
        commands.spawn((
            Sprite {
                color: Color::srgba(0.34, 0.02, 0.03, rng.gen_range(0.45..0.75)),
                custom_size: Some(Vec2::splat(sz.max(1.0))),
                ..default()
            },
            Transform::from_xyz(pos.x + off.x, pos.y + off.y, Z_DECAL + rng.gen_range(0.1..1.0)),
            Decal { life: rng.gen_range(10.0..20.0) },
        ));
    }
}

/// A headshot: brains, skull chips and a red mist blow out along `dir` (the far
/// side of the head), plus a lingering pink/grey splatter on the ground.
fn brain_burst(commands: &mut Commands, pos: Vec2, dir: f32) {
    let mut rng = rand::thread_rng();
    // Chunks of brain (pinkish-grey) flung out the exit side.
    for _ in 0..14 {
        let a = dir + rng.gen_range(-0.6..0.6);
        let sp = rng.gen_range(120.0..420.0);
        let g = rng.gen_range(0.55..0.78);
        spawn_particle(
            commands,
            pos,
            Vec2::new(a.cos(), a.sin()) * sp,
            Color::srgb(g, g * 0.62, g * 0.66), // pinkish grey
            rng.gen_range(2.5..5.5),
            rng.gen_range(0.3..0.65),
            0.0,
        );
    }
    // Skull chips (bone) + a red mist.
    for _ in 0..8 {
        let a = dir + rng.gen_range(-0.8..0.8);
        let sp = rng.gen_range(160.0..460.0);
        spawn_particle(
            commands,
            pos,
            Vec2::new(a.cos(), a.sin()) * sp,
            Color::srgb(0.86, 0.83, 0.74),
            rng.gen_range(1.6..3.2),
            rng.gen_range(0.2..0.5),
            0.0,
        );
    }
    blood_burst(commands, pos, dir, 8);
    // A pink/grey brain splatter fanning out on the exit side.
    let fwd = Vec2::new(dir.cos(), dir.sin());
    for _ in 0..10 {
        let d = rng.gen_range(6.0..34.0);
        let off = fwd * d + Vec2::new(rng.gen_range(-10.0..10.0), rng.gen_range(-10.0..10.0));
        let g = rng.gen_range(0.4..0.6);
        commands.spawn((
            Sprite {
                color: Color::srgba(g, g * 0.45, g * 0.5, rng.gen_range(0.5..0.8)),
                custom_size: Some(Vec2::splat({let v:f32=rng.gen_range(2.0..5.0);v.round()})),
                ..default()
            },
            Transform::from_xyz(pos.x + off.x, pos.y + off.y, Z_DECAL + rng.gen_range(0.2..0.6)),
            Decal { life: 14.0 },
        ));
    }
}

/// A little shower of sparks where a round bounces off / punches through a wall.
fn spark_burst(commands: &mut Commands, pos: Vec2, dir: f32) {
    let mut rng = rand::thread_rng();
    for _ in 0..5 {
        let a = dir + std::f32::consts::PI + rng.gen_range(-0.9..0.9);
        let sp = rng.gen_range(120.0..300.0);
        spawn_particle(
            commands,
            pos,
            Vec2::new(a.cos(), a.sin()) * sp,
            Color::srgb(1.0, 0.85, 0.5),
            rng.gen_range(1.6..2.8),
            rng.gen_range(0.1..0.25),
            0.0,
        );
    }
}

pub fn firing_system(
    time: Res<Time>,
    input: Res<InputState>,
    mut latch: ResMut<FireLatch>,
    mut shake: ResMut<Shake>,
    mut noise: ResMut<crate::enemy::Noise>,
    mut commands: Commands,
    mut q: Query<(&mut Player, &Transform)>,
    mut zombies: Query<(&mut Zombie, &Transform)>,
    mut props: Query<(&mut crate::world::PropObj, &Transform), (Without<Player>, Without<Zombie>)>,
) {
    let _ = time;
    let Ok((mut p, tf)) = q.single_mut() else {
        return;
    };
    // No firing while knocked out by a blast.
    if p.stun > 0.0 {
        return;
    }
    let pos = tf.translation.truncate();
    let w = *p.weapon();

    let want = input.fire;
    let fresh = want && !latch.0;
    latch.0 = want;

    let trigger = if w.auto { want } else { fresh };
    if !trigger || !p.can_fire() {
        return;
    }

    p.cooldown = 1.0 / w.rate;

    // Noise draws zombies in: loud guns carry far, melee is silent.
    let loud = match w.kind {
        WeaponKind::Melee => 0.0,
        WeaponKind::Flamethrower => 220.0,
        WeaponKind::Pistol | WeaponKind::Smg => 420.0,
        WeaponKind::Shotgun | WeaponKind::Sxs | WeaponKind::Magnum => 520.0,
        WeaponKind::Rifle => 560.0,
        WeaponKind::Launcher => 820.0,
    };
    noise.level = noise.level.max(loud);

    let angle = p.angle;
    // Muzzle sits at each weapon's barrel tip (the rifle's barrel reaches well
    // out in front of the body) so the flash and rounds leave from the right spot.
    let (muzzle_dist, flash_scale) = match w.kind {
        WeaponKind::Rifle => (50.0, 1.5),
        WeaponKind::Smg => (42.0, 1.0),
        WeaponKind::Shotgun => (37.0, 1.15),
        WeaponKind::Sxs => (36.0, 1.25),
        WeaponKind::Launcher => (50.0, 1.8),
        WeaponKind::Magnum => (40.0, 1.2),
        _ => (34.0, 1.0), // pistol
    };
    let muzzle = pos + Vec2::new(angle.cos(), angle.sin()) * muzzle_dist;

    if w.kind == WeaponKind::Melee {
        // Alternate a wide slash and a forward stab on each strike. A stab lunges
        // a touch further with a narrow forward arc; a slash sweeps a wide arc.
        p.melee_stab = !p.melee_stab;
        let stab = p.melee_stab;
        p.swing_dur = if stab { 0.18 } else { 0.24 };
        p.swing_t = p.swing_dur;
        let reach = w.reach + p.r + if stab { 8.0 } else { 0.0 };
        let window = if stab { 0.45 } else { 0.95 };
        // A stab concentrates its force (more knockback, a hair more damage).
        let dmg = if stab { w.damage * 1.1 } else { w.damage };
        let knock = if stab { w.knockback * 1.3 } else { w.knockback };
        for (mut z, ztf) in zombies.iter_mut() {
            let zp = ztf.translation.truncate();
            let d = zp - pos;
            if d.length() < reach + z.r {
                let ad = (d.y.atan2(d.x) - angle).rem_euclid(TAU);
                let ad = if ad > std::f32::consts::PI { ad - TAU } else { ad };
                if ad.abs() < window {
                    z.hp -= dmg;
                    z.hurt_flash = 0.1;
                    z.apply_knockback(d.y.atan2(d.x), knock);
                    blood_burst(&mut commands, zp, d.y.atan2(d.x), 6);
                }
            }
        }
        shake.add(0.12);
        return;
    }

    if w.kind == WeaponKind::Flamethrower {
        // Continuous fire: spend a unit of fuel, spray a cone of flame from the
        // nozzle, and set alight any zombie caught in it (the burn does the work).
        let slot = p.current;
        p.clip[slot] -= 1;
        p.recoil = 0.25;
        p.muzzle = 0.05;
        shake.add(0.03);
        let mut rng = rand::thread_rng();
        let fwd = Vec2::new(angle.cos(), angle.sin());
        // Flame tongues licking forward, hot at the root and deep orange at the tip.
        for _ in 0..7 {
            let a = angle + rng.gen_range(-0.26..0.26);
            let reach = rng.gen_range(0.15..1.0) * w.range;
            let at = muzzle + Vec2::new(a.cos(), a.sin()) * reach;
            let t = reach / w.range;
            let col = if t < 0.35 {
                Color::srgb(1.0, 0.92, 0.55)
            } else if t < 0.7 {
                Color::srgb(1.0, 0.6, 0.2)
            } else {
                Color::srgb(0.9, 0.3, 0.12)
            };
            spawn_particle(
                &mut commands,
                at,
                Vec2::new(a.cos(), a.sin()) * rng.gen_range(120.0..240.0),
                col,
                rng.gen_range(5.0..11.0),
                rng.gen_range(0.18..0.4),
                0.0,
            );
        }
        // Dark smoke rolling off the far end of the jet.
        if rng.gen_bool(0.5) {
            let g = rng.gen_range(0.12..0.2);
            spawn_particle(
                &mut commands,
                muzzle + fwd * w.range * 0.6,
                Vec2::new(fwd.x * 40.0, 45.0),
                Color::srgba(g, g, g, 0.6),
                rng.gen_range(6.0..12.0),
                rng.gen_range(0.4..0.8),
                0.0,
            );
        }
        // Ignite + singe every zombie in the forward cone.
        for (mut z, ztf) in zombies.iter_mut() {
            let d = ztf.translation.truncate() - pos;
            let dist = d.length();
            if dist < w.range + z.r {
                let ad = (d.y.atan2(d.x) - angle).rem_euclid(TAU);
                let ad = if ad > std::f32::consts::PI { ad - TAU } else { ad };
                if ad.abs() < 0.45 {
                    z.hp -= w.damage;
                    z.hurt_flash = 0.05;
                    z.burning = z.burning.max(3.0);
                    z.apply_knockback(angle, w.knockback);
                }
            }
        }
        // Set flammable props in the cone alight too.
        for (mut pr, ptf) in props.iter_mut() {
            if pr.wrecked || !pr.flammable {
                continue;
            }
            let d = ptf.translation.truncate() - pos;
            let dist = d.length();
            if dist < w.range + pr.r {
                let ad = (d.y.atan2(d.x) - angle).rem_euclid(TAU);
                let ad = if ad > std::f32::consts::PI { ad - TAU } else { ad };
                if ad.abs() < 0.45 {
                    pr.burning = pr.burning.max(3.0);
                    pr.hp -= w.damage * 0.5;
                }
            }
        }
        return;
    }

    // Ranged: consume ammo, recoil, muzzle, casing, projectiles.
    let slot = p.current;
    // The side-by-side fires BOTH barrels in a single pull — it dumps every
    // chambered shell at once and then runs dry to reload. Everything else fires
    // one round per shot.
    let shots = if w.kind == WeaponKind::Sxs {
        p.clip[slot].max(1)
    } else {
        1
    };
    p.clip[slot] = (p.clip[slot] - shots).max(0);
    p.recoil = 1.0;
    p.muzzle = 0.06;
    shake.add(if w.explosive > 0.0 { 0.5 } else { 0.12 + w.knockback * 0.0006 });

    let mut rng = rand::thread_rng();
    // Eject a casing from the gun's breech (out to the right side of the slide),
    // flung a good distance with a little tumble. The break-action side-by-side
    // holds onto its shells until the breech is cracked open to reload, so it
    // does NOT throw a casing on firing.
    if w.kind != WeaponKind::Sxs {
        let ca = angle + std::f32::consts::FRAC_PI_2 + rng.gen_range(-0.3..0.3);
        let fwd = Vec2::new(angle.cos(), angle.sin());
        let side = Vec2::new((angle + std::f32::consts::FRAC_PI_2).cos(), (angle + std::f32::consts::FRAC_PI_2).sin());
        let eject = pos + fwd * 26.0 + side * 4.0;
        let brass = Color::srgb(0.78, 0.62, 0.22);
        commands.spawn((
            Sprite::from_color(brass, Vec2::new(3.0, 1.6)),
            Transform {
                translation: Vec3::new(eject.x, eject.y, Z_PARTICLE),
                rotation: Quat::from_rotation_z(rng.gen_range(0.0..TAU)),
                ..default()
            },
            Casing {
                vel: Vec2::new(ca.cos(), ca.sin()) * rng.gen_range(150.0..260.0),
                life: 1.6,
                spin: rng.gen_range(-24.0..24.0),
            },
        ));
    }

    // Pixelated muzzle flash: a blocky star of squares bursting from the tip.
    spawn_muzzle_flash(&mut commands, muzzle, angle, flash_scale, &mut rng);

    // Bazooka backblast: a plume of exhaust smoke shoots out the rear of the tube.
    if w.explosive > 0.0 {
        let fwd = Vec2::new(angle.cos(), angle.sin());
        let rear = pos - fwd * 14.0;
        for _ in 0..14 {
            let a = angle + std::f32::consts::PI + rng.gen_range(-0.5..0.5);
            let sp = rng.gen_range(120.0..360.0);
            let g = rng.gen_range(0.35..0.6);
            spawn_particle(
                &mut commands,
                rear + Vec2::new(rng.gen_range(-3.0..3.0), rng.gen_range(-3.0..3.0)),
                Vec2::new(a.cos(), a.sin()) * sp,
                Color::srgb(g, g, g),
                rng.gen_range(5.0..11.0),
                rng.gen_range(0.35..0.7),
                0.0,
            );
        }
        // A brief orange flare right at the vent.
        for _ in 0..5 {
            let a = angle + std::f32::consts::PI + rng.gen_range(-0.6..0.6);
            spawn_particle(
                &mut commands,
                rear,
                Vec2::new(a.cos(), a.sin()) * rng.gen_range(200.0..420.0),
                Color::srgb(1.0, 0.6, 0.2),
                rng.gen_range(3.0..5.0),
                rng.gen_range(0.1..0.22),
                0.0,
            );
        }
    }

    // A side-by-side dumping both barrels throws double the buckshot at once.
    for _ in 0..(w.pellets * shots as u32) {
        // Guard against a zero-spread weapon (e.g. the bazooka): sampling an
        // empty `-0.0..0.0` range panics rand and would crash the game.
        let a = if w.spread > 0.0 {
            angle + rng.gen_range(-w.spread..w.spread)
        } else {
            angle
        };
        commands.spawn((
            Sprite::from_color(
                if w.explosive > 0.0 {
                    Color::srgb(1.0, 0.7, 0.3)
                } else {
                    Color::srgb(1.0, 0.95, 0.7)
                },
                if w.explosive > 0.0 { Vec2::new(10.0, 5.0) } else { Vec2::new(7.0, 2.5) },
            ),
            Transform {
                translation: Vec3::new(muzzle.x, muzzle.y, Z_PROJECTILE),
                rotation: Quat::from_rotation_z(a),
                ..default()
            },
            Projectile {
                vel: Vec2::new(a.cos(), a.sin()) * w.speed,
                damage: w.damage,
                range: w.range,
                traveled: 0.0,
                knockback: w.knockback,
                sever: w.sever,
                explosive: w.explosive,
                pierce: w.pierce,
                ricochet: w.ricochet,
                wall_pierce: w.wall_pierce,
                falloff: w.falloff,
                hostile: false,
                hit: Vec::new(),
            },
        ));
    }
}

/// On the frame a magazine-fed reload begins, fling the spent magazine out of
/// the mag well so it drops away to the side and fades.
pub fn reload_fx(
    mut commands: Commands,
    q: Query<(&Player, &Transform)>,
    mut prev: Local<f32>,
) {
    let Ok((p, tf)) = q.single() else {
        *prev = 0.0;
        return;
    };
    let now = p.reloading;
    let kind = p.weapon().kind;
    let mag_fed = matches!(kind, WeaponKind::Pistol | WeaponKind::Smg | WeaponKind::Rifle);
    // Rising edge: a reload just started.
    if now > 0.0 && *prev <= 0.0 && mag_fed {
        let angle = p.angle;
        let pos = tf.translation.truncate();
        let fwd = Vec2::new(angle.cos(), angle.sin());
        // The mag well sits under the grip, a little in front of the body.
        let at = pos + fwd * 20.0;
        // Drop toward the near/lower side and slightly back.
        let mut rng = rand::thread_rng();
        let drop = angle - std::f32::consts::FRAC_PI_2 + rng.gen_range(-0.25..0.25);
        let mag = Color::srgb(0.05, 0.05, 0.06);
        commands.spawn((
            Sprite::from_color(mag, Vec2::new(4.0, 6.0)),
            Transform {
                translation: Vec3::new(at.x, at.y, Z_PARTICLE),
                rotation: Quat::from_rotation_z(angle),
                ..default()
            },
            Particle {
                vel: Vec2::new(drop.cos(), drop.sin()) * rng.gen_range(45.0..80.0),
                life: 0.8,
                max_life: 0.8,
                drag: 0.9,
                gravity: 0.0,
                base: mag,
            },
        ));
    }
    // Side-by-side break action: the instant it opens, the two spent shells are
    // thrown up and back out of the breech.
    if now > 0.0 && *prev <= 0.0 && kind == WeaponKind::Sxs {
        let angle = p.angle;
        let pos = tf.translation.truncate();
        let fwd = Vec2::new(angle.cos(), angle.sin());
        let breech = pos + fwd * 6.0;
        let mut rng = rand::thread_rng();
        let shell = Color::srgb(0.62, 0.16, 0.12); // spent red plastic hull
        for i in 0..2 {
            let side = if i == 0 { 1.0 } else { -1.0 };
            // Flicked back over the shoulder, fanning slightly apart.
            let a = angle + std::f32::consts::PI + side * 0.3 + rng.gen_range(-0.15..0.15);
            commands.spawn((
                Sprite::from_color(shell, Vec2::new(4.5, 2.4)),
                Transform {
                    translation: Vec3::new(breech.x, breech.y, Z_PARTICLE),
                    rotation: Quat::from_rotation_z(rng.gen_range(0.0..TAU)),
                    ..default()
                },
                Casing {
                    vel: Vec2::new(a.cos(), a.sin()) * rng.gen_range(140.0..230.0),
                    life: 1.7,
                    spin: rng.gen_range(-20.0..20.0),
                },
            ));
        }
    }
    *prev = now;
}

pub fn spit_system(mut ev: EventReader<SpitEvent>, mut commands: Commands) {
    for s in ev.read() {
        let muzzle = s.pos + Vec2::new(s.angle.cos(), s.angle.sin()) * 10.0;
        commands.spawn((
            Sprite::from_color(Color::srgb(0.5, 0.8, 0.25), Vec2::splat(6.0)),
            Transform::from_xyz(muzzle.x, muzzle.y, Z_PROJECTILE),
            Projectile {
                vel: Vec2::new(s.angle.cos(), s.angle.sin()) * 360.0,
                damage: 5.0,
                range: 420.0,
                traveled: 0.0,
                knockback: 0.0,
                sever: 0.0,
                explosive: 0.0,
                pierce: 0,
                ricochet: 0,
                wall_pierce: 0,
                falloff: 1.0,
                hostile: true,
                hit: Vec::new(),
            },
        ));
    }
}

pub fn projectile_system(
    time: Res<Time>,
    world: Res<World>,
    mut commands: Commands,
    mut explosions: EventWriter<Explosion>,
    mut proj_q: Query<(Entity, &mut Projectile, &mut Transform)>,
    mut zombies: Query<(Entity, &mut Zombie, &Transform), Without<Projectile>>,
    mut player_q: Query<(&mut Player, &Transform), (Without<Projectile>, Without<Zombie>)>,
    crows: Query<(Entity, &Transform), (With<crate::ambient::Crow>, Without<Projectile>, Without<Zombie>, Without<Player>)>,
    cats: Query<(Entity, &Transform, &crate::ambient::Cat), (Without<Projectile>, Without<Zombie>, Without<Player>)>,
    mut props: Query<(&mut crate::world::PropObj, &Transform), Without<Projectile>>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    // Player position (for proximity-scaled headshots) — read-only peek.
    let ppos = player_q
        .iter()
        .next()
        .map(|(_, t)| t.translation.truncate())
        .unwrap_or(Vec2::ZERO);
    for (pe, mut proj, mut tf) in proj_q.iter_mut() {
        let prev = tf.translation.truncate();
        let step = proj.vel * dt;
        let next = prev + step;
        proj.traveled += step.length();
        tf.translation.x = next.x;
        tf.translation.y = next.y;

        let mut dead = proj.traveled >= proj.range;

        if world.blocks_point(next) {
            // If a prop (not a solid wall tile) stopped the round, chip its hp.
            let (tc, tr) = world.world_to_tile(next);
            if !world.solid(tc, tr) {
                for (mut pr, ptf) in props.iter_mut() {
                    if pr.wrecked {
                        continue;
                    }
                    if (ptf.translation.truncate() - next).length() < pr.r + 3.0 {
                        pr.hp -= proj.damage;
                        break;
                    }
                }
            }
            if proj.explosive > 0.0 {
                explosions.write(Explosion {
                    pos: next,
                    radius: proj.explosive,
                    damage: proj.damage,
                    knockback: proj.knockback,
                    sever: proj.sever,
                });
                dead = true;
            } else if proj.ricochet > 0 {
                // Small-calibre bounce: flip the velocity component that crossed
                // the wall, back the round out of it, and lose a little power.
                let hit_x = world.blocks_point(Vec2::new(next.x, prev.y));
                let hit_y = world.blocks_point(Vec2::new(prev.x, next.y));
                if hit_x && !hit_y {
                    proj.vel.x = -proj.vel.x;
                } else if hit_y && !hit_x {
                    proj.vel.y = -proj.vel.y;
                } else {
                    proj.vel = -proj.vel;
                }
                proj.ricochet -= 1;
                proj.damage *= proj.falloff;
                tf.translation.x = prev.x;
                tf.translation.y = prev.y;
                // Re-point the tracer along its new heading (it was set at spawn).
                tf.rotation = Quat::from_rotation_z(proj.vel.y.atan2(proj.vel.x));
                // A few sparks off the wall.
                spark_burst(&mut commands, next, proj.vel.y.atan2(proj.vel.x));
            } else if proj.wall_pierce > 0 {
                // Big-calibre punch-through: step past the wall, keep going with
                // reduced killing power.
                let dir = proj.vel.normalize_or_zero();
                let mut p = next;
                let mut cleared = false;
                for _ in 0..24 {
                    p += dir * 4.0;
                    if !world.blocks_point(p) {
                        cleared = true;
                        break;
                    }
                }
                if cleared {
                    proj.wall_pierce -= 1;
                    proj.damage *= proj.falloff;
                    tf.translation.x = p.x;
                    tf.translation.y = p.y;
                    spark_burst(&mut commands, next, dir.y.atan2(dir.x));
                } else {
                    dead = true;
                }
            } else {
                dead = true;
            }
        }

        if proj.hostile {
            // Hit the player.
            if let Ok((mut p, ptf)) = player_q.single_mut() {
                let pp = ptf.translation.truncate();
                if (pp - next).length() < p.r + 4.0 {
                    p.hurt(proj.damage);
                    dead = true;
                    blood_burst(&mut commands, next, proj.vel.y.atan2(proj.vel.x), 4);
                }
            }
        } else {
            // Crows and cats can be shot (feathers/fur + a small carcass).
            let mut hit_crow = false;
            for (ce, ctf) in crows.iter() {
                if (ctf.translation.truncate() - next).length() < 9.0 {
                    crate::ambient::kill_crow(&mut commands, ctf.translation.truncate(), &mut rng);
                    commands.entity(ce).despawn();
                    dead = true;
                    hit_crow = true;
                    break;
                }
            }
            if !hit_crow {
                for (ce, ctf, cat) in cats.iter() {
                    if (ctf.translation.truncate() - next).length() < 9.0 {
                        crate::ambient::kill_cat(&mut commands, ctf.translation.truncate(), cat.fur, &mut rng);
                        commands.entity(ce).despawn();
                        dead = true;
                        hit_crow = true;
                        break;
                    }
                }
            }
            let dir = proj.vel.y.atan2(proj.vel.x);
            for (ze, mut z, ztf) in zombies.iter_mut() {
                if hit_crow {
                    break;
                }
                if proj.hit.contains(&ze) {
                    continue;
                }
                let zp = ztf.translation.truncate();
                if (zp - next).length() < z.r + 3.0 {
                    z.hp -= proj.damage;
                    z.hurt_flash = 0.09;
                    z.apply_knockback(dir, proj.knockback);
                    // Bloodier on every hit — the spray scales with the zombie's
                    // gore, and some solid hits blow a chunk of flesh/organ loose.
                    blood_burst(&mut commands, zp, dir, (5.0 + 4.0 * z.gore) as u32);
                    if rng.gen_bool(0.22) {
                        gib_spray(&mut commands, zp, dir, &z, &mut rng);
                    }
                    // A solid non-lethal hit can stagger them (stumble + slow);
                    // a heavy round is more likely to knock them off balance.
                    if z.hp > 0.0 {
                        if proj.knockback > 120.0 && rng.gen_bool(0.5) {
                            z.stagger = rng.gen_range(0.25..0.5);
                            z.apply_knockback(dir, proj.knockback * 0.6);
                        } else if rng.gen_bool(0.15) {
                            z.stagger = rng.gen_range(0.15..0.3);
                        }
                    }
                    proj.hit.push(ze);

                    // Headshot: the closer the zombie is to the player, the better
                    // the odds. On a hit the brains blow out the FAR side of the
                    // head and it drops (ragdoll corpse handled at death).
                    if proj.explosive <= 0.0 && z.hp > 0.0 {
                        let pdist = (zp - ppos).length();
                        let hs = (0.10 + (1.0 - (pdist / 520.0).clamp(0.0, 1.0)) * 0.42)
                            .clamp(0.0, 0.55);
                        if rng.gen_bool(hs as f64) {
                            z.hp = 0.0;
                            z.headshot = true;
                            let head = zp + Vec2::new(z.angle.cos(), z.angle.sin()) * z.r * 0.5;
                            brain_burst(&mut commands, head, dir);
                            dead = true;
                            break;
                        }
                        // Otherwise a solid hit may blow a limb clean off.
                        else if rng.gen_bool(0.22) && z.severed_mask != 0b1111 {
                            let mut choices: Vec<i8> = Vec::new();
                            for l in 0..4i8 {
                                if z.severed_mask & (1 << l) == 0 {
                                    // Don't sever a limb the look already lacks.
                                    let absent = (l == 0 && z.look.missing_arm == 0)
                                        || (l == 1 && z.look.missing_arm == 1)
                                        || (l == 2 && z.look.missing_leg == 0)
                                        || (l == 3 && z.look.missing_leg == 1);
                                    if !absent {
                                        choices.push(l);
                                    }
                                }
                            }
                            if !choices.is_empty() {
                                z.sever_pending = choices[rng.gen_range(0..choices.len())];
                            }
                        }
                    }

                    if proj.explosive > 0.0 {
                        explosions.write(Explosion {
                            pos: next,
                            radius: proj.explosive,
                            damage: proj.damage,
                            knockback: proj.knockback,
                            sever: proj.sever,
                        });
                        dead = true;
                        break;
                    }
                    if proj.pierce > 0 {
                        // Punches on through, losing killing power each target.
                        proj.pierce -= 1;
                        proj.damage *= proj.falloff;
                    } else {
                        dead = true;
                        break;
                    }
                }
            }
        }

        if dead {
            commands.entity(pe).despawn();
        }
    }
}

pub fn explosion_system(
    mut ev: EventReader<Explosion>,
    mut shake: ResMut<Shake>,
    mut conc: ResMut<crate::hud::Concussion>,
    mut commands: Commands,
    mut zombies: Query<(&mut Zombie, &Transform)>,
    mut player_q: Query<(&mut Player, &Transform), Without<Zombie>>,
    mut props: Query<(&mut crate::world::PropObj, &Transform), (Without<Zombie>, Without<Player>)>,
) {
    for ex in ev.read() {
        shake.add(0.7);
        // Fireball particles.
        let mut rng = rand::thread_rng();
        for _ in 0..26 {
            let a = rng.gen_range(0.0..TAU);
            let sp = rng.gen_range(60.0..320.0);
            let col = if rng.gen_bool(0.5) {
                Color::srgb(1.0, 0.7, 0.2)
            } else {
                Color::srgb(0.9, 0.3, 0.1)
            };
            spawn_particle(
                &mut commands,
                ex.pos,
                Vec2::new(a.cos(), a.sin()) * sp,
                col,
                rng.gen_range(4.0..8.0),
                rng.gen_range(0.3..0.6),
                0.0,
            );
        }
        // Scorch decal.
        commands.spawn((
            Sprite {
                color: Color::srgba(0.05, 0.05, 0.05, 0.7),
                custom_size: Some(Vec2::splat(ex.radius * 1.6)),
                ..default()
            },
            Transform::from_xyz(ex.pos.x, ex.pos.y, Z_DECAL + 1.0),
            Decal { life: 20.0 },
        ));
        for (mut z, ztf) in zombies.iter_mut() {
            let zp = ztf.translation.truncate();
            let d = zp - ex.pos;
            let dist = d.length();
            if dist < ex.radius + z.r {
                let falloff = 1.0 - (dist / (ex.radius + z.r)).clamp(0.0, 1.0);
                z.hp -= ex.damage * falloff;
                z.hurt_flash = 0.12;
                let a = d.y.atan2(d.x);
                z.apply_knockback(a, ex.knockback * falloff);
                blood_burst(&mut commands, zp, a, 6);
                if z.hp > 0.0 && rng.gen_bool(0.4) {
                    z.burning = z.burning.max(2.5); // caught in the fireball
                }
            }
        }

        // Blasts damage and ignite nearby props (chain-reacting cars/barrels).
        for (mut pr, ptf) in props.iter_mut() {
            if pr.wrecked {
                continue;
            }
            let d = (ptf.translation.truncate() - ex.pos).length();
            if d < ex.radius + pr.r {
                let falloff = (1.0 - d / (ex.radius + pr.r)).clamp(0.0, 1.0);
                pr.hp -= ex.damage * (0.5 + 0.5 * falloff);
                if pr.flammable {
                    pr.burning = pr.burning.max(3.0);
                }
            }
        }

        // Shockwave hits the player too: knocked back hard (scaled by closeness),
        // and a close-enough blast concusses — knocked out for a spell that grows
        // the nearer they were to the centre, with a disorienting screen effect.
        if let Ok((mut p, ptf)) = player_q.single_mut() {
            let pp = ptf.translation.truncate();
            let to = pp - ex.pos;
            let d = to.length();
            let shock = ex.radius * 2.4;
            if d < shock {
                let t = (1.0 - d / shock).clamp(0.0, 1.0); // 1 = point blank
                let a = if d > 0.001 { to.y.atan2(to.x) } else { 0.0 };
                p.vel += Vec2::new(a.cos(), a.sin()) * (120.0 + 520.0 * t);
                if t > 0.18 {
                    let stun = 0.25 + 1.5 * t * t;
                    p.stun = p.stun.max(stun);
                    conc.intensity = conc.intensity.max((0.4 + t).min(1.0));
                    p.hurt(ex.damage * 0.22 * t);
                }
            }
        }
    }
}

pub fn particle_system(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Particle, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (e, mut pa, mut tf, mut sprite) in q.iter_mut() {
        pa.life -= dt;
        if pa.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        let drag = pa.drag.powf(dt * 60.0);
        pa.vel *= drag;
        pa.vel.y -= pa.gravity * dt;
        tf.translation.x += pa.vel.x * dt;
        tf.translation.y += pa.vel.y * dt;
        let t = (pa.life / pa.max_life).clamp(0.0, 1.0);
        let b = pa.base.to_srgba();
        sprite.color = Color::srgba(b.red, b.green, b.blue, t);
    }
}

pub fn decal_system(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Decal, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (e, mut d, mut sprite) in q.iter_mut() {
        d.life -= dt;
        if d.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        if d.life < 3.0 {
            let a = sprite.color.alpha() * (d.life / 3.0).clamp(0.0, 1.0);
            sprite.color.set_alpha(a);
        }
    }
}

/// Shoot a limb clean off a wounded zombie: hide the limb, fling a fleshy gib and
/// a spray of blood, and mark it severed so it stays gone.
pub fn zombie_disfigure(
    mut commands: Commands,
    mut q: Query<(&mut Zombie, &crate::art::Rig, &Transform)>,
    mut vis_q: Query<&mut Visibility>,
) {
    let mut rng = rand::thread_rng();
    for (mut z, rig, tf) in q.iter_mut() {
        if z.sever_pending < 0 {
            continue;
        }
        let limb = z.sever_pending;
        z.sever_pending = -1;
        if z.severed_mask & (1 << limb) != 0 {
            continue;
        }
        z.severed_mask |= 1 << limb;
        let e = match limb {
            0 => rig.arm_l,
            1 => rig.arm_r,
            2 => rig.leg_l,
            _ => rig.leg_r,
        };
        if let Ok(mut v) = vis_q.get_mut(e) {
            *v = Visibility::Hidden;
        }
        let pos = tf.translation.truncate();
        let a = z.angle + rng.gen_range(-1.2..1.2);
        let dir = Vec2::new(a.cos(), a.sin());
        // The detached limb tumbles away as a chunk.
        let (col, w, h) = if limb < 2 {
            (z.look.skin, 11.0 * z.r / 12.0, 4.5 * z.r / 12.0)
        } else {
            (z.look.pants, 8.0 * z.r / 12.0, 5.0 * z.r / 12.0)
        };
        commands.spawn((
            Sprite::from_color(col, Vec2::new(w, h)),
            Transform {
                translation: Vec3::new(pos.x, pos.y, Z_PARTICLE + 1.0),
                rotation: Quat::from_rotation_z(a),
                ..default()
            },
            Particle {
                vel: dir * rng.gen_range(90.0..220.0),
                life: 0.9,
                max_life: 0.9,
                drag: 0.9,
                gravity: 0.0,
                base: col,
            },
        ));
        // A nub of exposed bone left in the gib's wake.
        commands.spawn((
            Sprite::from_color(Color::srgb(0.86, 0.83, 0.74), Vec2::splat(2.4)),
            Transform::from_xyz(pos.x, pos.y, Z_PARTICLE + 1.1),
            Particle {
                vel: dir * rng.gen_range(60.0..140.0),
                life: 0.7,
                max_life: 0.7,
                drag: 0.9,
                gravity: 0.0,
                base: Color::srgb(0.86, 0.83, 0.74),
            },
        ));
        blood_burst(&mut commands, pos, a, 7);
    }
}

/// Sprawl a fallen zombie into a jointed corpse: torso, splayed arms and legs, a
/// head (or a burst skull with a brain trail on a headshot), plus a blood pool
/// and some spilled guts. Everything fades over ~half a minute.
fn spawn_kill_corpse(commands: &mut Commands, art: &crate::art::Art, pos: Vec2, angle: f32, look: &crate::enemy::Look, scale: f32, headshot: bool, is_dog: bool, rng: &mut impl Rng) {
    // Match the living body's scale (radius-derived), so a corpse is the same
    // size as the zombie that just fell — big for brutes, not shrunk to the build
    // multiplier.
    let s = scale;
    let life = 30.0;
    let rot = angle + rng.gen_range(-0.5..0.5); // fell at a messy angle
    let (ca, sa) = (rot.cos(), rot.sin());
    // A dog leaves a small sprawled carcass: elongated body, splayed legs, head
    // and snout, a blood pool and a little spilled gut — no human clothing/ribs.
    if is_dog {
        let place = |commands: &mut Commands, c: Color, w: f32, h: f32, ox: f32, oy: f32, extra: f32, z: f32| {
            let wx = pos.x + ox * ca - oy * sa;
            let wy = pos.y + ox * sa + oy * ca;
            commands.spawn((
                Sprite::from_color(c, Vec2::new(w, h)),
                Transform {
                    translation: Vec3::new(wx, wy, z),
                    rotation: Quat::from_rotation_z(rot + extra),
                    ..default()
                },
                Decal { life },
            ));
        };
        commands.spawn((
            Sprite {
                image: art.soft.clone(),
                color: Color::srgba(0.22, 0.01, 0.02, 0.6),
                custom_size: Some(Vec2::splat(30.0 * s)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, Z_DECAL + 1.6),
            Decal { life },
        ));
        let fur = look.skin;
        let dark = look.shirt;
        let zc = Z_DECAL + 2.0;
        // Splayed legs.
        for (ox, oy) in [(6.0, 7.0), (6.0, -7.0), (-7.0, 7.0), (-7.0, -7.0)] {
            place(commands, fur, 8.0 * s, 3.0 * s, ox * s, oy * s, rng.gen_range(-0.8..0.8), zc);
        }
        // Body + head.
        place(commands, dark, 20.0 * s, 9.0 * s, 0.0, 0.0, 0.0, zc + 0.1);
        place(commands, fur, 9.0 * s, 8.0 * s, 12.0 * s, rng.gen_range(-3.0..3.0), 0.0, zc + 0.2);
        place(commands, fur, 6.0 * s, 4.0 * s, 17.0 * s, 0.0, 0.0, zc + 0.19);
        // A little spilled gut.
        for _ in 0..rng.gen_range(2..5) {
            let g: f32 = rng.gen_range(0.36..0.6);
            place(commands, Color::srgb(g, 0.12, 0.14), rng.gen_range(3.0..5.0) * s, rng.gen_range(2.5..4.0) * s, rng.gen_range(-6.0..4.0) * s, rng.gen_range(-6.0..6.0) * s, rng.gen_range(0.0..3.0), zc + 0.15);
        }
        return;
    }
    // A body lying flat sprawls larger than its standing top-down footprint.
    let s = s * 1.4;
    let place = |commands: &mut Commands, c: Color, w: f32, h: f32, ox: f32, oy: f32, extra: f32, z: f32| {
        let wx = pos.x + ox * ca - oy * sa;
        let wy = pos.y + ox * sa + oy * ca;
        commands.spawn((
            Sprite::from_color(c, Vec2::new(w, h)),
            Transform {
                translation: Vec3::new(wx, wy, z),
                rotation: Quat::from_rotation_z(rot + extra),
                ..default()
            },
            Decal { life },
        ));
    };
    // Rounded piece (uses the circle texture) — for the head/skull so it reads as
    // an actual head rather than a flat square.
    let round = |commands: &mut Commands, c: Color, w: f32, h: f32, ox: f32, oy: f32, extra: f32, z: f32| {
        let wx = pos.x + ox * ca - oy * sa;
        let wy = pos.y + ox * sa + oy * ca;
        commands.spawn((
            Sprite {
                image: art.circle.clone(),
                color: c,
                custom_size: Some(Vec2::new(w, h)),
                ..default()
            },
            Transform {
                translation: Vec3::new(wx, wy, z),
                rotation: Quat::from_rotation_z(rot + extra),
                ..default()
            },
            Decal { life },
        ));
    };
    // Blood pool under the body (soft gradient), plus a couple of smaller
    // offset pools so the edge is ragged and it looks like it spread unevenly.
    commands.spawn((
        Sprite {
            image: art.soft.clone(),
            color: Color::srgba(0.22, 0.01, 0.02, 0.6),
            custom_size: Some(Vec2::splat(38.0 * s)),
            ..default()
        },
        Transform::from_xyz(pos.x, pos.y, Z_DECAL + 1.6),
        Decal { life },
        BloodDecal,
    ));
    for _ in 0..rng.gen_range(2..5) {
        let o = Vec2::new(rng.gen_range(-16.0..16.0), rng.gen_range(-16.0..16.0)) * s;
        commands.spawn((
            Sprite {
                image: art.soft.clone(),
                color: Color::srgba(0.20, 0.008, 0.015, rng.gen_range(0.32..0.55)),
                custom_size: Some(Vec2::splat(rng.gen_range(16.0..30.0) * s)),
                ..default()
            },
            Transform::from_xyz(pos.x + o.x, pos.y + o.y, Z_DECAL + 1.55),
            Decal { life },
        ));
    }
    let shirt = look.shirt.to_srgba();
    let cloth = Color::srgb(shirt.red * 0.5, shirt.green * 0.5, shirt.blue * 0.5);
    let cloth_hi = Color::srgb(shirt.red * 0.62, shirt.green * 0.62, shirt.blue * 0.62);
    let skin = look.skin;
    let sk = skin.to_srgba();
    let skin_dark = Color::srgb(sk.red * 0.82, sk.green * 0.82, sk.blue * 0.82);
    let hand_c = Color::srgb(sk.red * 0.88, sk.green * 0.88, sk.blue * 0.88);
    let pants = look.pants;
    let pd = pants.to_srgba();
    let pants_dark = Color::srgb(pd.red * 0.78, pd.green * 0.78, pd.blue * 0.78);
    let bone = Color::srgb(0.86, 0.83, 0.74);
    let zc = Z_DECAL + 2.0;

    // A jointed limb built in body-local space: upper segment → joint → lower
    // segment (bent at the joint) → extremity (hand/foot). `la` is the local
    // splay angle. Captures the place/round helpers.
    let limb = |commands: &mut Commands,
                sx: f32, sy: f32, la: f32, u: f32, l: f32, w: f32,
                seg: Color, joint: Color, ext: Color, bend: f32| {
        let (dx, dy) = (la.cos(), la.sin());
        place(commands, seg, u, w, sx + dx * u * 0.5, sy + dy * u * 0.5, la, zc);
        round(commands, joint, w * 1.05, w * 1.05, sx + dx * u, sy + dy * u, 0.0, zc + 0.02);
        let la2 = la + bend;
        let (ex, ey) = (sx + dx * u, sy + dy * u);
        let (dx2, dy2) = (la2.cos(), la2.sin());
        place(commands, seg, l, w * 0.9, ex + dx2 * l * 0.5, ey + dy2 * l * 0.5, la2, zc + 0.01);
        round(commands, ext, w * 1.15, w * 1.15, ex + dx2 * l, ey + dy2 * l, 0.0, zc + 0.03);
    };

    // Legs (pants), splayed back and out, knees bent.
    limb(commands, -5.0 * s, 5.0 * s, 2.4 + rng.gen_range(-0.3..0.3), 8.5 * s, 8.0 * s, 5.2 * s, pants, pants_dark, pants_dark, rng.gen_range(-0.5..0.2));
    limb(commands, -5.0 * s, -5.0 * s, -2.4 + rng.gen_range(-0.3..0.3), 8.5 * s, 8.0 * s, 5.2 * s, pants, pants_dark, pants_dark, rng.gen_range(-0.2..0.5));
    // Torso (rounded, clothed) with a centre seam highlight.
    round(commands, cloth, 17.0 * s, 15.0 * s, 0.0, 0.0, 0.0, zc + 0.1);
    place(commands, cloth_hi, 3.5 * s, 13.0 * s, -1.0 * s, 0.0, 0.0, zc + 0.11);
    // Arms (skin), flung out to the sides, elbows bent, hands at the ends.
    limb(commands, 3.0 * s, 7.0 * s, 1.1 + rng.gen_range(-0.3..0.3), 7.5 * s, 7.0 * s, 4.4 * s, skin, skin_dark, hand_c, rng.gen_range(-0.3..0.6));
    limb(commands, 3.0 * s, -7.0 * s, -1.1 + rng.gen_range(-0.3..0.3), 7.5 * s, 7.0 * s, 4.4 * s, skin, skin_dark, hand_c, rng.gen_range(-0.6..0.3));

    // Head + face, lolled to one side.
    let oy: f32 = rng.gen_range(-4.0..4.0);
    let hx = 13.5 * s;
    if headshot {
        // Burst skull: broken shell, brain, bone chips, brains trailing out.
        round(commands, skin, 11.0 * s, 10.0 * s, hx, oy, 0.0, zc + 0.2);
        round(commands, Color::srgb(0.35, 0.05, 0.06), 8.0 * s, 7.0 * s, hx + 1.5 * s, oy, 0.0, zc + 0.22);
        place(commands, bone, 3.2 * s, 2.2 * s, hx + 2.0 * s, oy + 3.0 * s, 0.6, zc + 0.24);
        place(commands, bone, 3.2 * s, 2.2 * s, hx + 2.0 * s, oy - 3.0 * s, -0.6, zc + 0.24);
        let fwd = Vec2::new(ca, sa);
        for _ in 0..8 {
            let d = rng.gen_range(14.0..46.0);
            let off = fwd * d + Vec2::new(rng.gen_range(-9.0..9.0), rng.gen_range(-9.0..9.0));
            let g = rng.gen_range(0.4..0.6);
            commands.spawn((
                Sprite {
                    color: Color::srgba(g, g * 0.45, g * 0.5, 0.7),
                    custom_size: Some(Vec2::splat({ let v: f32 = rng.gen_range(2.0..4.5); v.round() })),
                    ..default()
                },
                Transform::from_xyz(pos.x + off.x, pos.y + off.y, Z_DECAL + 1.7),
                Decal { life },
            ));
        }
    } else {
        if look.hair >= 0 {
            round(commands, look.hair_col, 14.5 * s, 13.5 * s, hx - 2.0 * s, oy, 0.0, zc + 0.19);
        }
        round(commands, skin, 14.0 * s, 13.0 * s, hx, oy, 0.0, zc + 0.2);
        // Face. Some corpses have a partial (torn-away) face showing skull + teeth.
        let eye = Color::srgb(0.07, 0.05, 0.05);
        place(commands, skin_dark, 2.0 * s, 3.2 * s, hx + 5.0 * s, oy + 0.5 * s, 0.0, zc + 0.235); // nose
        round(commands, eye, 2.6 * s, 2.6 * s, hx + 3.0 * s, oy + 3.2 * s, 0.0, zc + 0.24);
        if rng.gen_bool(0.4) {
            // Torn half: exposed skull, teeth, and blood where the cheek was.
            round(commands, bone, 7.0 * s, 8.0 * s, hx + 1.0 * s, oy - 3.5 * s, 0.0, zc + 0.23);
            for k in 0..3 {
                place(commands, Color::srgb(0.9, 0.88, 0.8), 1.4 * s, 2.0 * s, hx + 4.2 * s, oy - 5.0 * s + k as f32 * 2.0 * s, 0.0, zc + 0.25);
            }
            place(commands, Color::srgb(0.3, 0.02, 0.03), 4.5 * s, 5.5 * s, hx + 1.5 * s, oy - 3.0 * s, 0.3, zc + 0.235);
        } else {
            round(commands, eye, 2.6 * s, 2.6 * s, hx + 3.0 * s, oy - 3.2 * s, 0.0, zc + 0.24);
            place(commands, Color::srgb(0.12, 0.05, 0.06), 4.8 * s, 2.6 * s, hx + 5.2 * s, oy, 0.2, zc + 0.24); // agape mouth
        }
    }
    // Guts spilling out of the torso: a connected rope of intestine plus loose
    // organ chunks, streaking off to one side as if they slid out when it fell.
    let spill = rng.gen_range(0.0..TAU);
    let sdir = Vec2::new(spill.cos(), spill.sin());
    let coils = rng.gen_range(4..8);
    for i in 0..coils {
        let t = i as f32 / coils as f32;
        // Wander outward with a sideways wobble so it coils rather than lines up.
        let along = 4.0 + t * 24.0;
        let side = (i as f32 * 1.7).sin() * 6.0;
        let perp = Vec2::new(-sdir.y, sdir.x);
        let off = sdir * along + perp * side;
        let g = 0.42 + rng.gen_range(0.0..0.16);
        place(
            commands,
            Color::srgb(g, 0.14, 0.15),
            rng.gen_range(4.0..7.0) * s,
            rng.gen_range(3.0..4.5) * s,
            off.x * s,
            off.y * s,
            rng.gen_range(0.0..TAU),
            zc + 0.15,
        );
    }
    // Loose organ chunks scattered near the belly.
    for _ in 0..rng.gen_range(3..7) {
        let ox = rng.gen_range(-5.0..9.0) * s;
        let oy = rng.gen_range(-9.0..9.0) * s;
        let pink = rng.gen_range(0.36..0.6);
        place(commands, Color::srgb(pink, 0.1, 0.13), rng.gen_range(3.0..6.0) * s, rng.gen_range(3.0..5.0) * s, ox, oy, rng.gen_range(0.0..3.0), zc + 0.16);
    }
    // A dark clotted blood streak pouring from the wound along the spill.
    for i in 0..rng.gen_range(3..6) {
        let d = 3.0 + i as f32 * 6.0;
        let off = sdir * d;
        place(commands, Color::srgb(0.18, 0.01, 0.02), rng.gen_range(3.0..5.5) * s, rng.gen_range(2.0..3.5) * s, off.x * s, off.y * s, spill, zc + 0.14);
    }
    // Exposed rib bones through the torn torso.
    place(commands, bone, 5.0 * s, 1.1 * s, 3.0 * s, 1.0 * s, 0.1, zc + 0.18);
    place(commands, bone, 5.0 * s, 1.1 * s, 3.0 * s, -1.5 * s, -0.1, zc + 0.18);
    place(commands, bone, 4.0 * s, 1.0 * s, 5.0 * s, 2.5 * s, 0.15, zc + 0.18);
    place(commands, bone, 4.0 * s, 1.0 * s, 5.0 * s, -3.0 * s, -0.15, zc + 0.18);
}

/// How long the ragdoll fall plays before the actor becomes a static corpse.
pub const DEATH_DUR: f32 = 0.62;

/// Drive the death of a zombie: when its hp hits zero it enters a short ragdoll
/// fall (handled by the animator), sliding along the shot's momentum and
/// trailing gore, then it's replaced by a detailed sprawled corpse. Score + kill
/// credit are counted once, the frame it starts dying.
pub fn zombie_death_system(
    time: Res<Time>,
    world: Res<World>,
    mut commands: Commands,
    art: Res<crate::art::Art>,
    mut score: ResMut<Score>,
    mut player_q: Query<&mut Player>,
    mut q: Query<(Entity, &mut Zombie, &mut Transform)>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (e, mut z, mut tf) in q.iter_mut() {
        // Kick off the death sequence the first frame hp reaches zero.
        if z.death_t <= 0.0 {
            if z.hp > 0.0 && !z.dead {
                continue;
            }
            z.death_t = 0.0001;
            let knock = z.knock;
            // Which way the body topples as it falls.
            z.death_spin = if rng.gen_bool(0.5) { 1.0 } else { -1.0 } * rng.gen_range(0.7..1.5);
            // Keep sliding: carry the shot's momentum, plus a shove that way.
            let dir = if knock.length_squared() > 1.0 {
                knock.normalize()
            } else {
                -Vec2::new(z.angle.cos(), z.angle.sin())
            };
            z.knock += dir * rng.gen_range(40.0..130.0);
            let pos = tf.translation.truncate();
            let ga = dir.y.atan2(dir.x);
            // A body killed by fire crumbles to ash with a puff of embers rather
            // than bursting in a wet spray of blood and guts.
            z.ash = z.burning > 0.0;
            if z.ash {
                ember_burst(&mut commands, pos, &mut rng);
            } else {
                let amount = (12.0 * z.gore) as u32 + if z.headshot { 8 } else { 0 };
                blood_burst(&mut commands, pos, ga, amount);
                gib_spray(&mut commands, pos, ga, &z, &mut rng);
            }
            // Score + kill credit, once.
            score.kills += 1;
            score.points += z.score;
            if let Ok(mut p) = player_q.single_mut() {
                p.kills += 1;
            }
        }

        // Advance the fall and slide along the (decaying) knockback.
        z.death_t += dt;
        let knock = z.knock * 0.002f32.powf(dt);
        z.knock = knock;
        let pos = tf.translation.truncate();
        let resolved = world.collide(pos + knock * dt, z.r * 0.6);
        tf.translation.x = resolved.x;
        tf.translation.y = resolved.y;
        tf.translation.z = depth_z(Z_CHAR, resolved.y) - 0.002;

        // Dribble blood/guts along the slide during the first part of the fall
        // (not for a body that's burning away — it trails embers, handled by the
        // burn FX instead).
        if !z.ash && z.death_t < DEATH_DUR * 0.8 && rng.gen_bool(0.5) {
            let sz: f32 = rng.gen_range(4.0..9.0);
            let o = Vec2::new(rng.gen_range(-6.0..6.0), rng.gen_range(-6.0..6.0));
            commands.spawn((
                Sprite {
                    color: Color::srgba(0.30, 0.02, 0.03, rng.gen_range(0.5..0.75)),
                    custom_size: Some(Vec2::splat(sz)),
                    ..default()
                },
                Transform::from_xyz(pos.x + o.x, pos.y + o.y, Z_DECAL + rng.gen_range(0.1..1.0)),
                Decal { life: rng.gen_range(12.0..22.0) },
            ));
        }

        // Fall finished: lay the corpse (a bloody sprawl, or a smoldering ash pile
        // if it burned) at the angle it toppled, and remove the actor.
        if z.death_t >= DEATH_DUR {
            if z.ash {
                spawn_ash_pile(&mut commands, &art, resolved, z.r / 12.0, &mut rng);
            } else {
                spawn_kill_corpse(
                    &mut commands,
                    &art,
                    resolved,
                    z.angle + z.death_spin,
                    &z.look,
                    z.r / 12.0,
                    z.headshot,
                    z.kind == crate::enemy::ZKind::Dog,
                    &mut rng,
                );
            }
            commands.entity(e).despawn();
        }
    }
}

/// Fling a few organ/gut chunks and a bone shard outward — used on death and
/// when a limb is blown off. Reads gore scale off the zombie.
fn gib_spray(commands: &mut Commands, pos: Vec2, dir: f32, z: &Zombie, rng: &mut impl Rng) {
    let n = (3.0 + 3.0 * z.gore) as u32;
    for _ in 0..n {
        let a = dir + rng.gen_range(-1.1..1.1);
        let sp = rng.gen_range(80.0..240.0);
        // Purple-red organ chunk.
        let g: f32 = rng.gen_range(0.34..0.56);
        commands.spawn((
            Sprite::from_color(
                Color::srgb(g, 0.1, 0.13),
                Vec2::splat(rng.gen_range(3.0..6.0)),
            ),
            Transform {
                translation: Vec3::new(pos.x, pos.y, Z_PARTICLE + 1.0),
                rotation: Quat::from_rotation_z(rng.gen_range(0.0..TAU)),
                ..default()
            },
            Particle {
                vel: Vec2::new(a.cos(), a.sin()) * sp,
                life: rng.gen_range(0.5..0.9),
                max_life: 0.9,
                drag: 0.9,
                gravity: 0.0,
                base: Color::srgb(g, 0.1, 0.13),
            },
        ));
    }
    // A shard of bone.
    let a = dir + rng.gen_range(-0.9..0.9);
    commands.spawn((
        Sprite::from_color(Color::srgb(0.86, 0.83, 0.74), Vec2::new(4.0, 1.8)),
        Transform {
            translation: Vec3::new(pos.x, pos.y, Z_PARTICLE + 1.1),
            rotation: Quat::from_rotation_z(a),
            ..default()
        },
        Particle {
            vel: Vec2::new(a.cos(), a.sin()) * rng.gen_range(120.0..260.0),
            life: 0.8,
            max_life: 0.8,
            drag: 0.9,
            gravity: 0.0,
            base: Color::srgb(0.86, 0.83, 0.74),
        },
    ));
}

/// A puff of glowing embers + smoke thrown off a body that burned up.
fn ember_burst(commands: &mut Commands, pos: Vec2, rng: &mut impl Rng) {
    for _ in 0..14 {
        let a = rng.gen_range(0.0..TAU);
        let sp = rng.gen_range(40.0..180.0);
        let hot = if rng.gen_bool(0.5) {
            Color::srgb(1.0, 0.55, 0.15)
        } else {
            Color::srgb(1.0, 0.8, 0.3)
        };
        commands.spawn((
            Sprite::from_color(hot, Vec2::splat(rng.gen_range(2.0..4.0))),
            Transform::from_xyz(pos.x, pos.y, Z_FX - 1.0),
            Particle {
                vel: Vec2::new(a.cos() * sp, a.sin() * sp * 0.5 + rng.gen_range(20.0..70.0)),
                life: rng.gen_range(0.4..0.9),
                max_life: 0.9,
                drag: 0.9,
                gravity: 0.0,
                base: hot,
            },
        ));
    }
    for _ in 0..6 {
        let g = rng.gen_range(0.1..0.2);
        let smoke = Color::srgba(g, g, g, 0.7);
        commands.spawn((
            Sprite::from_color(smoke, Vec2::splat(rng.gen_range(5.0..10.0))),
            Transform::from_xyz(pos.x, pos.y, Z_FX - 1.5),
            Particle {
                vel: Vec2::new(rng.gen_range(-16.0..16.0), 30.0 + rng.gen_range(0.0..40.0)),
                life: rng.gen_range(0.7..1.4),
                max_life: 1.4,
                drag: 0.94,
                gravity: 0.0,
                base: smoke,
            },
        ));
    }
}

/// A smoldering ash pile left where a zombie burned up: charred blobs, a scorch
/// mark, a scatter of bone, and a few lingering embers that fade.
fn spawn_ash_pile(commands: &mut Commands, art: &crate::art::Art, pos: Vec2, scale: f32, rng: &mut impl Rng) {
    let s = scale;
    let life = 30.0;
    // Scorched ground.
    commands.spawn((
        Sprite {
            image: art.soft.clone(),
            color: Color::srgba(0.05, 0.04, 0.04, 0.75),
            custom_size: Some(Vec2::splat(40.0 * s)),
            ..default()
        },
        Transform::from_xyz(pos.x, pos.y, Z_DECAL + 1.5),
        Decal { life },
    ));
    // Ash + charred flesh blobs.
    for _ in 0..rng.gen_range(7..12) {
        let o = Vec2::new(rng.gen_range(-11.0..11.0), rng.gen_range(-11.0..11.0)) * s;
        let g = rng.gen_range(0.06..0.18);
        commands.spawn((
            Sprite::from_color(
                Color::srgb(g, g * 0.9, g * 0.85),
                Vec2::splat(rng.gen_range(4.0..9.0) * s),
            ),
            Transform {
                translation: Vec3::new(pos.x + o.x, pos.y + o.y, Z_DECAL + 2.0 + rng.gen_range(0.0..0.4)),
                rotation: Quat::from_rotation_z(rng.gen_range(0.0..TAU)),
                ..default()
            },
            Decal { life },
        ));
    }
    // A few pale bone bits poking out of the ash.
    let bone = Color::srgb(0.7, 0.67, 0.6);
    for _ in 0..rng.gen_range(2..5) {
        let o = Vec2::new(rng.gen_range(-8.0..8.0), rng.gen_range(-8.0..8.0)) * s;
        commands.spawn((
            Sprite::from_color(bone, Vec2::new(rng.gen_range(3.0..5.0) * s, 1.4 * s)),
            Transform {
                translation: Vec3::new(pos.x + o.x, pos.y + o.y, Z_DECAL + 2.5),
                rotation: Quat::from_rotation_z(rng.gen_range(0.0..TAU)),
                ..default()
            },
            Decal { life },
        ));
    }
    // Lingering embers glowing in the ash.
    for _ in 0..8 {
        let o = Vec2::new(rng.gen_range(-10.0..10.0), rng.gen_range(-10.0..10.0)) * s;
        let hot = Color::srgb(1.0, rng.gen_range(0.4..0.6), 0.15);
        commands.spawn((
            Sprite::from_color(hot, Vec2::splat(rng.gen_range(1.5..3.0))),
            Transform::from_xyz(pos.x + o.x, pos.y + o.y, Z_FX - 2.0),
            Particle {
                vel: Vec2::new(rng.gen_range(-6.0..6.0), rng.gen_range(6.0..24.0)),
                life: rng.gen_range(0.8..2.2),
                max_life: 2.2,
                drag: 0.96,
                gravity: 0.0,
                base: hot,
            },
        ));
    }
}
