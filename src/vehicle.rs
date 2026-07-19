use crate::common::*;
use crate::enemy::Zombie;
use crate::hud::{Cleanup, InteractBtn, InteractPrompt};
use crate::input::InputState;
use crate::player::Player;
use crate::world::{PropKind, World};
use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;

/// A parked car or van the player can climb into and drive. Lives on the same
/// entity as the prop's `PropObj` and visuals.
#[derive(Component)]
pub struct Vehicle {
    pub kind: PropKind,
    pub heading: f32,
    pub speed: f32, // units/sec along `heading`
    pub r: f32,     // collider radius
    pub half_len: f32,
    pub half_wid: f32,
    pub occupied: bool,
}

/// A swinging door, animated open then despawned. Child of the vehicle root.
#[derive(Component)]
pub struct VehicleDoor {
    pub closed: f32,
    pub open: f32,
    pub t: f32,
    pub life: f32,
}

/// Which vehicle (if any) the player is currently driving, and the nearest one
/// they could climb into (drives the on-screen prompt).
#[derive(Resource, Default)]
pub struct Driving {
    pub car: Option<Entity>,
    pub near: Option<Entity>,
}

impl Driving {
    pub fn active(&self) -> bool {
        self.car.is_some()
    }
}

const ENTER_RANGE: f32 = 74.0;

/// Spawn a pair of doors that swing open on the given vehicle, then self-despawn.
fn spawn_doors(commands: &mut Commands, car_e: Entity, v: &Vehicle) {
    let door_col = Color::srgb(0.13, 0.13, 0.16);
    let glass = Color::srgb(0.1, 0.13, 0.17);
    for side in [1.0f32, -1.0] {
        let door_len = v.half_len * 0.52;
        let hinge = commands
            .spawn((
                Transform::from_xyz(v.half_len * 0.14, side * v.half_wid, 0.42),
                Visibility::default(),
                VehicleDoor { closed: 0.0, open: -side * 1.35, t: 0.0, life: 1.0 },
            ))
            .id();
        let panel = commands
            .spawn((
                Sprite::from_color(door_col, Vec2::new(door_len, 4.5)),
                Transform::from_xyz(-door_len * 0.5, 0.0, 0.0),
            ))
            .id();
        let win = commands
            .spawn((
                Sprite::from_color(glass, Vec2::new(door_len * 0.5, 2.2)),
                Transform::from_xyz(-door_len * 0.5, 1.4, 0.05),
            ))
            .id();
        commands.entity(hinge).add_children(&[panel, win]);
        commands.entity(car_e).add_child(hinge);
    }
}

/// Enter the nearest vehicle, or step out of the one being driven. Also tracks
/// the nearest enterable vehicle each frame for the prompt.
pub fn vehicle_interact(
    input: Res<InputState>,
    mut driving: ResMut<Driving>,
    mut commands: Commands,
    mut world: ResMut<World>,
    mut player_q: Query<(&mut Player, &mut Transform, &mut Visibility), Without<Vehicle>>,
    mut veh_q: Query<(Entity, &mut Vehicle, &mut Transform), Without<Player>>,
) {
    let Ok((mut p, mut ptf, mut pvis)) = player_q.single_mut() else {
        return;
    };
    let ppos = ptf.translation.truncate();

    // --- Already driving: interact = get out. ---
    if let Some(car_e) = driving.car {
        if input.interact {
            if let Ok((_, mut v, vtf)) = veh_q.get_mut(car_e) {
                let cpos = vtf.translation.truncate();
                let heading = v.heading;
                v.occupied = false;
                v.speed = 0.0;
                // Restore a static collider where it now rests.
                world.park_vehicle(cpos, v.kind, v.r, heading);
                spawn_doors(&mut commands, car_e, &v);
                // Step out onto the driver side, clear of walls.
                let side = Vec2::new((heading + FRAC_PI_2).cos(), (heading + FRAC_PI_2).sin());
                let spot = world.collide(cpos + side * (v.half_wid + 26.0), p.r);
                ptf.translation.x = spot.x;
                ptf.translation.y = spot.y;
                ptf.translation.z = depth_z(Z_CHAR, spot.y);
                p.vel = Vec2::ZERO;
                p.invuln = 0.7;
                *pvis = Visibility::Inherited;
            }
            driving.car = None;
            driving.near = None;
        }
        return;
    }

    // --- On foot: find the nearest enterable vehicle. ---
    let mut best = ENTER_RANGE * ENTER_RANGE;
    let mut near = None;
    for (e, v, vtf) in veh_q.iter() {
        if v.occupied {
            continue;
        }
        let d2 = vtf.translation.truncate().distance_squared(ppos);
        if d2 < best {
            best = d2;
            near = Some(e);
        }
    }
    driving.near = near;

    if input.interact {
        if let Some(e) = near {
            if let Ok((_, mut v, vtf)) = veh_q.get_mut(e) {
                v.occupied = true;
                v.speed = 0.0;
                let cpos = vtf.translation.truncate();
                // Detach from its home chunk so streaming won't reclaim it while
                // it's being driven; tie it to the run's cleanup instead.
                world.detach_vehicle(e, cpos);
                commands.entity(e).insert(Cleanup);
                spawn_doors(&mut commands, e, &v);
            }
            *pvis = Visibility::Hidden;
            p.invuln = 0.7;
            driving.car = near;
            driving.near = None;
        }
    }
}

/// Drive the occupied vehicle: accelerate + steer from the movement input, run
/// over zombies, and carry the player anchor along so the camera, waves and
/// zombie targeting all follow the car.
pub fn vehicle_drive(
    time: Res<Time>,
    input: Res<InputState>,
    mut shake: ResMut<Shake>,
    driving: Res<Driving>,
    world: Res<World>,
    mut veh_q: Query<(&mut Vehicle, &mut Transform), Without<Player>>,
    mut player_q: Query<(&mut Player, &mut Transform), Without<Vehicle>>,
    mut zombies: Query<(&mut Zombie, &Transform), (Without<Player>, Without<Vehicle>)>,
) {
    let Some(car_e) = driving.car else {
        return;
    };
    let dt = time.delta_secs();
    let Ok((mut v, mut vtf)) = veh_q.get_mut(car_e) else {
        return;
    };

    let want = input.move_mag > 0.08;
    let van = v.kind == PropKind::Van;
    let max_speed = if van { 250.0 } else { 320.0 };
    let accel = if van { 200.0 } else { 265.0 };

    if want {
        v.speed = (v.speed + accel * dt).min(max_speed);
        // Steer toward the stick; grippier the faster you go, no turning at rest.
        let target = input.move_dir.y.atan2(input.move_dir.x);
        let grip = (v.speed / max_speed).clamp(0.0, 1.0);
        v.heading = angle_lerp(v.heading, target, (dt * (1.1 + 3.6 * grip)).clamp(0.0, 1.0));
    } else {
        v.speed *= 1.0 - (dt * 1.5).clamp(0.0, 1.0);
        if v.speed < 3.0 {
            v.speed = 0.0;
        }
    }

    let fwd = Vec2::new(v.heading.cos(), v.heading.sin());
    let pos = vtf.translation.truncate();
    let next = pos + fwd * v.speed * dt;
    let resolved = world.collide(next, v.r);
    if resolved.distance(next) > 1.5 && v.speed > 55.0 {
        shake.add(0.28);
        v.speed *= 0.32; // crunch into a wall
    }
    vtf.translation.x = resolved.x;
    vtf.translation.y = resolved.y;
    vtf.translation.z = depth_z(Z_CHAR, resolved.y) + 0.5;
    vtf.rotation = Quat::from_rotation_z(v.heading);
    // Engine rumble scales with speed.
    shake.add(v.speed / max_speed * 0.02);

    // Carry the (hidden) player along so everything that tracks the player
    // tracks the car. The driver is shielded while inside.
    if let Ok((mut p, mut ptf)) = player_q.single_mut() {
        ptf.translation.x = resolved.x;
        ptf.translation.y = resolved.y;
        ptf.translation.z = depth_z(Z_CHAR, resolved.y);
        p.vel = fwd * v.speed;
        p.angle = v.heading;
        p.invuln = p.invuln.max(0.2);
    }

    // Plow through zombies in the car's path.
    if v.speed > 45.0 {
        let sp = v.speed;
        for (mut z, ztf) in zombies.iter_mut() {
            if z.dead || z.death_t > 0.0 {
                continue;
            }
            let zp = ztf.translation.truncate();
            if zp.distance(resolved) < v.r + z.r * 0.7 {
                z.hp -= sp * 0.12 + 18.0;
                z.hurt_flash = 0.05;
                z.apply_knockback(v.heading, sp * 1.4);
                if sp > 150.0 {
                    z.dead = true;
                }
                shake.add(0.05);
                v.speed *= 0.99;
            }
        }
    }
}

/// Swing doors open, then despawn them.
pub fn vehicle_door_anim(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut VehicleDoor, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (e, mut d, mut tf) in q.iter_mut() {
        d.t += dt;
        d.life -= dt;
        let k = (d.t / 0.3).clamp(0.0, 1.0);
        let ang = d.closed + (d.open - d.closed) * k;
        tf.rotation = Quat::from_rotation_z(ang);
        if d.life <= 0.0 {
            commands.entity(e).try_despawn();
        }
    }
}

/// Show/hide the "drive" prompt (desktop text + mobile button) based on whether
/// a vehicle is in reach or being driven.
pub fn vehicle_prompt(
    input: Res<InputState>,
    driving: Res<Driving>,
    mut prompt_q: Query<&mut Node, (With<InteractPrompt>, Without<InteractBtn>)>,
    mut btn_q: Query<(&mut Node, &mut ImageNode), With<InteractBtn>>,
    mut prompt_text: Query<&mut Text, With<InteractPromptText>>,
    mut btn_text: Query<&mut Text, (With<InteractBtnText>, Without<InteractPromptText>)>,
) {
    let showing = driving.active() || driving.near.is_some();
    let label = if driving.active() { "EXIT" } else { "DRIVE" };

    if let Ok(mut n) = prompt_q.single_mut() {
        n.display = if showing && !input.touch_mode { Display::Flex } else { Display::None };
    }
    if let Ok(mut t) = prompt_text.single_mut() {
        **t = if driving.active() {
            "PRESS  E  -  EXIT".to_string()
        } else {
            "PRESS  E  -  DRIVE".to_string()
        };
    }
    if let Ok((mut n, mut img)) = btn_q.single_mut() {
        let show_btn = showing && input.touch_mode;
        n.display = if show_btn { Display::Flex } else { Display::None };
        n.left = Val::Px(input.interact_center.x - crate::input::BTN_R);
        n.top = Val::Px(input.interact_center.y - crate::input::BTN_R);
        let a = if input.interact_down { 0.7 } else { 0.4 };
        img.color = Color::srgba(0.3, 0.75, 0.45, a);
    }
    if let Ok(mut t) = btn_text.single_mut() {
        **t = label.to_string();
    }
}

/// Marker components on the prompt/button label text (added in `hud`).
#[derive(Component)]
pub struct InteractPromptText;
#[derive(Component)]
pub struct InteractBtnText;
