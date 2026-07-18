use crate::common::*;
use bevy::prelude::*;
use rand::Rng;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Floor,
    Wall,
}

#[derive(Resource)]
pub struct World {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<Cell>,
    pub spawn: Vec2,
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

    /// Is this world point inside a solid tile? (used by projectiles)
    pub fn blocks_point(&self, p: Vec2) -> bool {
        let (c, r) = self.world_to_tile(p);
        self.solid(c, r)
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
        p
    }
}

/// Build a "streets" arena: solid border plus scattered building blocks for cover.
pub fn generate_world() -> World {
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
    // Scatter rectangular buildings/crates, keeping a clear zone at the center spawn.
    let mut rng = rand::thread_rng();
    let center = (cols / 2, rows / 2);
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
                // leave a doorway gap
                if rng.gen_bool(0.12) {
                    continue;
                }
                set(&mut cells, c, r, Cell::Wall);
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
    World { cols, rows, cells, spawn }
}

#[derive(Component)]
pub struct WorldTile;

/// Spawn floor + wall sprites once when the game starts.
pub fn spawn_world_sprites(commands: &mut Commands, world: &World) {
    let mut rng = rand::thread_rng();
    for r in 0..world.rows {
        for c in 0..world.cols {
            let center = world.tile_center(c, r);
            match world.cells[r * world.cols + c] {
                Cell::Floor => {
                    // Asphalt with per-tile jitter, brighter than pitch black so
                    // the scene reads as a lit street.
                    let j = rng.gen_range(-0.02..0.02);
                    let warm = rng.gen_range(-0.008..0.012);
                    let base = 0.27 + j;
                    let col = Color::srgb(base + warm, base, base + 0.015);
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
                    // Soft cast shadow on the ground below the wall for grounding.
                    commands.spawn((
                        Sprite::from_color(
                            Color::srgba(0.0, 0.0, 0.0, 0.35),
                            Vec2::new(TILE, TILE * 0.5),
                        ),
                        Transform::from_xyz(center.x + 4.0, center.y - TILE * 0.5, Z_DECAL + 5.0),
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
