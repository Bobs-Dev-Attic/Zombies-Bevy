use crate::common::*;
use bevy::prelude::*;
use rand::Rng;

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

#[derive(Resource)]
pub struct World {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<Cell>,
    pub spawn: Vec2,
    pub props: Vec<Prop>,
    pub scene: Scene,
}

impl World {
    pub fn at(&self, c: isize, r: usize) -> Cell {
        if c < 0 || r >= self.rows || c as usize >= self.cols {
            return Cell::Wall;
        }
        self.cells[r * self.cols + c as usize]
    }
    pub fn solid(&self, c: isize, r: isize) -> bool {
        if c < 0 || r < 0 || c as usize >= self.cols || r as usize >= self.rows {
            return true;
        }
        self.cells[r as usize * self.cols + c as usize] == Cell::Wall
    }

    /// Center of a tile in world coordinates.
    pub fn tile_center(&self, c: usize, r: usize) -> Vec2 {
        Vec2::new(c as f32 * TILE + TILE * 0.5, -(r as f32 * TILE + TILE * 0.5))
    }

    pub fn world_to_tile(&self, p: Vec2) -> (isize, isize) {
        ((p.x / TILE).floor() as isize, (-p.y / TILE).floor() as isize)
    }

    /// Is this world point inside a solid tile or a solid prop? (used by
    /// projectiles, flames, casings and rolling grenades)
    pub fn blocks_point(&self, p: Vec2) -> bool {
        let (c, r) = self.world_to_tile(p);
        if self.solid(c, r) {
            return true;
        }
        self.props
            .iter()
            .any(|pr| pr.solid && p.distance(pr.pos) < pr.r)
    }

    /// Push a circle out of any solid tiles it overlaps. Returns resolved center.
    pub fn collide(&self, mut p: Vec2, radius: f32) -> Vec2 {
        let (cc, cr) = self.world_to_tile(p);
        for r in (cr - 1)..=(cr + 1) {
            for c in (cc - 1)..=(cc + 1) {
                if !self.solid(c, r) {
                    continue;
                }
                // Tile AABB.
                let min = Vec2::new(c as f32 * TILE, -((r as f32 + 1.0) * TILE));
                let max = Vec2::new((c as f32 + 1.0) * TILE, -(r as f32 * TILE));
                let closest = p.clamp(min, max);
                let delta = p - closest;
                let d = delta.length();
                if d < radius {
                    if d > 0.0001 {
                        p += delta / d * (radius - d);
                    } else {
                        // Center is inside the tile; push out along the shallowest axis.
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
        // Push out of any solid props (circle-vs-circle).
        for prop in &self.props {
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
pub fn generate_world(choice: Option<Scene>) -> World {
    let cols = 46usize;
    let rows = 34usize;
    let mut cells = vec![Cell::Floor; cols * rows];
    let mut set = |cells: &mut Vec<Cell>, c: usize, r: usize, v: Cell| {
        if c < cols && r < rows {
            cells[r * cols + c] = v;
        }
    };
    // Border walls.
    for c in 0..cols {
        set(&mut cells, c, 0, Cell::Wall);
        set(&mut cells, c, rows - 1, Cell::Wall);
    }
    for r in 0..rows {
        set(&mut cells, 0, r, Cell::Wall);
        set(&mut cells, cols - 1, r, Cell::Wall);
    }
    let mut rng = rand::thread_rng();
    let center = (cols / 2, rows / 2);
    let scene = choice.unwrap_or_else(|| {
        *[Scene::Streets, Scene::Park, Scene::Neighborhood]
            .get(rng.gen_range(0..3))
            .unwrap()
    });

    match scene {
        Scene::Streets => {
            // Scattered rectangular buildings/crates with occasional doorway gaps.
            let blocks: [(usize, usize, usize, usize); 10] = [
                (5, 4, 6, 4),
                (34, 5, 6, 5),
                (6, 24, 7, 5),
                (33, 25, 7, 4),
                (20, 3, 5, 3),
                (19, 27, 6, 3),
                (3, 14, 3, 6),
                (40, 15, 3, 6),
                (14, 12, 4, 3),
                (28, 18, 4, 3),
            ];
            for (bx, by, bw, bh) in blocks {
                for r in by..(by + bh) {
                    for c in bx..(bx + bw) {
                        if rng.gen_bool(0.12) {
                            continue;
                        }
                        set(&mut cells, c, r, Cell::Wall);
                    }
                }
            }
        }
        Scene::Park => {
            // Mostly open green space with a couple of small sheds/restrooms for
            // cover — the trees and bushes (props) do the rest.
            let sheds: [(usize, usize, usize, usize); 3] = [
                (7, 6, 4, 3),
                (34, 7, 4, 3),
                (20, 26, 5, 3),
            ];
            for (bx, by, bw, bh) in sheds {
                for r in by..(by + bh) {
                    for c in bx..(bx + bw) {
                        if rng.gen_bool(0.15) {
                            continue;
                        }
                        set(&mut cells, c, r, Cell::Wall);
                    }
                }
            }
        }
        Scene::Neighborhood => {
            // A loose grid of houses: hollow wall rings with a door gap, streets
            // running between them.
            let houses: [(usize, usize, usize, usize); 6] = [
                (4, 4, 8, 6),
                (18, 4, 8, 6),
                (34, 5, 8, 6),
                (5, 22, 8, 6),
                (20, 24, 8, 6),
                (34, 22, 8, 6),
            ];
            for (bx, by, bw, bh) in houses {
                // Walls around the perimeter of the house.
                for c in bx..(bx + bw) {
                    set(&mut cells, c, by, Cell::Wall);
                    set(&mut cells, c, by + bh - 1, Cell::Wall);
                }
                for r in by..(by + bh) {
                    set(&mut cells, bx, r, Cell::Wall);
                    set(&mut cells, bx + bw - 1, r, Cell::Wall);
                }
                // Punch a door gap on the front (bottom) wall.
                let door = bx + 2 + rng.gen_range(0..bw.saturating_sub(4).max(1));
                set(&mut cells, door, by + bh - 1, Cell::Floor);
            }
        }
    }

    // Guarantee the spawn area is clear.
    for r in (center.1 - 2)..=(center.1 + 2) {
        for c in (center.0 - 2)..=(center.0 + 2) {
            set(&mut cells, c, r, Cell::Floor);
        }
    }
    let spawn = Vec2::new(
        center.0 as f32 * TILE + TILE * 0.5,
        -(center.1 as f32 * TILE + TILE * 0.5),
    );
    let mut world = World { cols, rows, cells, spawn, props: Vec::new(), scene };

    // Scatter scenery, weighted to fit the scene — a park is mostly trees and
    // bushes, a neighborhood has cars and furniture, downtown a mix.
    let kinds: &[PropKind] = match scene {
        Scene::Park => &[
            PropKind::Tree,
            PropKind::Tree,
            PropKind::Tree,
            PropKind::Bush,
            PropKind::Bush,
            PropKind::Bush,
            PropKind::Bench,
            PropKind::Bench,
            PropKind::Barrel,
            PropKind::Hydrant,
        ],
        Scene::Neighborhood => &[
            PropKind::Car,
            PropKind::Car,
            PropKind::Van,
            PropKind::Bush,
            PropKind::Bush,
            PropKind::Tree,
            PropKind::Bench,
            PropKind::Sofa,
            PropKind::Table,
            PropKind::Crate,
            PropKind::Dumpster,
            PropKind::Hydrant,
        ],
        Scene::Streets => &[
            PropKind::Tree,
            PropKind::Bush,
            PropKind::Bush,
            PropKind::Car,
            PropKind::Van,
            PropKind::Bench,
            PropKind::Dumpster,
            PropKind::Barrel,
            PropKind::Crate,
            PropKind::Hydrant,
            PropKind::Table,
            PropKind::Sofa,
        ],
    };
    let want = match scene {
        Scene::Park => 46,
        Scene::Neighborhood => 40,
        Scene::Streets => 36,
    };
    let mut attempts = 0;
    while world.props.len() < want && attempts < 1400 {
        attempts += 1;
        let c = rng.gen_range(2..cols - 2);
        let r = rng.gen_range(2..rows - 2);
        let pos = world.tile_center(c, r);
        if pos.distance(spawn) < 150.0 {
            continue;
        }
        let kind = kinds[rng.gen_range(0..kinds.len())];
        let (pr, solid) = prop_spec(kind);
        // Big props need a clear 3x3; small furniture only needs its own tile
        // clear (so a sofa or table can sit inside a house room).
        let ok = if pr > 16.0 {
            let mut clear = true;
            'chk: for dr in -1..=1 {
                for dc in -1..=1 {
                    if world.solid(c as isize + dc, r as isize + dr) {
                        clear = false;
                        break 'chk;
                    }
                }
            }
            clear
        } else {
            !world.solid(c as isize, r as isize)
        };
        if !ok {
            continue;
        }
        // Don't overlap another prop.
        if world
            .props
            .iter()
            .any(|p| p.pos.distance(pos) < p.r + pr + 14.0)
        {
            continue;
        }
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        world.props.push(Prop { kind, pos, r: pr, solid, angle });
    }

    world
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
pub fn spawn_props(commands: &mut Commands, world: &World, art: &crate::art::Art) {
    let mut rng = rand::thread_rng();
    for prop in &world.props {
        let pos = prop.pos;
        let z = depth_z(Z_PROP, pos.y);
        // Soft cast shadow on the ground.
        let (sw, sh) = match prop.kind {
            PropKind::Car => (58.0, 30.0),
            PropKind::Van => (66.0, 34.0),
            PropKind::Tree => (44.0, 40.0),
            _ => (prop.r * 2.6, prop.r * 2.0),
        };
        commands.spawn((
            Sprite {
                image: art.soft.clone(),
                color: Color::srgba(0.0, 0.0, 0.0, 0.4),
                custom_size: Some(Vec2::new(sw, sh)),
                ..default()
            },
            Transform::from_xyz(pos.x + 5.0, pos.y - prop.r * 0.5, Z_DECAL + 4.0),
            WorldTile,
        ));
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
    }
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

/// Spawn floor + wall sprites once when the game starts. `soft` is the shared
/// radial-gradient texture used for soft cast shadows.
pub fn spawn_world_sprites(commands: &mut Commands, world: &World, soft: &Handle<Image>) {
    let mut rng = rand::thread_rng();
    let grass = world.scene == Scene::Park;
    for r in 0..world.rows {
        for c in 0..world.cols {
            let center = world.tile_center(c, r);
            match world.cells[r * world.cols + c] {
                Cell::Floor => {
                    // Asphalt with per-tile jitter (grass green in the park), a bit
                    // brighter than pitch black so the ground reads as lit.
                    let j = rng.gen_range(-0.02..0.02);
                    let warm = rng.gen_range(-0.008..0.012);
                    let base = 0.27 + j;
                    let col = if grass {
                        Color::srgb(0.13 + j + warm, 0.24 + j, 0.13 + j)
                    } else {
                        Color::srgb(base + warm, base, base + 0.015)
                    };
                    commands.spawn((
                        Sprite::from_color(col, Vec2::splat(TILE)),
                        Transform::from_xyz(center.x, center.y, Z_FLOOR),
                        WorldTile,
                    ));
                    // Faint tile seams (a subtle grid) for texture and scale.
                    let seam = Color::srgb(base - 0.05, base - 0.05, base - 0.045);
                    commands.spawn((
                        Sprite::from_color(seam, Vec2::new(TILE, 1.5)),
                        Transform::from_xyz(center.x, center.y - TILE * 0.5, Z_FLOOR + 0.05),
                        WorldTile,
                    ));
                    commands.spawn((
                        Sprite::from_color(seam, Vec2::new(1.5, TILE)),
                        Transform::from_xyz(center.x - TILE * 0.5, center.y, Z_FLOOR + 0.05),
                        WorldTile,
                    ));
                    // Occasional cracks / gravel speckle for density.
                    if rng.gen_bool(0.16) {
                        let sp = rng.gen_range(2.0..5.0);
                        let d = rng.gen_range(-0.06..0.06);
                        commands.spawn((
                            Sprite::from_color(
                                Color::srgb(base + d, base + d, base + d + 0.01),
                                Vec2::splat(sp),
                            ),
                            Transform::from_xyz(
                                center.x + rng.gen_range(-15.0..15.0),
                                center.y + rng.gen_range(-15.0..15.0),
                                Z_FLOOR + 0.1,
                            ),
                            WorldTile,
                        ));
                    }
                }
                Cell::Wall => {
                    let shade = rng.gen_range(-0.02..0.02);
                    let wz = depth_z(Z_PROP, center.y);
                    // Soft gradient cast shadow on the ground below the wall.
                    commands.spawn((
                        Sprite {
                            image: soft.clone(),
                            color: Color::srgba(0.0, 0.0, 0.0, 0.42),
                            custom_size: Some(Vec2::new(TILE * 1.25, TILE * 0.85)),
                            ..default()
                        },
                        Transform::from_xyz(center.x + 5.0, center.y - TILE * 0.42, Z_DECAL + 5.0),
                        WorldTile,
                    ));
                    // Wall body (a brick/concrete block).
                    commands.spawn((
                        Sprite::from_color(
                            Color::srgb(0.17 + shade, 0.18 + shade, 0.22 + shade),
                            Vec2::splat(TILE),
                        ),
                        Transform::from_xyz(center.x, center.y, wz),
                        WorldTile,
                    ));
                    // Top highlight lip (light catching the top edge).
                    commands.spawn((
                        Sprite::from_color(
                            Color::srgb(0.30, 0.31, 0.37),
                            Vec2::new(TILE, 7.0),
                        ),
                        Transform::from_xyz(center.x, center.y + TILE * 0.5 - 3.5, wz + 0.05),
                        WorldTile,
                    ));
                    // Darker bottom edge for contact shading.
                    commands.spawn((
                        Sprite::from_color(
                            Color::srgb(0.10, 0.10, 0.13),
                            Vec2::new(TILE, 4.0),
                        ),
                        Transform::from_xyz(center.x, center.y - TILE * 0.5 + 2.0, wz + 0.05),
                        WorldTile,
                    ));
                }
            }
        }
    }
}
