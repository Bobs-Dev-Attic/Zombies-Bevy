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

const MM_SCALE: f32 = 3.0; // minimap pixels per world tile

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

/// Render the world grid to a small minimap texture (walls light, floor dark).
fn make_minimap(images: &mut Assets<Image>, world: &World) -> Handle<Image> {
    let cols = world.cols as u32;
    let rows = world.rows as u32;
    let px = MM_SCALE as u32;
    let (w, h) = (cols * px, rows * px);
    let mut data = vec![0u8; (w * h * 4) as usize];
    for r in 0..world.rows {
        for c in 0..world.cols {
            let wall = world.cells[r * world.cols + c] == crate::world::Cell::Wall;
            let (cr, cg, cb, ca) = if wall {
                (120u8, 124, 140, 235)
            } else {
                (26, 27, 33, 205)
            };
            for dy in 0..px {
                for dx in 0..px {
                    let x = c as u32 * px + dx;
                    let y = r as u32 * px + dy;
                    let i = ((y * w + x) * 4) as usize;
                    data[i] = cr;
                    data[i + 1] = cg;
                    data[i + 2] = cb;
                    data[i + 3] = ca;
                }
            }
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

/// Map a world position to minimap-local pixels (top-left origin).
fn world_to_mm(world: &World, p: Vec2) -> Vec2 {
    let wspan = world.cols as f32 * TILE;
    let hspan = world.rows as f32 * TILE;
    let u = (p.x / wspan).clamp(0.0, 1.0);
    let v = ((-p.y) / hspan).clamp(0.0, 1.0);
    Vec2::new(u * world.cols as f32 * MM_SCALE, v * world.rows as f32 * MM_SCALE)
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

    let mm = make_minimap(images, world);
    let mm_w = world.cols as f32 * MM_SCALE;
    let mm_h = world.rows as f32 * MM_SCALE;

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

/// Refresh the minimap dots (player, objective, nearest zombies).
pub fn minimap_system(
    world: Res<World>,
    obj: Res<Objective>,
    player_q: Query<&Transform, With<Player>>,
    zombies: Query<&Transform, With<Zombie>>,
    mut set: ParamSet<(
        Query<&mut Node, With<MMPlayer>>,
        Query<&mut Node, With<MMObjective>>,
        Query<(&MMZombie, &mut Node)>,
    )>,
) {
    if let Ok(ptf) = player_q.single() {
        let m = world_to_mm(&world, ptf.translation.truncate());
        if let Ok(mut n) = set.p0().single_mut() {
            n.left = Val::Px(m.x - 2.5);
            n.top = Val::Px(m.y - 2.5);
        }
    }
    {
        let m = world_to_mm(&world, obj.pos);
        if let Ok(mut n) = set.p1().single_mut() {
            n.left = Val::Px(m.x - 3.0);
            n.top = Val::Px(m.y - 3.0);
        }
    }
    // Zombies → red dots (first MM_ZOMBIE_DOTS of them).
    let mut points: Vec<Vec2> = zombies.iter().map(|t| t.translation.truncate()).collect();
    points.truncate(MM_ZOMBIE_DOTS);
    for (dot, mut n) in set.p2().iter_mut() {
        if let Some(&wp) = points.get(dot.0) {
            let m = world_to_mm(&world, wp);
            n.display = Display::Flex;
            n.left = Val::Px(m.x - 1.5);
            n.top = Val::Px(m.y - 1.5);
        } else {
            n.display = Display::None;
        }
    }
}
