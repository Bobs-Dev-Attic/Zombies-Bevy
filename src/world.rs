use crate::common::*;
use crate::player::Player;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Floor,
    Wall,
}

/// Scattered scenery. Solid kinds are circular obstacles that block movement,
/// bullets, flames and rolling grenades; bushes are passable cover.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PropKind {
    Tree,
    Bush,
    Car,
    Van,
    Bench,
    Dumpster,
    Barrel,
    Crate,
    Hydrant,
    Table,
    Sofa,
}

/// Radius of the collider and whether it blocks movement/shots. Sized close to
/// real-world proportions relative to the ~1.7m-tall player.
pub fn prop_spec(kind: PropKind) -> (f32, bool) {
    match kind {
        PropKind::Tree => (14.0, true),
        PropKind::Bush => (17.0, false),
        PropKind::Car => (30.0, true),
        PropKind::Van => (35.0, true),
        PropKind::Bench => (17.0, true),
        PropKind::Dumpster => (21.0, true),
        PropKind::Barrel => (10.0, true),
        PropKind::Crate => (13.0, true),
        PropKind::Hydrant => (7.0, true),
        PropKind::Table => (15.0, true),
        PropKind::Sofa => (19.0, true),
    }
}

/// Destructibility of a prop: (hit points, flammable, explodes when destroyed).
pub fn prop_stats(kind: PropKind) -> (f32, bool, bool) {
    match kind {
        PropKind::Tree => (75.0, true, false),
        PropKind::Bush => (18.0, true, false),
        PropKind::Car => (130.0, true, true),
        PropKind::Van => (150.0, true, true),
        PropKind::Bench => (45.0, true, false),
        PropKind::Dumpster => (110.0, true, false),
        PropKind::Barrel => (28.0, true, true),
        PropKind::Crate => (34.0, true, false),
        PropKind::Hydrant => (400.0, false, false),
        PropKind::Table => (32.0, true, false),
        PropKind::Sofa => (42.0, true, false),
    }
}

/// A prop that can be shot, burned, flipped and wrecked. Lives on the prop root
/// entity alongside its visuals; the static collider in `World.props` is left in
/// place as wreckage.
#[derive(Component)]
pub struct PropObj {
    pub kind: PropKind,
    pub hp: f32,
    pub max_hp: f32,
    pub r: f32,
    pub flammable: bool,
    pub explodes: bool,
    pub burning: f32,
    pub burn_fx: f32,
    pub wrecked: bool,
}

#[derive(Clone, Copy)]
pub struct Prop {
    pub kind: PropKind,
    pub pos: Vec2,
    pub r: f32,
    pub solid: bool,
    pub angle: f32,
}

/// Which environment this run generates. Each picks a different wall layout,
/// ground colour and prop mix.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Scene {
    Streets,
    Park,
    Neighborhood,
}

impl Scene {
    pub fn label(self) -> &'static str {
        match self {
            Scene::Streets => "Downtown Streets",
            Scene::Park => "City Park",
            Scene::Neighborhood => "The Neighborhood",
        }
    }
}

/// The scenario the player picked on the menu; `None` = random each run.
#[derive(Resource, Default)]
pub struct SceneChoice(pub Option<Scene>);

/// Side length of a streaming chunk, in tiles.
pub const CHUNK: i32 = 20;
/// Chunks kept loaded out from the player's chunk — a (2R+1)² window.
pub const CHUNK_R: i32 = 2;

/// One loaded piece of the endless map.
pub struct Chunk {
    pub cells: Vec<Cell>,  // CHUNK*CHUNK, row-major local tiles
    pub props: Vec<Prop>,  // world-space colliders
    pub ents: Vec<Entity>, // spawned visuals, despawned when the chunk unloads
}

/// The world is an infinite grid of chunks streamed in around the player.
#[derive(Resource)]
pub struct World {
    pub seed: u64,
    pub scene: Scene,
    pub spawn: Vec2,
    pub chunks: HashMap<(i32, i32), Chunk>,
}

/// Global tile → (chunk key, local index within that chunk).
#[inline]
fn tile_to_chunk(gc: i32, gr: i32) -> ((i32, i32), usize) {
    let cx = gc.div_euclid(CHUNK);
    let cy = gr.div_euclid(CHUNK);
    let lx = gc.rem_euclid(CHUNK) as usize;
    let ly = gr.rem_euclid(CHUNK) as usize;
    ((cx, cy), ly * CHUNK as usize + lx)
}

impl World {
    pub fn world_to_tile(&self, p: Vec2) -> (i32, i32) {
        ((p.x / TILE).floor() as i32, (-p.y / TILE).floor() as i32)
    }

    /// Center of a tile in world coordinates.
    pub fn tile_center(&self, gc: i32, gr: i32) -> Vec2 {
        Vec2::new(gc as f32 * TILE + TILE * 0.5, -(gr as f32 * TILE + TILE * 0.5))
    }

    /// Is this tile a wall? Tiles in not-yet-loaded chunks read as open floor
    /// (they're always beyond the player's reach until they load).
    pub fn solid(&self, gc: i32, gr: i32) -> bool {
        let (key, idx) = tile_to_chunk(gc, gr);
        match self.chunks.get(&key) {
            Some(ch) => ch.cells[idx] == Cell::Wall,
            None => false,
        }
    }

    /// Copy of all props in the chunk containing `p` and its 8 neighbours.
    fn near_props(&self, p: Vec2) -> Vec<Prop> {
        let (gc, gr) = self.world_to_tile(p);
        let ((cx, cy), _) = tile_to_chunk(gc, gr);
        let mut out = Vec::new();
        for dy in -1..=1 {
            for dx in -1..=1 {
                if let Some(ch) = self.chunks.get(&(cx + dx, cy + dy)) {
                    out.extend(ch.props.iter().copied());
                }
            }
        }
        out
    }

    /// Inside a solid tile or a solid prop? (projectiles, flames, casings, grenades)
    pub fn blocks_point(&self, p: Vec2) -> bool {
        let (gc, gr) = self.world_to_tile(p);
        if self.solid(gc, gr) {
            return true;
        }
        self.near_props(p)
            .iter()
            .any(|pr| pr.solid && p.distance(pr.pos) < pr.r)
    }

    /// Push a circle out of any solid tiles / props it overlaps.
    pub fn collide(&self, mut p: Vec2, radius: f32) -> Vec2 {
        let (cc, cr) = self.world_to_tile(p);
        for r in (cr - 1)..=(cr + 1) {
            for c in (cc - 1)..=(cc + 1) {
                if !self.solid(c, r) {
                    continue;
                }
                let min = Vec2::new(c as f32 * TILE, -((r as f32 + 1.0) * TILE));
                let max = Vec2::new((c as f32 + 1.0) * TILE, -(r as f32 * TILE));
                let closest = p.clamp(min, max);
                let delta = p - closest;
                let d = delta.length();
                if d < radius {
                    if d > 0.0001 {
                        p += delta / d * (radius - d);
                    } else {
                        let dx = (p.x - min.x).min(max.x - p.x);
                        let dy = (p.y - min.y).min(max.y - p.y);
                        if dx < dy {
                            p.x += if p.x - min.x < max.x - p.x { -(dx + radius) } else { dx + radius };
                        } else {
                            p.y += if p.y - min.y < max.y - p.y { -(dy + radius) } else { dy + radius };
                        }
                    }
                }
            }
        }
        for prop in self.near_props(p) {
            if !prop.solid {
                continue;
            }
            let delta = p - prop.pos;
            let d = delta.length();
            let rr = radius + prop.r;
            if d < rr {
                if d > 0.0001 {
                    p += delta / d * (rr - d);
                } else {
                    p += Vec2::new(rr, 0.0);
                }
            }
        }
        p
    }
}

/// Build the arena for a randomly chosen scene: a downtown street grid, an open
/// city park, or a house-lined neighborhood — each with its own walls and props.
/// Create a fresh streaming world (no chunks loaded yet).
pub fn new_world(choice: Option<Scene>) -> World {
    let mut rng = rand::thread_rng();
    let scene = choice.unwrap_or_else(|| {
        *[Scene::Streets, Scene::Park, Scene::Neighborhood]
            .get(rng.gen_range(0..3))
            .unwrap()
    });
    let seed: u64 = rng.gen();
    // Spawn at the centre of chunk (0,0).
    let spawn = Vec2::new(CHUNK as f32 * TILE * 0.5, -(CHUNK as f32 * TILE * 0.5));
    World { seed, scene, spawn, chunks: HashMap::new() }
}

/// Deterministic per-chunk generation of wall cells + prop colliders.
fn gen_chunk(cx: i32, cy: i32, seed: u64, scene: Scene, spawn: Vec2) -> (Vec<Cell>, Vec<Prop>) {
    let cu = CHUNK as usize;
    let mut cells = vec![Cell::Floor; cu * cu];
    let h = seed
        ^ (cx as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (cy as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    let mut rng = StdRng::seed_from_u64(h);
    let set = |cells: &mut Vec<Cell>, lx: i32, ly: i32, v: Cell| {
        if lx >= 0 && ly >= 0 && lx < CHUNK && ly < CHUNK {
            cells[ly as usize * cu + lx as usize] = v;
        }
    };
    match scene {
        Scene::Streets => {
            // Building blocks with margins so roads run between chunks.
            for _ in 0..rng.gen_range(1..=2) {
                let bw = rng.gen_range(4..8);
                let bh = rng.gen_range(3..6);
                let bx = rng.gen_range(3..(CHUNK - 3 - bw).max(4));
                let by = rng.gen_range(3..(CHUNK - 3 - bh).max(4));
                for ry in by..(by + bh) {
                    for rx in bx..(bx + bw) {
                        if rng.gen_bool(0.12) {
                            continue;
                        }
                        set(&mut cells, rx, ry, Cell::Wall);
                    }
                }
            }
        }
        Scene::Park => {
            if rng.gen_bool(0.35) {
                let bw = rng.gen_range(3..5);
                let bh = rng.gen_range(3..4);
                let bx = rng.gen_range(3..(CHUNK - 3 - bw).max(4));
                let by = rng.gen_range(3..(CHUNK - 3 - bh).max(4));
                for ry in by..(by + bh) {
                    for rx in bx..(bx + bw) {
                        if rng.gen_bool(0.15) {
                            continue;
                        }
                        set(&mut cells, rx, ry, Cell::Wall);
                    }
                }
            }
        }
        Scene::Neighborhood => {
            // A house: a wall ring with a door on the front (bottom) wall.
            if rng.gen_bool(0.85) {
                let bw = rng.gen_range(7..10);
                let bh = rng.gen_range(6..8);
                let bx = rng.gen_range(2..(CHUNK - 2 - bw).max(3));
                let by = rng.gen_range(2..(CHUNK - 2 - bh).max(3));
                for rx in bx..(bx + bw) {
                    set(&mut cells, rx, by, Cell::Wall);
                    set(&mut cells, rx, by + bh - 1, Cell::Wall);
                }
                for ry in by..(by + bh) {
                    set(&mut cells, bx, ry, Cell::Wall);
                    set(&mut cells, bx + bw - 1, ry, Cell::Wall);
                }
                let door = bx + 2 + rng.gen_range(0..(bw - 4).max(1));
                set(&mut cells, door, by + bh - 1, Cell::Floor);
            }
        }
    }

    // Keep the spawn area clear (only affects the spawn chunk).
    let (sc, sr) = ((spawn.x / TILE).floor() as i32, (-spawn.y / TILE).floor() as i32);
    for gr in (sr - 3)..=(sr + 3) {
        for gc in (sc - 3)..=(sc + 3) {
            if gc.div_euclid(CHUNK) == cx && gr.div_euclid(CHUNK) == cy {
                set(&mut cells, gc.rem_euclid(CHUNK), gr.rem_euclid(CHUNK), Cell::Floor);
            }
        }
    }

    // Scatter props onto open floor.
    let kinds: &[PropKind] = match scene {
        Scene::Park => &[
            PropKind::Tree, PropKind::Tree, PropKind::Bush, PropKind::Bush,
            PropKind::Bench, PropKind::Barrel, PropKind::Hydrant,
        ],
        Scene::Neighborhood => &[
            PropKind::Car, PropKind::Car, PropKind::Van, PropKind::Bush, PropKind::Tree,
            PropKind::Bench, PropKind::Sofa, PropKind::Table, PropKind::Crate,
            PropKind::Dumpster, PropKind::Hydrant,
        ],
        Scene::Streets => &[
            PropKind::Tree, PropKind::Bush, PropKind::Car, PropKind::Van, PropKind::Bench,
            PropKind::Dumpster, PropKind::Barrel, PropKind::Crate, PropKind::Hydrant,
            PropKind::Table, PropKind::Sofa,
        ],
    };
    let want = match scene {
        Scene::Park => 9,
        Scene::Neighborhood => 6,
        Scene::Streets => 6,
    };
    let solid_at = |cells: &Vec<Cell>, lx: i32, ly: i32| -> bool {
        lx < 0 || ly < 0 || lx >= CHUNK || ly >= CHUNK
            || cells[ly as usize * cu + lx as usize] == Cell::Wall
    };
    let mut props: Vec<Prop> = Vec::new();
    let mut tries = 0;
    while props.len() < want && tries < 120 {
        tries += 1;
        let lx = rng.gen_range(1..CHUNK - 1);
        let ly = rng.gen_range(1..CHUNK - 1);
        let gc = cx * CHUNK + lx;
        let gr = cy * CHUNK + ly;
        let pos = Vec2::new(gc as f32 * TILE + TILE * 0.5, -(gr as f32 * TILE + TILE * 0.5));
        if pos.distance(spawn) < 130.0 {
            continue;
        }
        let kind = kinds[rng.gen_range(0..kinds.len())];
        let (pr, solid) = prop_spec(kind);
        let ok = if pr > 16.0 {
            let mut clear = true;
            'chk: for dy in -1..=1 {
                for dx in -1..=1 {
                    if solid_at(&cells, lx + dx, ly + dy) {
                        clear = false;
                        break 'chk;
                    }
                }
            }
            clear
        } else {
            !solid_at(&cells, lx, ly)
        };
        if !ok {
            continue;
        }
        if props.iter().any(|p| p.pos.distance(pos) < p.r + pr + 14.0) {
            continue;
        }
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        props.push(Prop { kind, pos, r: pr, solid, angle });
    }
    (cells, props)
}

/// Bake a chunk's ground into one small texture (one sprite per chunk floor).
fn chunk_floor_image(images: &mut Assets<Image>, scene: Scene) -> Handle<Image> {
    let px = 4u32; // pixels per tile
    let side = CHUNK as u32 * px;
    let mut data = vec![0u8; (side * side * 4) as usize];
    let mut rng = rand::thread_rng();
    let grass = scene == Scene::Park;
    for ly in 0..CHUNK as u32 {
        for lx in 0..CHUNK as u32 {
            let j = rng.gen_range(-0.02f32..0.02);
            let warm = rng.gen_range(-0.008f32..0.012);
            let (rr, gg, bb) = if grass {
                (0.13 + j + warm, 0.24 + j, 0.13 + j)
            } else {
                let base = 0.27 + j;
                (base + warm, base, base + 0.015)
            };
            let br = (rr.clamp(0.0, 1.0) * 255.0) as u8;
            let bg = (gg.clamp(0.0, 1.0) * 255.0) as u8;
            let bl = (bb.clamp(0.0, 1.0) * 255.0) as u8;
            for dy in 0..px {
                for dx in 0..px {
                    let seam = dx == 0 || dy == 0; // faint grid line
                    let (cr, cg, cb) = if seam {
                        ((br as f32 * 0.8) as u8, (bg as f32 * 0.8) as u8, (bl as f32 * 0.8) as u8)
                    } else {
                        (br, bg, bl)
                    };
                    let x = lx * px + dx;
                    let y = ly * px + dy;
                    let i = ((y * side + x) * 4) as usize;
                    data[i] = cr;
                    data[i + 1] = cg;
                    data[i + 2] = cb;
                    data[i + 3] = 255;
                }
            }
        }
    }
    images.add(Image::new(
        Extent3d { width: side, height: side, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    ))
}

/// Spawn the sprites for one wall tile (shadow, body, top lip, bottom edge).
fn spawn_wall_tile(commands: &mut Commands, soft: &Handle<Image>, center: Vec2) -> Vec<Entity> {
    let mut rng = rand::thread_rng();
    let shade = rng.gen_range(-0.02..0.02);
    let wz = depth_z(Z_PROP, center.y);
    vec![
        commands
            .spawn((
                Sprite {
                    image: soft.clone(),
                    color: Color::srgba(0.0, 0.0, 0.0, 0.42),
                    custom_size: Some(Vec2::new(TILE * 1.25, TILE * 0.85)),
                    ..default()
                },
                Transform::from_xyz(center.x + 5.0, center.y - TILE * 0.42, Z_DECAL + 5.0),
                WorldTile,
            ))
            .id(),
        commands
            .spawn((
                Sprite::from_color(Color::srgb(0.17 + shade, 0.18 + shade, 0.22 + shade), Vec2::splat(TILE)),
                Transform::from_xyz(center.x, center.y, wz),
                WorldTile,
            ))
            .id(),
        commands
            .spawn((
                Sprite::from_color(Color::srgb(0.30, 0.31, 0.37), Vec2::new(TILE, 7.0)),
                Transform::from_xyz(center.x, center.y + TILE * 0.5 - 3.5, wz + 0.05),
                WorldTile,
            ))
            .id(),
        commands
            .spawn((
                Sprite::from_color(Color::srgb(0.10, 0.10, 0.13), Vec2::new(TILE, 4.0)),
                Transform::from_xyz(center.x, center.y - TILE * 0.5 + 2.0, wz + 0.05),
                WorldTile,
            ))
            .id(),
    ]
}

/// Generate + spawn one chunk and record it in the world.
pub fn load_chunk(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    art: &crate::art::Art,
    world: &mut World,
    cx: i32,
    cy: i32,
) {
    if world.chunks.contains_key(&(cx, cy)) {
        return;
    }
    let (cells, props) = gen_chunk(cx, cy, world.seed, world.scene, world.spawn);
    let mut ents: Vec<Entity> = Vec::new();
    // Floor (one textured sprite covering the whole chunk).
    let img = chunk_floor_image(images, world.scene);
    let span = CHUNK as f32 * TILE;
    let center = Vec2::new(cx as f32 * span + span * 0.5, -(cy as f32 * span + span * 0.5));
    ents.push(
        commands
            .spawn((
                Sprite {
                    image: img,
                    custom_size: Some(Vec2::splat(span)),
                    ..default()
                },
                Transform::from_xyz(center.x, center.y, Z_FLOOR),
                WorldTile,
            ))
            .id(),
    );
    // Walls.
    let cu = CHUNK as usize;
    for ly in 0..CHUNK {
        for lx in 0..CHUNK {
            if cells[ly as usize * cu + lx as usize] == Cell::Wall {
                let gc = cx * CHUNK + lx;
                let gr = cy * CHUNK + ly;
                let c = Vec2::new(gc as f32 * TILE + TILE * 0.5, -(gr as f32 * TILE + TILE * 0.5));
                ents.extend(spawn_wall_tile(commands, &art.soft, c));
            }
        }
    }
    // Props.
    for prop in &props {
        ents.extend(spawn_prop_entity(commands, art, *prop));
    }
    world.chunks.insert((cx, cy), Chunk { cells, props, ents });
}

fn unload_chunk(commands: &mut Commands, world: &mut World, key: (i32, i32)) {
    if let Some(ch) = world.chunks.remove(&key) {
        for e in ch.ents {
            commands.entity(e).try_despawn();
        }
    }
}

/// Stream chunks in around the player and drop far ones each frame.
pub fn stream_chunks(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    art: Res<crate::art::Art>,
    mut world: ResMut<World>,
    player_q: Query<&Transform, With<Player>>,
) {
    let Ok(ptf) = player_q.single() else {
        return;
    };
    let (pc, pr) = world.world_to_tile(ptf.translation.truncate());
    let (pcx, pcy) = (pc.div_euclid(CHUNK), pr.div_euclid(CHUNK));
    // Unload anything outside the window.
    let far: Vec<(i32, i32)> = world
        .chunks
        .keys()
        .copied()
        .filter(|(cx, cy)| (cx - pcx).abs() > CHUNK_R || (cy - pcy).abs() > CHUNK_R)
        .collect();
    for key in far {
        unload_chunk(&mut commands, &mut world, key);
    }
    // Load a couple of missing chunks per frame (amortised).
    let mut budget = 2;
    for cy in (pcy - CHUNK_R)..=(pcy + CHUNK_R) {
        for cx in (pcx - CHUNK_R)..=(pcx + CHUNK_R) {
            if budget <= 0 {
                return;
            }
            if !world.chunks.contains_key(&(cx, cy)) {
                load_chunk(&mut commands, &mut images, &art, &mut world, cx, cy);
                budget -= 1;
            }
        }
    }
}

#[derive(Component)]
pub struct WorldTile;

// --- Prop art helpers: spawn a single local-space sprite child, return its id. ---
fn pr_rect(commands: &mut Commands, c: Color, w: f32, h: f32, x: f32, y: f32, z: f32) -> Entity {
    commands
        .spawn((
            Sprite::from_color(c, Vec2::new(w, h)),
            Transform::from_xyz(x, y, z),
        ))
        .id()
}
fn pr_round(commands: &mut Commands, art: &crate::art::Art, c: Color, w: f32, h: f32, x: f32, y: f32, z: f32) -> Entity {
    commands
        .spawn((
            Sprite {
                image: art.circle.clone(),
                color: c,
                custom_size: Some(Vec2::new(w, h)),
                ..default()
            },
            Transform::from_xyz(x, y, z),
        ))
        .id()
}
fn pr_soft(commands: &mut Commands, art: &crate::art::Art, c: Color, w: f32, h: f32, x: f32, y: f32, z: f32) -> Entity {
    commands
        .spawn((
            Sprite {
                image: art.rounded.clone(),
                color: c,
                custom_size: Some(Vec2::new(w, h)),
                ..default()
            },
            Transform::from_xyz(x, y, z),
        ))
        .id()
}

/// Draw all of the world's props (trees, bushes, vehicles, furniture). Each prop
/// is a rotated root with layered child sprites, plus a soft cast shadow.
/// Spawn the visuals for a single prop (shadow + rotated root with children +
/// `PropObj`) and return the top-level entities so a chunk can despawn them.
pub fn spawn_prop_entity(commands: &mut Commands, art: &crate::art::Art, prop: Prop) -> Vec<Entity> {
    let mut rng = rand::thread_rng();
    let mut ents: Vec<Entity> = Vec::new();
    {
        let pos = prop.pos;
        let z = depth_z(Z_PROP, pos.y);
        // Soft cast shadow on the ground.
        let (sw, sh) = match prop.kind {
            PropKind::Car => (58.0, 30.0),
            PropKind::Van => (66.0, 34.0),
            PropKind::Tree => (44.0, 40.0),
            _ => (prop.r * 2.6, prop.r * 2.0),
        };
        ents.push(
            commands
                .spawn((
                    Sprite {
                        image: art.soft.clone(),
                        color: Color::srgba(0.0, 0.0, 0.0, 0.4),
                        custom_size: Some(Vec2::new(sw, sh)),
                        ..default()
                    },
                    Transform::from_xyz(pos.x + 5.0, pos.y - prop.r * 0.5, Z_DECAL + 4.0),
                    WorldTile,
                ))
                .id(),
        );
        let (hp, flammable, explodes) = prop_stats(prop.kind);
        let root = commands
            .spawn((
                Transform::from_xyz(pos.x, pos.y, z)
                    .with_rotation(Quat::from_rotation_z(prop.angle)),
                Visibility::default(),
                WorldTile,
                PropObj {
                    kind: prop.kind,
                    hp,
                    max_hp: hp,
                    r: prop.r,
                    flammable,
                    explodes,
                    burning: 0.0,
                    burn_fx: 0.0,
                    wrecked: false,
                },
            ))
            .id();
        let mut parts: Vec<Entity> = Vec::new();
        match prop.kind {
            PropKind::Tree => {
                // Trunk + layered canopy blobs.
                let trunk = Color::srgb(0.28, 0.19, 0.11);
                parts.push(pr_rect(commands, trunk, 10.0, 10.0, 0.0, 0.0, 0.1));
                let g1 = Color::srgb(0.12, 0.28, 0.13);
                let g2 = Color::srgb(0.16, 0.35, 0.17);
                parts.push(pr_round(commands, art, g1, 56.0, 56.0, 0.0, 0.0, 0.3));
                for _ in 0..5 {
                    let a = rng.gen_range(0.0..std::f32::consts::TAU);
                    let d = rng.gen_range(8.0..16.0);
                    parts.push(pr_round(
                        commands,
                        art,
                        if rng.gen_bool(0.5) { g1 } else { g2 },
                        rng.gen_range(22.0..32.0),
                        rng.gen_range(22.0..32.0),
                        a.cos() * d,
                        a.sin() * d,
                        0.35 + rng.gen_range(0.0..0.1),
                    ));
                }
            }
            PropKind::Bush => {
                let g1 = Color::srgb(0.14, 0.30, 0.15);
                let g2 = Color::srgb(0.18, 0.36, 0.18);
                for _ in 0..5 {
                    let a = rng.gen_range(0.0..std::f32::consts::TAU);
                    let d = rng.gen_range(0.0..9.0);
                    parts.push(pr_round(
                        commands,
                        art,
                        if rng.gen_bool(0.5) { g1 } else { g2 },
                        rng.gen_range(12.0..20.0),
                        rng.gen_range(12.0..20.0),
                        a.cos() * d,
                        a.sin() * d,
                        0.2 + rng.gen_range(0.0..0.1),
                    ));
                }
            }
            PropKind::Car | PropKind::Van => {
                let van = prop.kind == PropKind::Van;
                let len = if van { 78.0 } else { 64.0 };
                let wid = if van { 32.0 } else { 28.0 };
                let bodies = [
                    Color::srgb(0.5, 0.15, 0.14),
                    Color::srgb(0.15, 0.28, 0.45),
                    Color::srgb(0.2, 0.22, 0.25),
                    Color::srgb(0.55, 0.5, 0.2),
                    Color::srgb(0.3, 0.35, 0.32),
                ];
                let body = bodies[rng.gen_range(0..bodies.len())];
                let dark = {
                    let s = body.to_srgba();
                    Color::srgb(s.red * 0.6, s.green * 0.6, s.blue * 0.6)
                };
                // Wheels poking out under the body.
                let blk = Color::srgb(0.05, 0.05, 0.06);
                let wx = len * 0.3;
                let wy = wid * 0.5 + 1.0;
                for (sx, sy) in [(wx, wy), (wx, -wy), (-wx, wy), (-wx, -wy)] {
                    parts.push(pr_rect(commands, blk, 9.0, 5.0, sx, sy, 0.1));
                }
                // Body.
                parts.push(pr_soft(commands, art, body, len, wid, 0.0, 0.0, 0.3));
                // Cabin / roof.
                let cab_len = if van { len * 0.5 } else { len * 0.42 };
                parts.push(pr_soft(commands, art, dark, cab_len, wid * 0.82, if van { -len * 0.12 } else { -2.0 }, 0.0, 0.34));
                // Windows (dark glass).
                let glass = Color::srgb(0.1, 0.13, 0.17);
                parts.push(pr_rect(commands, glass, cab_len * 0.44, wid * 0.66, cab_len * 0.28 - if van { len * 0.12 } else { 2.0 }, 0.0, 0.36));
                if !van {
                    parts.push(pr_rect(commands, glass, cab_len * 0.34, wid * 0.66, -cab_len * 0.34 - 2.0, 0.0, 0.36));
                }
                // Headlights at the front (local +X).
                let lit = Color::srgb(0.9, 0.88, 0.6);
                parts.push(pr_rect(commands, lit, 2.5, 3.0, len * 0.5 - 1.5, wid * 0.3, 0.35));
                parts.push(pr_rect(commands, lit, 2.5, 3.0, len * 0.5 - 1.5, -wid * 0.3, 0.35));
            }
            PropKind::Bench => {
                let wood = Color::srgb(0.35, 0.22, 0.12);
                let metal = Color::srgb(0.12, 0.12, 0.14);
                parts.push(pr_rect(commands, metal, 38.0, 15.0, 0.0, 0.0, 0.2));
                for i in 0..3 {
                    parts.push(pr_rect(commands, wood, 36.0, 3.2, 0.0, -5.0 + i as f32 * 5.0, 0.3));
                }
            }
            PropKind::Dumpster => {
                let body = Color::srgb(0.15, 0.32, 0.2);
                let lid = Color::srgb(0.1, 0.22, 0.14);
                parts.push(pr_rect(commands, body, 38.0, 28.0, 0.0, 0.0, 0.2));
                parts.push(pr_rect(commands, lid, 40.0, 10.0, 0.0, 9.0, 0.3));
                parts.push(pr_rect(commands, Color::srgb(0.08, 0.16, 0.1), 38.0, 2.5, 0.0, -3.0, 0.31));
            }
            PropKind::Barrel => {
                let rust = Color::srgb(0.4, 0.3, 0.16);
                parts.push(pr_round(commands, art, rust, 20.0, 20.0, 0.0, 0.0, 0.2));
                parts.push(pr_round(commands, art, Color::srgb(0.5, 0.4, 0.22), 13.0, 13.0, 0.0, 0.0, 0.25));
                parts.push(pr_rect(commands, Color::srgb(0.2, 0.15, 0.08), 20.0, 2.4, 0.0, 3.5, 0.26));
            }
            PropKind::Crate => {
                let wood = Color::srgb(0.42, 0.3, 0.16);
                let edge = Color::srgb(0.3, 0.21, 0.11);
                parts.push(pr_rect(commands, wood, 26.0, 26.0, 0.0, 0.0, 0.2));
                parts.push(pr_rect(commands, edge, 26.0, 3.4, 0.0, 9.5, 0.25));
                parts.push(pr_rect(commands, edge, 26.0, 3.4, 0.0, -9.5, 0.25));
                parts.push(pr_rect(commands, edge, 3.4, 26.0, 0.0, 0.0, 0.25));
            }
            PropKind::Hydrant => {
                let red = Color::srgb(0.7, 0.12, 0.1);
                parts.push(pr_round(commands, art, red, 11.0, 12.0, 0.0, 0.0, 0.2));
                parts.push(pr_rect(commands, Color::srgb(0.5, 0.08, 0.07), 5.0, 3.0, 6.0, 0.0, 0.25));
                parts.push(pr_round(commands, art, Color::srgb(0.8, 0.2, 0.15), 5.0, 5.0, 0.0, 6.0, 0.26));
            }
            PropKind::Table => {
                let wood = Color::srgb(0.36, 0.24, 0.14);
                parts.push(pr_round(commands, art, Color::srgb(0.2, 0.14, 0.08), 32.0, 32.0, 0.0, 0.0, 0.2));
                parts.push(pr_round(commands, art, wood, 27.0, 27.0, 0.0, 0.0, 0.25));
            }
            PropKind::Sofa => {
                let fab = Color::srgb(0.3, 0.26, 0.35);
                let fab2 = Color::srgb(0.36, 0.31, 0.42);
                parts.push(pr_soft(commands, art, fab, 38.0, 27.0, 0.0, 0.0, 0.2));
                // Back + arms.
                parts.push(pr_rect(commands, fab2, 38.0, 7.0, 0.0, -10.0, 0.26));
                parts.push(pr_rect(commands, fab2, 7.0, 27.0, -15.0, 0.0, 0.26));
                parts.push(pr_rect(commands, fab2, 7.0, 27.0, 15.0, 0.0, 0.26));
                // Cushions.
                parts.push(pr_soft(commands, art, fab2, 14.0, 15.0, -7.5, 2.0, 0.28));
                parts.push(pr_soft(commands, art, fab2, 14.0, 15.0, 7.5, 2.0, 0.28));
            }
        }
        commands.entity(root).add_children(&parts);
        ents.push(root);
    }
    ents
}

/// Drive damaged props: burning ones throw flame + smoke and burn down; when a
/// prop's hp hits zero it's wrecked — tipped over, charred, left smoldering — and
/// explosive props (cars, barrels) detonate.
pub fn prop_system(
    time: Res<Time>,
    mut commands: Commands,
    mut explosions: EventWriter<crate::combat::Explosion>,
    mut q: Query<(Entity, &mut PropObj, &mut Transform)>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (e, mut p, mut tf) in q.iter_mut() {
        if p.burning > 0.0 {
            p.burning -= dt;
            if !p.wrecked {
                p.hp -= 9.0 * dt;
            }
            p.burn_fx -= dt;
            if p.burn_fx <= 0.0 {
                p.burn_fx = rng.gen_range(0.05..0.12);
                let pos = tf.translation.truncate();
                let spread = p.r * 0.7;
                let off = Vec2::new(rng.gen_range(-spread..spread), rng.gen_range(-spread..spread));
                let hot = if rng.gen_bool(0.5) {
                    Color::srgb(1.0, 0.6, 0.15)
                } else {
                    Color::srgb(1.0, 0.85, 0.3)
                };
                commands.spawn((
                    Sprite::from_color(hot, Vec2::splat(rng.gen_range(4.0..8.0))),
                    Transform::from_xyz(pos.x + off.x, pos.y + off.y, Z_FX - 2.0),
                    crate::combat::Particle {
                        vel: Vec2::new(rng.gen_range(-12.0..12.0), 34.0 + rng.gen_range(0.0..46.0)),
                        life: rng.gen_range(0.3..0.6),
                        max_life: 0.6,
                        drag: 0.9,
                        gravity: 0.0,
                        base: hot,
                    },
                ));
                if rng.gen_bool(0.5) {
                    let g = rng.gen_range(0.1..0.2);
                    let smoke = Color::srgba(g, g, g, 0.6);
                    commands.spawn((
                        Sprite::from_color(smoke, Vec2::splat(rng.gen_range(6.0..12.0))),
                        Transform::from_xyz(pos.x + off.x, pos.y + off.y, Z_FX - 1.0),
                        crate::combat::Particle {
                            vel: Vec2::new(rng.gen_range(-10.0..10.0), 40.0 + rng.gen_range(0.0..40.0)),
                            life: rng.gen_range(0.6..1.3),
                            max_life: 1.3,
                            drag: 0.93,
                            gravity: 0.0,
                            base: smoke,
                        },
                    ));
                }
            }
        }
        if p.hp <= 0.0 && !p.wrecked {
            p.wrecked = true;
            let pos = tf.translation.truncate();
            let sign = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
            // Tipped/flipped over.
            tf.rotation *= Quat::from_rotation_z(sign * rng.gen_range(0.5..1.2));
            // Charred overlay across the whole prop.
            let over = commands
                .spawn((
                    Sprite {
                        color: Color::srgba(0.05, 0.04, 0.04, 0.72),
                        custom_size: Some(Vec2::splat(p.r * 2.3)),
                        ..default()
                    },
                    Transform::from_xyz(0.0, 0.0, 0.5),
                ))
                .id();
            commands.entity(e).add_child(over);
            p.burning = p.burning.max(2.5); // smolder a while longer
            if p.explodes {
                explosions.write(crate::combat::Explosion {
                    pos,
                    radius: 72.0,
                    damage: 85.0,
                    knockback: 250.0,
                    sever: 0.4,
                });
            }
        }
    }
}
