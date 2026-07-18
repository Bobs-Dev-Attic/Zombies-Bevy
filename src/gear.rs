use crate::art::Art;
use crate::common::*;
use crate::player::Player;
use crate::world::World;
use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::TAU;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PickupKind {
    Helmet,
    Armor,
    Medkit,
}

#[derive(Component)]
pub struct Pickup {
    pub kind: PickupKind,
    pub r: f32,
}

/// Marker on the bobbing icon container so we can float it.
#[derive(Component)]
pub struct PickupIcon {
    pub phase: f32,
}

/// Keeps a trickle of gear spawning during play.
#[derive(Resource)]
pub struct PickupSpawner {
    pub timer: f32,
}
impl Default for PickupSpawner {
    fn default() -> Self {
        Self { timer: 10.0 }
    }
}

fn rect(color: Color, w: f32, h: f32, z: f32) -> impl Bundle {
    (
        Sprite::from_color(color, Vec2::new(w, h)),
        Transform::from_xyz(0.0, 0.0, z),
    )
}
fn disc(art: &Art, color: Color, d: f32, z: f32) -> impl Bundle {
    (
        Sprite {
            image: art.circle.clone(),
            color,
            custom_size: Some(Vec2::splat(d)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, z),
    )
}

pub fn spawn_pickup(commands: &mut Commands, art: &Art, pos: Vec2, kind: PickupKind, rng: &mut impl Rng) {
    let root = commands
        .spawn((
            Pickup { kind, r: 13.0 },
            Transform::from_xyz(pos.x, pos.y, depth_z(Z_CHAR, pos.y)),
            Visibility::default(),
        ))
        .id();

    // Soft glow on the ground so pickups are easy to spot.
    let glow_color = match kind {
        PickupKind::Helmet => Color::srgba(0.5, 0.7, 0.4, 0.35),
        PickupKind::Armor => Color::srgba(0.5, 0.6, 0.8, 0.35),
        PickupKind::Medkit => Color::srgba(0.9, 0.3, 0.3, 0.35),
    };
    let glow = commands
        .spawn((
            Sprite {
                image: art.soft.clone(),
                color: glow_color,
                custom_size: Some(Vec2::splat(34.0)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, -0.5),
        ))
        .id();
    commands.entity(root).add_child(glow);

    // Bobbing icon container.
    let icon = commands
        .spawn((
            Transform::from_xyz(0.0, 0.0, 0.2),
            Visibility::default(),
            PickupIcon { phase: rng.gen_range(0.0..TAU) },
        ))
        .id();
    let mut parts: Vec<Entity> = Vec::new();
    match kind {
        PickupKind::Helmet => {
            let dome = commands.spawn(disc(art, Color::srgb(0.24, 0.28, 0.20), 18.0, 0.2)).id();
            let mut b = commands.spawn(rect(Color::srgb(0.14, 0.16, 0.13), 20.0, 5.0, 0.21));
            b.insert(Transform::from_xyz(0.0, -6.0, 0.21));
            let rim = b.id();
            let vis = commands.spawn(rect(Color::srgb(0.10, 0.11, 0.10), 8.0, 3.0, 0.22)).id();
            commands.entity(vis).insert(Transform::from_xyz(5.0, -4.0, 0.22));
            parts.extend([dome, rim, vis]);
        }
        PickupKind::Armor => {
            let vest = commands.spawn(rect(Color::srgb(0.16, 0.17, 0.15), 18.0, 20.0, 0.2)).id();
            let plate = commands.spawn(rect(Color::srgb(0.24, 0.26, 0.22), 12.0, 13.0, 0.21)).id();
            let s1 = commands.spawn(rect(Color::srgb(0.09, 0.09, 0.09), 4.0, 20.0, 0.22)).id();
            commands.entity(s1).insert(Transform::from_xyz(-5.0, 0.0, 0.22));
            let s2 = commands.spawn(rect(Color::srgb(0.09, 0.09, 0.09), 4.0, 20.0, 0.22)).id();
            commands.entity(s2).insert(Transform::from_xyz(5.0, 0.0, 0.22));
            parts.extend([vest, plate, s1, s2]);
        }
        PickupKind::Medkit => {
            let box_ = commands.spawn(rect(Color::srgb(0.88, 0.88, 0.9), 18.0, 16.0, 0.2)).id();
            let cv = commands.spawn(rect(Color::srgb(0.82, 0.12, 0.12), 5.0, 12.0, 0.21)).id();
            let ch = commands.spawn(rect(Color::srgb(0.82, 0.12, 0.12), 12.0, 5.0, 0.21)).id();
            parts.extend([box_, cv, ch]);
        }
    }
    commands.entity(icon).add_children(&parts);
    commands.entity(root).add_child(icon);
}

fn floor_point_away(world: &World, avoid: Vec2, rng: &mut impl Rng) -> Option<Vec2> {
    for _ in 0..40 {
        let a = rng.gen_range(0.0..TAU);
        let d = rng.gen_range(160.0..760.0);
        let p = avoid + Vec2::new(a.cos(), a.sin()) * d;
        if !world.blocks_point(p) {
            return Some(p);
        }
    }
    None
}

/// Scatter a starter set of gear across the map (called from start_game).
pub fn scatter_pickups(commands: &mut Commands, art: &Art, world: &World, center: Vec2) {
    let mut rng = rand::thread_rng();
    // A helmet and a vest right by the spawn so you gear up immediately.
    let near = [
        (Vec2::new(72.0, 0.0), PickupKind::Helmet),
        (Vec2::new(-72.0, 0.0), PickupKind::Armor),
    ];
    for (off, k) in near {
        let p = center + off;
        if !world.blocks_point(p) {
            spawn_pickup(commands, art, p, k, &mut rng);
        }
    }
    // The rest scattered around the map.
    let kinds = [
        PickupKind::Helmet,
        PickupKind::Armor,
        PickupKind::Medkit,
    ];
    for k in kinds {
        if let Some(p) = floor_point_away(world, center, &mut rng) {
            spawn_pickup(commands, art, p, k, &mut rng);
        }
    }
}

pub fn pickup_spawn_over_time(
    time: Res<Time>,
    mut spawner: ResMut<PickupSpawner>,
    art: Res<Art>,
    world: Res<World>,
    player_q: Query<&Transform, With<Player>>,
    existing: Query<(), With<Pickup>>,
    mut commands: Commands,
) {
    spawner.timer -= time.delta_secs();
    if spawner.timer > 0.0 {
        return;
    }
    spawner.timer = 14.0;
    if existing.iter().count() >= 8 {
        return;
    }
    let Ok(ptf) = player_q.single() else { return };
    let mut rng = rand::thread_rng();
    let kind = *[PickupKind::Helmet, PickupKind::Armor, PickupKind::Medkit]
        .get(rng.gen_range(0..3))
        .unwrap();
    if let Some(p) = floor_point_away(&world, ptf.translation.truncate(), &mut rng) {
        spawn_pickup(&mut commands, &art, p, kind, &mut rng);
    }
}

pub fn pickup_icon_bob(time: Res<Time>, mut q: Query<(&PickupIcon, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (icon, mut tf) in q.iter_mut() {
        tf.translation.y = (t * 3.0 + icon.phase).sin() * 3.0 + 2.0;
        tf.rotation = Quat::from_rotation_z((t * 1.2 + icon.phase).sin() * 0.12);
    }
}

pub fn pickup_collect(
    mut commands: Commands,
    mut player_q: Query<(&mut Player, &Transform)>,
    pickups: Query<(Entity, &Pickup, &Transform)>,
) {
    let Ok((mut p, ptf)) = player_q.single_mut() else { return };
    let pp = ptf.translation.truncate();
    for (e, pick, tf) in pickups.iter() {
        let d = (tf.translation.truncate() - pp).length();
        if d > pick.r + p.r {
            continue;
        }
        match pick.kind {
            PickupKind::Helmet => p.equip_helmet(55.0),
            PickupKind::Armor => p.equip_armor(110.0),
            PickupKind::Medkit => {
                if p.health >= p.max_health - 1.0 {
                    continue; // leave it for when we're hurt
                }
                p.heal_by(40.0);
            }
        }
        commands.entity(e).despawn();
    }
}
