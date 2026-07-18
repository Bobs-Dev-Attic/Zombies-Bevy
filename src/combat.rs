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
    // A lingering ground stain.
    commands.spawn((
        Sprite {
            color: Color::srgba(0.30, 0.02, 0.03, 0.7),
            custom_size: Some(Vec2::splat(rng.gen_range(10.0..18.0))),
            ..default()
        },
        Transform::from_xyz(pos.x, pos.y, Z_DECAL + rng.gen_range(0.0..1.0)),
        Decal { life: 14.0 },
    ));
}

pub fn firing_system(
    time: Res<Time>,
    input: Res<InputState>,
    mut latch: ResMut<FireLatch>,
    mut shake: ResMut<Shake>,
    mut commands: Commands,
    mut q: Query<(&mut Player, &Transform)>,
    mut zombies: Query<(&mut Zombie, &Transform)>,
) {
    let _ = time;
    let Ok((mut p, tf)) = q.single_mut() else {
        return;
    };
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

    let angle = p.angle;
    let muzzle = pos + Vec2::new(angle.cos(), angle.sin()) * 18.0;

    if w.kind == WeaponKind::Melee {
        p.swing_dur = 0.22;
        p.swing_t = p.swing_dur;
        // Arc melee: hit zombies in front within reach.
        let reach = w.reach + p.r;
        for (mut z, ztf) in zombies.iter_mut() {
            let zp = ztf.translation.truncate();
            let d = zp - pos;
            if d.length() < reach + z.r {
                let ad = (d.y.atan2(d.x) - angle).rem_euclid(TAU);
                let ad = if ad > std::f32::consts::PI { ad - TAU } else { ad };
                if ad.abs() < 0.9 {
                    z.hp -= w.damage;
                    z.hurt_flash = 0.1;
                    z.apply_knockback(d.y.atan2(d.x), w.knockback);
                    blood_burst(&mut commands, zp, d.y.atan2(d.x), 6);
                }
            }
        }
        shake.add(0.12);
        return;
    }

    // Ranged: consume ammo, recoil, muzzle, casing, projectiles.
    let slot = p.current;
    p.clip[slot] -= 1;
    p.recoil = 1.0;
    p.muzzle = 0.06;
    shake.add(if w.explosive > 0.0 { 0.5 } else { 0.12 + w.knockback * 0.0006 });

    let mut rng = rand::thread_rng();
    // eject a casing
    let ca = angle + std::f32::consts::FRAC_PI_2 + rng.gen_range(-0.3..0.3);
    spawn_particle(
        &mut commands,
        muzzle,
        Vec2::new(ca.cos(), ca.sin()) * rng.gen_range(50.0..90.0),
        Color::srgb(0.75, 0.6, 0.2),
        2.0,
        0.5,
        0.0,
    );

    for _ in 0..w.pellets {
        let a = angle + rng.gen_range(-w.spread..w.spread);
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
                pierce: if w.kind == WeaponKind::Rifle { 1 } else { 0 },
                hostile: false,
                hit: Vec::new(),
            },
        ));
    }
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
) {
    let dt = time.delta_secs();
    for (pe, mut proj, mut tf) in proj_q.iter_mut() {
        let prev = tf.translation.truncate();
        let step = proj.vel * dt;
        let next = prev + step;
        proj.traveled += step.length();
        tf.translation.x = next.x;
        tf.translation.y = next.y;

        let mut dead = proj.traveled >= proj.range;

        if world.blocks_point(next) {
            dead = true;
            if proj.explosive > 0.0 {
                explosions.write(Explosion {
                    pos: next,
                    radius: proj.explosive,
                    damage: proj.damage,
                    knockback: proj.knockback,
                    sever: proj.sever,
                });
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
            let dir = proj.vel.y.atan2(proj.vel.x);
            for (ze, mut z, ztf) in zombies.iter_mut() {
                if proj.hit.contains(&ze) {
                    continue;
                }
                let zp = ztf.translation.truncate();
                if (zp - next).length() < z.r + 3.0 {
                    z.hp -= proj.damage;
                    z.hurt_flash = 0.09;
                    z.apply_knockback(dir, proj.knockback);
                    blood_burst(&mut commands, zp, dir, 5);
                    proj.hit.push(ze);
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
                        proj.pierce -= 1;
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
    mut commands: Commands,
    mut zombies: Query<(&mut Zombie, &Transform)>,
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

/// Turn dead zombies into corpses + gore and bump the score.
pub fn zombie_death_system(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut player_q: Query<&mut Player>,
    q: Query<(Entity, &Zombie, &Transform)>,
) {
    for (e, z, tf) in q.iter() {
        if z.hp > 0.0 && !z.dead {
            continue;
        }
        let pos = tf.translation.truncate();
        // Gore burst.
        blood_burst(&mut commands, pos, rand::thread_rng().gen_range(0.0..TAU), (10.0 * z.gore) as u32);
        // Corpse decal.
        let s = z.look.shirt.to_srgba();
        commands.spawn((
            Sprite::from_color(
                Color::srgb(s.red * 0.4, s.green * 0.4, s.blue * 0.4),
                Vec2::new(z.r * 2.4, z.r * 1.8),
            ),
            Transform::from_xyz(pos.x, pos.y, Z_DECAL + 2.0),
            Decal { life: 25.0 },
        ));
        score.kills += 1;
        score.points += z.score;
        if let Ok(mut p) = player_q.single_mut() {
            p.kills += 1;
        }
        commands.entity(e).despawn();
    }
}
