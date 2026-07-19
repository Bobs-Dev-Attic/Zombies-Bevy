use crate::common::*;
use crate::enemy::Zombie;
use crate::hud::Cleanup;
use crate::player::Player;
use crate::world::World;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rand::Rng;
use std::f32::consts::TAU;

/// The current waypoint the player is directed toward ("extraction point").
#[derive(Resource, Default)]
pub struct Objective {
    pub pos: Vec2,
    pub index: u32,
    pub msg_t: f32, // brief "reached!" flash timer
}

const MM_PX: f32 = 132.0; // radar panel size in screen pixels
const MM_RANGE: f32 = 1050.0; // world units from the radar centre to its edge

#[derive(Component)]
pub struct MinimapPanel;
#[derive(Component)]
pub struct MMPlayer;
#[derive(Component)]
pub struct MMObjective;
#[derive(Component)]
pub struct MMZombie(pub usize);
#[derive(Component)]
pub struct ObjArrow;
#[derive(Component)]
pub struct ObjText;

const MM_ZOMBIE_DOTS: usize = 48;

fn pick_objective(world: &World, from: Vec2, rng: &mut impl Rng) -> Vec2 {
    let mut best = from + Vec2::new(300.0, 0.0);
    for _ in 0..60 {
        let a = rng.gen_range(0.0..TAU);
        let d = rng.gen_range(420.0..1000.0);
        let p = from + Vec2::new(a.cos(), a.sin()) * d;
        if !world.blocks_point(p) {
            best = p;
            break;
        }
    }
    best
}

/// A small filled arrow (triangle pointing +x) for the direction indicator.
fn make_arrow(images: &mut Assets<Image>) -> Handle<Image> {
    let (w, h) = (16u32, 16u32);
    let mut data = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let fx = x as f32 / (w as f32 - 1.0); // 0..1 along +x (tip at 1)
            let fy = (y as f32 / (h as f32 - 1.0) - 0.5).abs() * 2.0; // 0 centre .. 1 edge
            // Triangle: allowed half-width shrinks toward the tip.
            let inside = fy <= (1.0 - fx) * 1.05;
            let a = if inside && fx > 0.05 { 255u8 } else { 0 };
            let i = ((y * w + x) * 4) as usize;
            data[i] = 255;
            data[i + 1] = 245;
            data[i + 2] = 120;
            data[i + 3] = a;
        }
    }
    images.add(Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    ))
}

/// A small dark radar backdrop with a rim and a faint crosshair. The world is
/// endless, so the minimap is a player-centred radar rather than a whole map.
fn make_radar(images: &mut Assets<Image>) -> Handle<Image> {
    let s = 66u32;
    let mut data = vec![0u8; (s * s * 4) as usize];
    let c = (s as f32 - 1.0) / 2.0;
    for y in 0..s {
        for x in 0..s {
            let dx = x as f32 - c;
            let dy = y as f32 - c;
            let dist = (dx * dx + dy * dy).sqrt() / (s as f32 * 0.5);
            let (r, g, b, a) = if dist > 0.98 {
                (110u8, 114, 130, 235) // rim
            } else if (dx.abs() < 0.6 || dy.abs() < 0.6) && dist < 0.98 {
                (60, 64, 78, 200) // crosshair
            } else {
                (20, 21, 27, (200.0 * (1.0 - 0.4 * dist)) as u8)
            };
            let i = ((y * s + x) * 4) as usize;
            data[i] = r;
            data[i + 1] = g;
            data[i + 2] = b;
            data[i + 3] = a;
        }
    }
    images.add(Image::new(
        Extent3d { width: s, height: s, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    ))
}

/// Radar-local pixel for a world point relative to the player at the centre.
/// Returns (pixel, in_range); off-range points are clamped to the rim so the
/// objective still shows a direction.
fn radar_pos(player: Vec2, p: Vec2) -> (Vec2, bool) {
    let rel = p - player;
    let scale = (MM_PX * 0.5) / MM_RANGE;
    let mut px = MM_PX * 0.5 + rel.x * scale;
    let mut py = MM_PX * 0.5 - rel.y * scale; // world +y is up, screen +y is down
    let inside = rel.length() <= MM_RANGE;
    px = px.clamp(3.0, MM_PX - 3.0);
    py = py.clamp(3.0, MM_PX - 3.0);
    (Vec2::new(px, py), inside)
}

/// Build the objective, direction arrow and minimap. Called from `start_game`
/// while the freshly-generated world is still in hand (before it's a resource).
pub fn build_nav(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    world: &World,
    obj: &mut Objective,
    spawn: Vec2,
) {
    let mut rng = rand::thread_rng();
    *obj = Objective { pos: pick_objective(world, spawn, &mut rng), index: 1, msg_t: 0.0 };

    let mm = make_radar(images);
    let mm_w = MM_PX;
    let mm_h = MM_PX;

    // Direction arrow — a world sprite that hovers by the player and points at
    // the objective.
    commands.spawn((
        Sprite {
            image: make_arrow(images),
            color: Color::srgba(1.0, 0.92, 0.35, 0.9),
            custom_size: Some(Vec2::new(16.0, 12.0)),
            ..default()
        },
        Transform::from_xyz(spawn.x, spawn.y, Z_FX + 2.0),
        ObjArrow,
        Cleanup,
    ));

    // Objective banner (top-centre).
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(58.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(28),
            Cleanup,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(""),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::srgb(1.0, 0.88, 0.4)),
                ObjText,
            ));
        });

    // Minimap panel (top-right, under the wave/score text).
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(14.0),
                top: Val::Px(96.0),
                width: Val::Px(mm_w),
                height: Val::Px(mm_h),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor(Color::srgba(0.7, 0.72, 0.8, 0.6)),
            ImageNode { image: mm, ..default() },
            GlobalZIndex(28),
            MinimapPanel,
            Cleanup,
        ))
        .with_children(|p| {
            // Zombie dots (pool, hidden until used).
            for i in 0..MM_ZOMBIE_DOTS {
                p.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(3.0),
                        height: Val::Px(3.0),
                        display: Display::None,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.85, 0.2, 0.18)),
                    MMZombie(i),
                ));
            }
            // Objective marker (yellow).
            p.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(6.0),
                    height: Val::Px(6.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(1.0, 0.9, 0.3)),
                MMObjective,
            ));
            // Player dot (green).
            p.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(5.0),
                    height: Val::Px(5.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.4, 0.9, 0.45)),
                MMPlayer,
            ));
        });
}

/// Advance the objective when reached, and point the arrow + banner at it.
pub fn objective_system(
    time: Res<Time>,
    world: Res<World>,
    mut obj: ResMut<Objective>,
    player_q: Query<&Transform, With<Player>>,
    mut arrow_q: Query<(&mut Transform, &mut Visibility), (With<ObjArrow>, Without<Player>)>,
    mut text_q: Query<&mut Text, With<ObjText>>,
) {
    let dt = time.delta_secs();
    obj.msg_t = (obj.msg_t - dt).max(0.0);
    let Ok(ptf) = player_q.single() else { return };
    let ppos = ptf.translation.truncate();
    let to = obj.pos - ppos;
    let dist = to.length();

    // Reached → pick the next one.
    if dist < 36.0 {
        obj.index += 1;
        obj.msg_t = 2.2;
        let mut rng = rand::thread_rng();
        obj.pos = pick_objective(&world, ppos, &mut rng);
    }

    // Arrow hovers a fixed distance from the player, aimed at the objective.
    if let Ok((mut atf, mut vis)) = arrow_q.single_mut() {
        let dir = to.normalize_or_zero();
        let at = ppos + dir * 42.0;
        atf.translation.x = at.x;
        atf.translation.y = at.y;
        atf.translation.z = Z_FX + 2.0;
        atf.rotation = Quat::from_rotation_z(dir.y.atan2(dir.x));
        *vis = if dist > 60.0 { Visibility::Inherited } else { Visibility::Hidden };
    }

    if let Ok(mut t) = text_q.single_mut() {
        **t = if obj.msg_t > 0.0 {
            "OBJECTIVE REACHED".to_string()
        } else {
            format!("OBJECTIVE  >>  {}m", (dist / 10.0).round() as i32)
        };
    }
}

/// Refresh the radar dots (player centred; objective + nearby zombies relative).
pub fn minimap_system(
    obj: Res<Objective>,
    player_q: Query<&Transform, With<Player>>,
    zombies: Query<&Transform, With<Zombie>>,
    mut set: ParamSet<(
        Query<&mut Node, With<MMPlayer>>,
        Query<&mut Node, With<MMObjective>>,
        Query<(&MMZombie, &mut Node)>,
    )>,
) {
    let Ok(ptf) = player_q.single() else { return };
    let pp = ptf.translation.truncate();
    // Player always at the centre.
    if let Ok(mut n) = set.p0().single_mut() {
        n.left = Val::Px(MM_PX * 0.5 - 2.5);
        n.top = Val::Px(MM_PX * 0.5 - 2.5);
    }
    // Objective — clamped to the rim if beyond range so it points the way.
    {
        let (m, _) = radar_pos(pp, obj.pos);
        if let Ok(mut n) = set.p1().single_mut() {
            n.left = Val::Px(m.x - 3.0);
            n.top = Val::Px(m.y - 3.0);
        }
    }
    // Zombies within range → red dots.
    let mut points: Vec<Vec2> = zombies.iter().map(|t| t.translation.truncate()).collect();
    points.truncate(MM_ZOMBIE_DOTS);
    for (dot, mut n) in set.p2().iter_mut() {
        if let Some(&wp) = points.get(dot.0) {
            let (m, inside) = radar_pos(pp, wp);
            if inside {
                n.display = Display::Flex;
                n.left = Val::Px(m.x - 1.5);
                n.top = Val::Px(m.y - 1.5);
            } else {
                n.display = Display::None;
            }
        } else {
            n.display = Display::None;
        }
    }
}
