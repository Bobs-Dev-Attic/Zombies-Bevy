use crate::common::*;
use crate::enemy::{NewZombieRadius, Zombie};
use crate::player::Player;
use crate::weapons::WeaponKind;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Player body colours, shared between rig construction and the per-frame
/// hurt-flash tint so the base always restores correctly.
pub const PLAYER_JACKET: Color = Color::srgb(0.28, 0.33, 0.21);
pub const PLAYER_SKIN: Color = Color::srgb(0.33, 0.23, 0.18);

/// Shared generated textures for soft circular shapes.
#[derive(Resource)]
pub struct Art {
    pub circle: Handle<Image>,
    pub soft: Handle<Image>,
}

/// Marker: this character needs its visual rig built.
#[derive(Component)]
pub struct NeedsRig;

/// A ring of ticks above the player that fills as a reload cycles.
#[derive(Component)]
pub struct ReloadRing {
    pub ticks: Vec<Entity>,
}

const RELOAD_TICKS: usize = 14;

/// Entity handles to a character's body parts.
#[derive(Component)]
pub struct Rig {
    pub body: Entity,
    pub shadow: Entity,
    pub torso: Entity,
    pub head: Entity,
    pub arm_l: Entity,
    pub arm_r: Entity,
    pub leg_l: Entity,
    pub leg_r: Entity,
    pub weapon: Entity,
    pub flash: Entity,
}

fn make_circle(images: &mut Assets<Image>, size: u32) -> Handle<Image> {
    let mut data = vec![0u8; (size * size * 4) as usize];
    let c = (size as f32 - 1.0) / 2.0;
    let rad = c;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - c;
            let dy = y as f32 - c;
            let d = (dx * dx + dy * dy).sqrt();
            let a = if d <= rad - 0.75 {
                255.0
            } else if d <= rad + 0.25 {
                (rad + 0.25 - d) / 1.0 * 255.0
            } else {
                0.0
            };
            let i = ((y * size + x) * 4) as usize;
            data[i] = 255;
            data[i + 1] = 255;
            data[i + 2] = 255;
            data[i + 3] = a.clamp(0.0, 255.0) as u8;
        }
    }
    images.add(Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    ))
}

fn make_soft(images: &mut Assets<Image>, size: u32) -> Handle<Image> {
    let mut data = vec![0u8; (size * size * 4) as usize];
    let c = (size as f32 - 1.0) / 2.0;
    let rad = c;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - c;
            let dy = y as f32 - c;
            let d = ((dx * dx + dy * dy).sqrt() / rad).clamp(0.0, 1.0);
            let a = (1.0 - d).powf(1.7);
            let i = ((y * size + x) * 4) as usize;
            data[i] = 255;
            data[i + 1] = 255;
            data[i + 2] = 255;
            data[i + 3] = (a * 255.0) as u8;
        }
    }
    images.add(Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    ))
}

pub fn setup_art(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let circle = make_circle(&mut images, 48);
    let soft = make_soft(&mut images, 64);
    commands.insert_resource(Art { circle, soft });
}

fn ellipse(art: &Art, color: Color, w: f32, h: f32, z: f32) -> impl Bundle {
    (
        Sprite {
            image: art.circle.clone(),
            color,
            custom_size: Some(Vec2::new(w, h)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, z),
    )
}

fn rect(color: Color, w: f32, h: f32, z: f32) -> impl Bundle {
    (
        Sprite::from_color(color, Vec2::new(w, h)),
        Transform::from_xyz(0.0, 0.0, z),
    )
}

/// Build rigs for any character flagged with NeedsRig.
pub fn build_rigs(
    mut commands: Commands,
    art: Res<Art>,
    players: Query<Entity, (With<NeedsRig>, With<Player>)>,
    zombies: Query<(Entity, &Zombie, Option<&NewZombieRadius>), With<NeedsRig>>,
) {
    // Player rig.
    for e in players.iter() {
        build_player_rig(&mut commands, &art, e);
        commands.entity(e).remove::<NeedsRig>();
    }
    // Zombie rigs.
    for (e, z, _r) in zombies.iter() {
        build_zombie_rig(&mut commands, &art, e, z);
        commands.entity(e).remove::<NeedsRig>();
        commands.entity(e).remove::<NewZombieRadius>();
    }
}

fn build_player_rig(commands: &mut Commands, art: &Art, root: Entity) {
    // Palette matched to the reference art: olive-drab field jacket, dark skin,
    // a padded/segmented olive combat helmet, gunmetal pistol.
    let jacket = PLAYER_JACKET;
    let jacket_dark = Color::srgb(0.19, 0.23, 0.15);
    let skin = PLAYER_SKIN;
    let pants = Color::srgb(0.17, 0.18, 0.15);
    let helmet = Color::srgb(0.42, 0.40, 0.25);
    let helmet_dark = Color::srgb(0.30, 0.29, 0.17);

    // Shadow (does not rotate — child of root).
    let shadow = commands
        .spawn((
            Sprite {
                image: art.soft.clone(),
                color: Color::srgba(0.0, 0.0, 0.0, 0.38),
                custom_size: Some(Vec2::new(30.0, 20.0)),
                ..default()
            },
            Transform::from_xyz(0.0, -4.0, -0.5),
        ))
        .id();

    // Body pivot (rotates to face aim).
    let body = commands.spawn((Transform::default(), Visibility::default())).id();

    let leg_l = commands.spawn(rect(pants, 8.0, 6.0, -0.2)).id();
    let leg_r = commands.spawn(rect(pants, 8.0, 6.0, -0.2)).id();
    let torso = commands.spawn(ellipse(art, jacket, 22.0, 21.0, 0.0)).id();
    // Shoulder yoke for a bit of depth.
    let yoke = commands.spawn(ellipse(art, jacket_dark, 20.0, 16.0, 0.02)).id();
    let arm_l = commands.spawn(rect(jacket_dark, 13.0, 5.0, 0.1)).id();
    let arm_r = commands.spawn(rect(jacket_dark, 14.0, 5.0, 0.1)).id();
    // Dark-skinned face.
    let head = commands.spawn(ellipse(art, skin, 14.0, 14.0, 0.25)).id();

    // Padded/segmented combat helmet sitting on the crown (child of the head so
    // it tracks the aim), rendered just behind the face so the face peeks out.
    let helmet_base = commands.spawn(ellipse(art, helmet, 17.0, 16.0, -0.05)).id();
    commands.entity(helmet_base).insert(Transform::from_xyz(-3.5, 0.0, -0.05));
    // A few quilted segments for texture.
    let seg = |dx: f32, dy: f32| -> (Sprite, Transform) {
        (
            Sprite::from_color(helmet_dark, Vec2::new(3.0, 5.0)),
            Transform::from_xyz(dx, dy, -0.04),
        )
    };
    let hs1 = commands.spawn(seg(-2.0, -5.0)).id();
    let hs2 = commands.spawn(seg(-6.0, 0.0)).id();
    let hs3 = commands.spawn(seg(-2.0, 5.0)).id();
    commands.entity(head).add_children(&[helmet_base, hs1, hs2, hs3]);

    // Gunmetal 9mm: a slide plus a short grip.
    let weapon = commands.spawn(rect(Color::srgb(0.09, 0.09, 0.11), 18.0, 4.5, 0.15)).id();
    let grip = commands.spawn(rect(Color::srgb(0.06, 0.06, 0.07), 5.0, 6.0, 0.14)).id();
    commands.entity(grip).insert(Transform::from_xyz(-4.0, 3.0, 0.14));
    commands.entity(weapon).add_child(grip);
    let flash = commands
        .spawn((
            Sprite {
                image: art.soft.clone(),
                color: Color::srgba(1.0, 0.85, 0.4, 0.0),
                custom_size: Some(Vec2::new(24.0, 24.0)),
                ..default()
            },
            Transform::from_xyz(24.0, 0.0, 0.3),
        ))
        .id();

    commands
        .entity(body)
        .add_children(&[leg_l, leg_r, torso, yoke, arm_l, arm_r, weapon, head, flash]);

    // Reload cycle indicator: a ring of ticks floating above the head. Child of
    // root (not body) so it stays screen-aligned regardless of aim.
    let mut ticks = Vec::with_capacity(RELOAD_TICKS);
    let radius = 12.0;
    let cy = 26.0;
    for i in 0..RELOAD_TICKS {
        // Start at the top, go clockwise.
        let a = std::f32::consts::FRAC_PI_2 - (i as f32 / RELOAD_TICKS as f32) * std::f32::consts::TAU;
        let t = commands
            .spawn((
                Sprite::from_color(Color::srgba(1.0, 1.0, 1.0, 0.0), Vec2::splat(3.6)),
                Transform::from_xyz(a.cos() * radius, cy + a.sin() * radius, 2.0),
            ))
            .id();
        ticks.push(t);
    }
    commands.entity(root).add_children(&ticks);
    commands.entity(root).insert(ReloadRing { ticks });

    commands.entity(root).add_children(&[shadow, body]);
    commands.entity(root).insert(Rig {
        body,
        shadow,
        torso,
        head,
        arm_l,
        arm_r,
        leg_l,
        leg_r,
        weapon,
        flash,
    });
}

fn build_zombie_rig(commands: &mut Commands, art: &Art, root: Entity, z: &Zombie) {
    let look = z.look;
    let scale = z.r / 12.0;

    let shadow = commands
        .spawn((
            Sprite {
                image: art.soft.clone(),
                color: Color::srgba(0.0, 0.0, 0.0, 0.36),
                custom_size: Some(Vec2::new(30.0 * scale, 20.0 * scale)),
                ..default()
            },
            Transform::from_xyz(0.0, -4.0 * scale, -0.5),
        ))
        .id();

    let body = commands.spawn((Transform::default(), Visibility::default())).id();

    let leg_l = commands.spawn(rect(look.pants, 8.0 * scale, 6.0 * scale, -0.2)).id();
    let leg_r = commands.spawn(rect(look.pants, 8.0 * scale, 6.0 * scale, -0.2)).id();
    let torso = commands.spawn(ellipse(art, look.shirt, 22.0 * scale, 20.0 * scale, 0.0)).id();
    let arm_l = commands.spawn(rect(look.shirt, 13.0 * scale, 5.0 * scale, 0.1)).id();
    let arm_r = commands.spawn(rect(look.shirt, 13.0 * scale, 5.0 * scale, 0.1)).id();
    let head = commands.spawn(ellipse(art, look.skin, 15.0 * scale, 15.0 * scale, 0.25)).id();
    let hair_e = if look.hair >= 0 {
        let hh = if look.hair == 1 { 15.0 } else { 12.0 };
        Some(commands.spawn(ellipse(art, look.hair_col, 15.0 * scale, hh * scale, 0.26)).id())
    } else {
        None
    };
    // placeholders so Rig fields are populated
    let weapon = commands.spawn((Transform::default(), Visibility::Hidden)).id();
    let flash = commands.spawn((Transform::default(), Visibility::Hidden)).id();

    if let Some(h) = hair_e {
        commands.entity(head).add_child(h);
    }
    commands
        .entity(body)
        .add_children(&[leg_l, leg_r, torso, arm_l, arm_r, head]);
    commands.entity(root).add_children(&[shadow, body, weapon, flash]);
    commands.entity(root).insert(Rig {
        body,
        shadow,
        torso,
        head,
        arm_l,
        arm_r,
        leg_l,
        leg_r,
        weapon,
        flash,
    });
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    let a = a.to_srgba();
    let b = b.to_srgba();
    Color::srgb(
        a.red + (b.red - a.red) * t,
        a.green + (b.green - a.green) * t,
        a.blue + (b.blue - a.blue) * t,
    )
}

pub fn animate_player(
    player_q: Query<(&Player, &Rig)>,
    mut tf_q: Query<&mut Transform>,
    mut sprite_q: Query<&mut Sprite>,
) {
    let Ok((p, rig)) = player_q.single() else {
        return;
    };
    let angle = p.angle;
    if let Ok(mut b) = tf_q.get_mut(rig.body) {
        b.rotation = Quat::from_rotation_z(angle);
    }

    let run_amp = if p.running { 6.0 } else { 4.0 };
    let stride = (p.walk_frame).sin();
    let bob = if p.moving { p.walk_frame.sin().abs() } else { (p.idle_t * 2.2).sin() * 0.25 };

    // Legs scissor along local X (fore/aft), offset on perpendicular (local Y).
    if let Ok(mut l) = tf_q.get_mut(rig.leg_l) {
        l.translation.x = -3.0 + stride * run_amp;
        l.translation.y = 5.0;
    }
    if let Ok(mut r) = tf_q.get_mut(rig.leg_r) {
        r.translation.x = -3.0 - stride * run_amp;
        r.translation.y = -5.0;
    }
    if let Ok(mut t) = tf_q.get_mut(rig.torso) {
        t.translation.y = bob * 0.6;
    }
    if let Ok(mut h) = tf_q.get_mut(rig.head) {
        h.translation.x = 4.0;
        h.translation.y = bob * 0.5;
    }

    // Arms + weapon depend on weapon type / recoil / swing.
    let w = p.weapon();
    let melee = w.kind == WeaponKind::Melee;
    let recoil = p.recoil;
    if melee {
        // swing arc
        let sw = if p.swing_dur > 0.0 { p.swing_t / p.swing_dur } else { 0.0 };
        let swing = (1.0 - sw) * 1.4 - 0.7; // sweeps across
        if let Ok(mut a) = tf_q.get_mut(rig.arm_r) {
            a.translation = Vec3::new(8.0, -4.0, 0.1);
            a.rotation = Quat::from_rotation_z(swing);
        }
        if let Ok(mut a) = tf_q.get_mut(rig.arm_l) {
            a.translation = Vec3::new(6.0, 4.0, 0.1);
            a.rotation = Quat::from_rotation_z(swing * 0.6);
        }
        if let Ok(mut wt) = tf_q.get_mut(rig.weapon) {
            wt.translation = Vec3::new(14.0, -4.0, 0.15);
            wt.rotation = Quat::from_rotation_z(swing);
        }
    } else {
        // Two-handed pistol grip pushed out in front, recoiling backward on fire.
        let back = recoil * 5.0;
        if let Ok(mut a) = tf_q.get_mut(rig.arm_r) {
            a.translation = Vec3::new(11.0 - back, -2.5, 0.1);
            a.rotation = Quat::from_rotation_z(-0.10);
        }
        if let Ok(mut a) = tf_q.get_mut(rig.arm_l) {
            a.translation = Vec3::new(10.0 - back, 2.5, 0.1);
            a.rotation = Quat::from_rotation_z(0.10);
        }
        if let Ok(mut wt) = tf_q.get_mut(rig.weapon) {
            wt.translation = Vec3::new(18.0 - back, 0.0, 0.15);
            wt.rotation = Quat::IDENTITY;
        }
    }

    // Muzzle flash.
    if let Ok(mut fs) = sprite_q.get_mut(rig.flash) {
        fs.color = Color::srgba(1.0, 0.85, 0.4, (p.muzzle * 6.0).clamp(0.0, 1.0) * 0.9);
    }

    // Hurt flash tint.
    let flash = (p.hurt_flash * 5.0).clamp(0.0, 1.0);
    let jacket = PLAYER_JACKET;
    let skin = PLAYER_SKIN;
    if let Ok(mut s) = sprite_q.get_mut(rig.torso) {
        s.color = mix(jacket, Color::WHITE, flash * 0.7);
    }
    if let Ok(mut s) = sprite_q.get_mut(rig.head) {
        s.color = mix(skin, Color::WHITE, flash * 0.7);
    }
}

/// Light up the reload ring proportionally to the current reload's progress.
pub fn animate_reload_ring(
    player_q: Query<(&Player, &ReloadRing)>,
    mut sprite_q: Query<&mut Sprite>,
) {
    let Ok((p, ring)) = player_q.single() else {
        return;
    };
    let active = p.reloading > 0.0;
    let progress = p.reload_progress();
    let n = ring.ticks.len();
    for (i, &e) in ring.ticks.iter().enumerate() {
        if let Ok(mut s) = sprite_q.get_mut(e) {
            if !active {
                s.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
                continue;
            }
            let frac = (i as f32 + 1.0) / n as f32;
            if frac <= progress {
                // Filled tick — warm amber, fully lit.
                s.color = Color::srgba(1.0, 0.82, 0.30, 0.95);
            } else {
                // Pending tick — dim outline so the full cycle is visible.
                s.color = Color::srgba(0.9, 0.9, 1.0, 0.18);
            }
        }
    }
}

pub fn animate_zombies(
    zombies: Query<(&Zombie, &Rig)>,
    mut tf_q: Query<&mut Transform>,
    mut sprite_q: Query<&mut Sprite>,
) {
    for (z, rig) in zombies.iter() {
        let scale = z.r / 12.0;
        if let Ok(mut b) = tf_q.get_mut(rig.body) {
            // Shambling body sway around the facing angle.
            let sway = (z.frame * 1.5).sin() * 0.18;
            b.rotation = Quat::from_rotation_z(z.angle + sway);
        }
        let moving = z.vel.length_squared() > 4.0;
        let stride = if moving { (z.frame * z.stride_rate * 2.0).sin() } else { 0.0 };
        let amp = 5.0;
        if let Ok(mut l) = tf_q.get_mut(rig.leg_l) {
            l.translation.x = -2.0 + stride * amp;
            l.translation.y = 5.0 * scale;
        }
        if let Ok(mut r) = tf_q.get_mut(rig.leg_r) {
            r.translation.x = -2.0 - stride * amp;
            r.translation.y = -5.0 * scale;
        }
        // Reaching arms swing fore/aft.
        let reach = (z.frame * 1.3).sin() * 3.0;
        if let Ok(mut a) = tf_q.get_mut(rig.arm_l) {
            a.translation = Vec3::new(9.0 * scale + reach, 4.0 * scale, 0.1);
            a.rotation = Quat::from_rotation_z(0.2);
        }
        if let Ok(mut a) = tf_q.get_mut(rig.arm_r) {
            a.translation = Vec3::new(9.0 * scale - reach, -4.0 * scale, 0.1);
            a.rotation = Quat::from_rotation_z(-0.2);
        }
        if let Ok(mut h) = tf_q.get_mut(rig.head) {
            h.translation.x = 4.0 * scale;
        }

        // Hurt flash + low-hp darkening.
        let flash = (z.hurt_flash * 8.0).clamp(0.0, 1.0);
        let hp = (z.hp / z.max_hp).clamp(0.0, 1.0);
        let darken = 0.55 + 0.45 * hp;
        let shirt = z.look.shirt.to_srgba();
        let base = Color::srgb(shirt.red * darken, shirt.green * darken, shirt.blue * darken);
        if let Ok(mut s) = sprite_q.get_mut(rig.torso) {
            s.color = mix(base, Color::WHITE, flash);
        }
        if let Ok(mut s) = sprite_q.get_mut(rig.head) {
            let sk = z.look.skin.to_srgba();
            let skb = Color::srgb(sk.red * darken, sk.green * darken, sk.blue * darken);
            s.color = mix(skb, Color::WHITE, flash);
        }
    }
}
