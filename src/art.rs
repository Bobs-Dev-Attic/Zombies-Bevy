use crate::common::*;
use crate::enemy::{NewZombieRadius, Zombie};
use crate::player::{BodyGear, HeadGear, Player};
use crate::weapons::WeaponKind;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Player body colours, shared between rig construction and the per-frame
/// hurt-flash tint so the base always restores correctly.
pub const PLAYER_SHIRT: Color = Color::srgb(0.34, 0.40, 0.52); // casual t-shirt
pub const PLAYER_SKIN: Color = Color::srgb(0.82, 0.63, 0.49);

/// Shared generated textures for soft circular shapes.
#[derive(Resource)]
pub struct Art {
    pub circle: Handle<Image>,
    pub soft: Handle<Image>,
    /// White rounded rectangle for softly-cornered body parts.
    pub rounded: Handle<Image>,
    /// Red edge-vignette (transparent centre → opaque edges) for the hurt flash.
    pub vignette: Handle<Image>,
}

/// Marker: this character needs its visual rig built.
#[derive(Component)]
pub struct NeedsRig;

/// A ring of ticks above the player that fills as a reload cycles.
#[derive(Component)]
pub struct ReloadRing {
    pub ticks: Vec<Entity>,
}

/// Toggleable roots for swappable gear, so equipping/breaking gear just flips
/// visibility instead of rebuilding the rig.
#[derive(Component)]
pub struct GearVisuals {
    pub cap_root: Entity,
    pub helmet_root: Entity,
    pub armor_root: Entity,
    pub backpack_root: Entity,
    pub hair: Entity,
}

/// A distinct held model per weapon kind (indexed by `WeaponKind::index`), all
/// children of `Rig::weapon`; only the equipped one is shown. `pistol_slide` and
/// `pistol_mag` are driven during a reload (slide racks, magazine drops).
#[derive(Component)]
pub struct WeaponVisuals {
    pub roots: [Entity; crate::weapons::WEAPON_KINDS],
    pub pistol_slide: Entity,
    pub pistol_mag: Entity,
    pub shotgun_pump: Entity,
    pub rifle_mag: Entity,
    /// The side-by-side's break-action barrel pivot (hinges open on reload).
    pub sxs_barrel: Entity,
    /// A wooden bat/club model (shown for the Bat instead of the knife blade).
    pub bat_model: Entity,
}

/// The player's two forearm (elbow) pivots, so poses can bend the arms — e.g.
/// folding the elbows to bring the hands onto a shotgun's pump and trigger.
#[derive(Component)]
pub struct PlayerArms {
    pub fore_l: Entity,
    pub fore_r: Entity,
}

/// A zombie's articulated joint pivots. Arms and legs are two-segment limbs like
/// the player's: a shoulder/hip pivot (stored on `Rig` as `arm_*`/`leg_*`) drives
/// the upper segment, and these elbow/knee pivots bend the lower segment. Holding
/// each joint separately is also the groundwork for limbs being shot off later.
#[derive(Component)]
pub struct ZombieLimbs {
    pub fore_l: Entity, // left elbow pivot
    pub fore_r: Entity, // right elbow pivot
    pub shin_l: Entity, // left knee pivot
    pub shin_r: Entity, // right knee pivot
    pub tatters: Option<Entity>, // torn clothing dragging behind (swayed each frame)
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
            // Smootherstep falloff → a soft, realistic radial gradient.
            let s = 1.0 - (d * d * (3.0 - 2.0 * d));
            let a = s.powf(1.3);
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

/// A full-frame vignette: transparent in the centre, ramping to opaque toward
/// the edges/corners. White RGB so it can be tinted (red for the hurt flash).
fn make_vignette(images: &mut Assets<Image>, size: u32) -> Handle<Image> {
    let mut data = vec![0u8; (size * size * 4) as usize];
    let c = (size as f32 - 1.0) / 2.0;
    for y in 0..size {
        for x in 0..size {
            let dx = (x as f32 - c) / c;
            let dy = (y as f32 - c) / c;
            let d = (dx * dx + dy * dy).sqrt().clamp(0.0, 1.0);
            // Empty until ~45% out, then ramp up to the edge.
            let t = ((d - 0.45) / 0.55).clamp(0.0, 1.0);
            let a = t * t * (3.0 - 2.0 * t);
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

/// A white rounded rectangle (corner radius = `radius_frac` of the half-size).
fn make_rounded_rect(images: &mut Assets<Image>, size: u32, radius_frac: f32) -> Handle<Image> {
    let mut data = vec![0u8; (size * size * 4) as usize];
    let half = (size as f32 - 1.0) / 2.0;
    let r = radius_frac * half;
    for y in 0..size {
        for x in 0..size {
            let px = x as f32 - half;
            let py = y as f32 - half;
            // Signed distance to a rounded box centred at the origin.
            let qx = px.abs() - (half - r);
            let qy = py.abs() - (half - r);
            let ox = qx.max(0.0);
            let oy = qy.max(0.0);
            let d = (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - r;
            let a = (0.5 - d).clamp(0.0, 1.0);
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
    let soft = make_soft(&mut images, 96);
    let rounded = make_rounded_rect(&mut images, 64, 0.45);
    let vignette = make_vignette(&mut images, 128);
    commands.insert_resource(Art { circle, soft, rounded, vignette });
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

/// A rectangle with softly rounded corners.
fn rrect(art: &Art, color: Color, w: f32, h: f32, z: f32) -> impl Bundle {
    (
        Sprite {
            image: art.rounded.clone(),
            color,
            custom_size: Some(Vec2::new(w, h)),
            ..default()
        },
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
    // Starts casual: t-shirt, bare head, no pack. Helmet/olive/armour parts are
    // still built but hidden until the matching gear is equipped.
    let shirt = PLAYER_SHIRT;
    let shirt_dark = Color::srgb(0.24, 0.28, 0.36);
    let shirt_hi = Color::srgb(0.44, 0.50, 0.62);
    let skin = PLAYER_SKIN;
    let skin_dark = Color::srgb(0.66, 0.47, 0.35);
    let pants = Color::srgb(0.17, 0.18, 0.20);
    let hat = Color::srgb(0.41, 0.39, 0.25);
    let hat_dark = Color::srgb(0.28, 0.27, 0.16);
    let pack = Color::srgb(0.40, 0.38, 0.24);
    let pack_dark = Color::srgb(0.26, 0.25, 0.15);
    let strap = Color::srgb(0.20, 0.19, 0.13);
    let gun = Color::srgb(0.10, 0.10, 0.12);

    // Soft, gradient contact shadow (does not rotate — child of root).
    let shadow = commands
        .spawn((
            Sprite {
                image: art.soft.clone(),
                color: Color::srgba(0.0, 0.0, 0.0, 0.34),
                custom_size: Some(Vec2::new(40.0, 26.0)),
                ..default()
            },
            Transform::from_xyz(1.0, -5.0, -0.5),
        ))
        .id();

    // Body pivot (rotates to face aim).
    let body = commands.spawn((Transform::default(), Visibility::default())).id();

    // ---- Backpack (behind the torso, toward the back = -X). Blocky. ----
    let backpack_root = commands.spawn((Transform::default(), Visibility::default())).id();
    let pack_base = commands.spawn(rect(pack, 17.0, 23.0, -0.35)).id();
    commands.entity(pack_base).insert(Transform::from_xyz(-8.5, 0.0, -0.35));
    let pack_lid = commands.spawn(rect(pack_dark, 12.0, 9.0, -0.34)).id();
    commands.entity(pack_lid).insert(Transform::from_xyz(-10.0, 0.0, -0.34));
    let pack_seam_v = commands.spawn(rect(pack_dark, 1.6, 20.0, -0.33)).id();
    commands.entity(pack_seam_v).insert(Transform::from_xyz(-8.0, 0.0, -0.33));
    let pack_seam_h = commands.spawn(rect(pack_dark, 15.0, 1.6, -0.33)).id();
    commands.entity(pack_seam_h).insert(Transform::from_xyz(-8.5, 6.0, -0.33));
    let buckle_a = commands.spawn(rect(strap, 3.0, 3.0, -0.32)).id();
    commands.entity(buckle_a).insert(Transform::from_xyz(-4.0, 6.5, -0.32));
    let buckle_b = commands.spawn(rect(strap, 3.0, 3.0, -0.32)).id();
    commands.entity(buckle_b).insert(Transform::from_xyz(-4.0, -6.5, -0.32));
    // Shoulder straps running forward over the torso.
    let strap_a = commands.spawn(rect(strap, 20.0, 3.2, 0.06)).id();
    commands.entity(strap_a).insert(Transform::from_xyz(2.0, 6.0, 0.06));
    let strap_b = commands.spawn(rect(strap, 20.0, 3.2, 0.06)).id();
    commands.entity(strap_b).insert(Transform::from_xyz(2.0, -6.0, 0.06));
    commands.entity(backpack_root).add_children(&[
        pack_base, pack_lid, pack_seam_v, pack_seam_h, buckle_a, buckle_b, strap_a, strap_b,
    ]);

    // ---- Legs ----
    let leg_l = commands.spawn(rect(pants, 8.0, 6.0, -0.2)).id();
    let leg_r = commands.spawn(rect(pants, 8.0, 6.0, -0.2)).id();

    // ---- Torso: rectangular body with softly rounded back & shoulders ----
    // The main body block is `torso` (recoloured on hit); the rest are detail.
    let torso = commands.spawn(rrect(art, shirt, 15.0, 16.0, 0.0)).id();
    // Rounded back/upper-back hump (thinner front-to-back).
    let back_block = commands.spawn(rrect(art, shirt_dark, 8.0, 16.0, -0.01)).id();
    commands.entity(back_block).insert(Transform::from_xyz(-4.0, 0.0, -0.01));
    let chest = commands.spawn(rrect(art, shirt_hi, 8.0, 10.0, 0.02)).id();
    commands.entity(chest).insert(Transform::from_xyz(2.0, 0.0, 0.02));
    // Rounded shoulders (set a little narrower).
    let shoulder_l = commands.spawn(rrect(art, shirt, 7.0, 8.0, 0.03)).id();
    commands.entity(shoulder_l).insert(Transform::from_xyz(1.0, 7.5, 0.03));
    let shoulder_r = commands.spawn(rrect(art, shirt, 7.0, 8.0, 0.03)).id();
    commands.entity(shoulder_r).insert(Transform::from_xyz(1.0, -7.5, 0.03));
    // Body-armour plate carrier (toggled on when equipped).
    let armor_root = commands.spawn((Transform::default(), Visibility::Hidden)).id();
    // Match the slimmer torso (thinner front-to-back).
    let vest = commands.spawn(rect(Color::srgb(0.15, 0.16, 0.14), 14.0, 17.0, 0.05)).id();
    let plate = commands.spawn(rect(Color::srgb(0.22, 0.24, 0.20), 9.0, 12.0, 0.06)).id();
    commands.entity(plate).insert(Transform::from_xyz(2.0, 0.0, 0.06));
    let pouch_a = commands.spawn(rect(Color::srgb(0.12, 0.13, 0.11), 4.0, 6.0, 0.07)).id();
    commands.entity(pouch_a).insert(Transform::from_xyz(-2.0, 5.0, 0.07));
    let pouch_b = commands.spawn(rect(Color::srgb(0.12, 0.13, 0.11), 4.0, 6.0, 0.07)).id();
    commands.entity(pouch_b).insert(Transform::from_xyz(-2.0, -5.0, 0.07));
    let a_strap_l = commands.spawn(rect(Color::srgb(0.09, 0.09, 0.09), 5.0, 3.0, 0.07)).id();
    commands.entity(a_strap_l).insert(Transform::from_xyz(4.5, 7.0, 0.07));
    let a_strap_r = commands.spawn(rect(Color::srgb(0.09, 0.09, 0.09), 5.0, 3.0, 0.07)).id();
    commands.entity(a_strap_r).insert(Transform::from_xyz(4.5, -7.0, 0.07));
    commands
        .entity(armor_root)
        .add_children(&[vest, plate, pouch_a, pouch_b, a_strap_l, a_strap_r]);

    commands
        .entity(torso)
        .add_children(&[back_block, chest, shoulder_l, shoulder_r, armor_root]);

    // ---- Arms: big, long, fully bare (skin) two-segment limbs hinged at a
    // rounded elbow, ending in a fist. `bend` angles the forearm inward so both
    // hands meet the gun. Rounded rects so the arms read as muscle, not planks. ----
    let build_arm = |commands: &mut Commands, bend: f32| -> (Entity, Entity) {
        let pivot = commands.spawn((Transform::default(), Visibility::default())).id();
        let l1 = 12.5; // upper arm (longer)
        let l2 = 13.0; // forearm (longer)
        let w = 6.6; // forearm thickness
        let wu = 8.8; // upper-arm (sleeve) thickness — bigger than the forearm

        // Upper arm is a beefy shirt sleeve; the forearm below is bare skin.
        let upper = commands.spawn(rrect(art, shirt, l1, wu, 0.1)).id();
        commands.entity(upper).insert(Transform::from_xyz(l1 * 0.5, 0.0, 0.1));
        // Elbow joint — a circle (bare skin, where the sleeve ends).
        let elbow = commands.spawn(ellipse(art, skin, w * 1.05, w * 1.05, 0.12)).id();
        commands.entity(elbow).insert(Transform::from_xyz(l1, 0.0, 0.12));

        // Forearm pivots at the elbow and bends inward.
        let forearm_pivot = commands
            .spawn((
                Transform::from_xyz(l1, 0.0, 0.0).with_rotation(Quat::from_rotation_z(bend)),
                Visibility::default(),
            ))
            .id();
        let forearm = commands.spawn(rrect(art, skin, l2, w * 0.92, 0.1)).id();
        commands.entity(forearm).insert(Transform::from_xyz(l2 * 0.5, 0.0, 0.1));
        let hand = commands.spawn(ellipse(art, skin_dark, 7.0, 7.0, 0.13)).id();
        commands.entity(hand).insert(Transform::from_xyz(l2 + 1.0, 0.0, 0.13));
        commands.entity(forearm_pivot).add_children(&[forearm, hand]);

        commands
            .entity(pivot)
            .add_children(&[upper, elbow, forearm_pivot]);
        (pivot, forearm_pivot)
    };
    // Right/gun arm bends up toward centre; left arm bends down toward centre.
    let (arm_l, fore_l) = build_arm(commands, -0.42);
    let (arm_r, fore_r) = build_arm(commands, 0.42);

    // ---- Head (smaller, sits at the player's centre) with headgear roots ----
    let head = commands.spawn(ellipse(art, skin, 12.5, 12.5, 0.25)).id();
    let brow = commands.spawn(ellipse(art, skin_dark, 9.0, 3.6, 0.255)).id();
    commands.entity(brow).insert(Transform::from_xyz(1.5, 0.0, 0.255));

    // Designed hair (shown only when bare-headed): a rounded crown that covers
    // the back and top of the head, with a couple of tufts, so the face peeks
    // out the front.
    let hair_col = Color::srgb(0.14, 0.10, 0.07);
    let hair_dark = Color::srgb(0.09, 0.06, 0.05);
    let hair = commands.spawn((Transform::default(), Visibility::default())).id();
    let hair_crown = commands.spawn(ellipse(art, hair_col, 14.0, 13.5, -0.06)).id();
    commands.entity(hair_crown).insert(Transform::from_xyz(-2.4, 0.0, -0.06));
    let hair_top = commands.spawn(ellipse(art, hair_col, 11.5, 13.0, 0.30)).id();
    commands.entity(hair_top).insert(Transform::from_xyz(-2.8, 0.0, 0.30));
    let tuft_a = commands.spawn(ellipse(art, hair_dark, 4.0, 4.5, 0.31)).id();
    commands.entity(tuft_a).insert(Transform::from_xyz(-6.0, 2.5, 0.31));
    let tuft_b = commands.spawn(ellipse(art, hair_dark, 4.0, 4.5, 0.31)).id();
    commands.entity(tuft_b).insert(Transform::from_xyz(-6.0, -2.5, 0.31));
    let fringe = commands.spawn(rrect(art, hair_col, 3.0, 9.0, 0.31)).id();
    commands.entity(fringe).insert(Transform::from_xyz(2.2, 0.0, 0.31));
    commands
        .entity(hair)
        .add_children(&[hair_crown, hair_top, tuft_a, tuft_b, fringe]);

    // Soft padded cap (default; no protection).
    let cap_root = commands.spawn((Transform::default(), Visibility::default())).id();
    let hat_base = commands.spawn(ellipse(art, hat, 18.0, 17.0, -0.05)).id();
    commands.entity(hat_base).insert(Transform::from_xyz(-4.0, 0.0, -0.05));
    let hat_brim = commands.spawn(ellipse(art, hat_dark, 8.0, 16.0, -0.045)).id();
    commands.entity(hat_brim).insert(Transform::from_xyz(1.5, 0.0, -0.045));
    let seg = |dx: f32, dy: f32, w: f32, h: f32| -> (Sprite, Transform) {
        (
            Sprite::from_color(hat_dark, Vec2::new(w, h)),
            Transform::from_xyz(dx, dy, -0.04),
        )
    };
    let hs1 = commands.spawn(seg(-3.0, -6.0, 3.2, 5.0)).id();
    let hs2 = commands.spawn(seg(-7.5, -1.5, 4.0, 3.0)).id();
    let hs3 = commands.spawn(seg(-7.5, 2.5, 4.0, 3.0)).id();
    let hs4 = commands.spawn(seg(-3.0, 6.0, 3.2, 5.0)).id();
    commands
        .entity(cap_root)
        .add_children(&[hat_base, hat_brim, hs1, hs2, hs3, hs4]);

    // Hard combat helmet (protective; toggled on when equipped).
    let helmet_root = commands.spawn((Transform::default(), Visibility::Hidden)).id();
    let helm_dome = commands.spawn(ellipse(art, Color::srgb(0.22, 0.26, 0.19), 16.5, 16.0, -0.04)).id();
    commands.entity(helm_dome).insert(Transform::from_xyz(-1.0, 0.0, -0.04));
    let helm_rim = commands.spawn(rect(Color::srgb(0.13, 0.15, 0.12), 3.5, 15.5, 0.052)).id();
    commands.entity(helm_rim).insert(Transform::from_xyz(6.5, 0.0, 0.052));
    let helm_ridge = commands.spawn(rect(Color::srgb(0.15, 0.17, 0.13), 12.5, 2.2, -0.035)).id();
    commands.entity(helm_ridge).insert(Transform::from_xyz(-1.0, 0.0, -0.035));
    commands
        .entity(helmet_root)
        .add_children(&[helm_dome, helm_rim, helm_ridge]);

    commands
        .entity(head)
        .add_children(&[hair, brow, cap_root, helmet_root]);

    // ---- Weapons: a distinct top-down model per kind, all hung off one weapon
    // pivot. Every gun is drawn barrel-forward (+X) with the grip at the origin
    // (where the hands hold it), so it never reads as a sideways bar. Only the
    // equipped kind is shown (see `animate_player`). ----
    let gun_dark = Color::srgb(0.05, 0.05, 0.06);
    let steel = Color::srgb(0.55, 0.57, 0.62);
    let wood = Color::srgb(0.32, 0.20, 0.11);
    let weapon = commands.spawn((Transform::default(), Visibility::default())).id();

    // Helper: spawn a rect part at (x,y) as a child of a weapon group.
    let part = |commands: &mut Commands, c: Color, w: f32, h: f32, x: f32, y: f32| -> Entity {
        commands
            .spawn((
                Sprite::from_color(c, Vec2::new(w, h)),
                Transform::from_xyz(x, y, 0.15),
            ))
            .id()
    };
    let group = |commands: &mut Commands, parts: Vec<Entity>| -> Entity {
        let g = commands.spawn((Transform::default(), Visibility::Hidden)).id();
        commands.entity(g).add_children(&parts);
        g
    };

    // Melee knife: a short blade forward of a dark handle.
    let melee_g = {
        let blade = part(commands, steel, 16.0, 3.0, 9.0, 0.0);
        let tip = part(commands, steel, 3.0, 5.0, 17.0, 0.0);
        let guard = part(commands, gun_dark, 2.5, 6.0, 1.0, 0.0);
        let handle = part(commands, wood, 5.0, 3.0, -2.0, 0.0);
        group(commands, vec![blade, tip, guard, handle])
    };

    // Bat: a wooden club — thin handle at the grip (origin) tapering out to a
    // fat barrel. Shown for the Bat weapon in place of the knife blade.
    let bat_wood = Color::srgb(0.46, 0.31, 0.17);
    let bat_wood_hi = Color::srgb(0.55, 0.38, 0.22);
    let bat_model = {
        let knob = part(commands, gun_dark, 2.5, 3.6, -2.0, 0.0);
        let grip = part(commands, bat_wood, 6.0, 2.8, 2.0, 0.0);
        let shaft = part(commands, bat_wood, 10.0, 3.6, 10.0, 0.0);
        let barrel = part(commands, bat_wood_hi, 9.0, 5.4, 18.5, 0.0);
        let cap = part(commands, bat_wood, 3.0, 5.8, 23.5, 0.0);
        group(commands, vec![knob, grip, shaft, barrel, cap])
    };

    // Pistol: compact slide + grip + magazine. The slide and magazine are
    // animated during a reload.
    let pistol_slide = part(commands, gun, 13.0, 5.0, 7.0, 0.0);
    let pistol_mag = part(commands, gun_dark, 4.0, 5.0, 0.0, -4.5);
    let pistol_g = {
        let frame = part(commands, gun_dark, 7.0, 6.5, 0.5, -0.5);
        let barrel = part(commands, gun_dark, 3.0, 3.0, 13.0, 0.0);
        group(commands, vec![frame, pistol_mag, pistol_slide, barrel])
    };

    // SMG (machine gun): body, foregrip mag, short barrel, stubby stock.
    let smg_g = {
        let body = part(commands, gun, 14.0, 5.0, 6.0, 0.0);
        let grip = part(commands, gun_dark, 5.0, 6.0, 0.5, -3.5);
        let mag = part(commands, gun_dark, 4.0, 7.0, 5.0, -5.5);
        let barrel = part(commands, gun_dark, 7.0, 2.6, 15.0, 0.0);
        let stock = part(commands, gun_dark, 5.0, 4.0, -3.0, 0.0);
        group(commands, vec![stock, body, grip, mag, barrel])
    };

    // Shotgun: barrel runs along the centreline (so it fires straight down the
    // sights); the stock angles back and down from the receiver to the butt so
    // the butt can tuck into the shoulder/armpit. The pump slides on the barrel.
    let shotgun_pump = part(commands, gun_dark, 6.0, 5.0, 9.0, 0.0);
    let shotgun_g = {
        let barrel = part(commands, gun, 24.0, 4.0, 13.0, 0.0);
        let receiver = part(commands, gun_dark, 6.0, 5.0, 2.0, -0.5);
        // Angled stock: long axis runs from the receiver down-back to the butt.
        let stock = commands
            .spawn((
                Sprite::from_color(wood, Vec2::new(11.0, 4.5)),
                Transform::from_xyz(-3.0, -4.0, 0.15).with_rotation(Quat::from_rotation_z(0.6)),
            ))
            .id();
        let grip = part(commands, gun_dark, 4.0, 5.5, -1.0, -5.5);
        group(commands, vec![stock, grip, receiver, barrel, shotgun_pump])
    };

    // Assault rifle: long body, a big curved mag (animated on reload), thin
    // barrel, stock, pistol grip.
    let rifle_mag = part(commands, gun_dark, 5.0, 9.0, 6.0, -6.0);
    let rifle_g = {
        let stock = part(commands, gun_dark, 7.0, 5.0, -4.0, 0.0);
        let body = part(commands, gun, 20.0, 4.5, 9.0, 0.0);
        let barrel = part(commands, gun_dark, 9.0, 2.4, 21.0, 0.0);
        let grip = part(commands, gun_dark, 4.0, 5.0, 2.5, -4.0);
        group(commands, vec![stock, body, grip, rifle_mag, barrel])
    };

    // Bazooka: fat tube with a rear vent, top sight and a pistol grip.
    let launcher_g = {
        let tube = part(commands, gun, 30.0, 7.5, 13.0, 0.0);
        let rear = part(commands, gun_dark, 5.0, 9.5, -3.5, 0.0);
        let sight = part(commands, gun_dark, 3.0, 4.0, 7.0, 4.5);
        let grip = part(commands, gun_dark, 4.0, 5.5, 3.0, -5.5);
        group(commands, vec![rear, tube, sight, grip])
    };

    // Side-by-side: twin barrels on a break-action hinge. `sxs_barrel` is the
    // pivot holding the two barrels + fore-end; it hinges open (rotates) on a
    // reload so the muzzles drop and two fresh shells go in. The stock/receiver
    // stay fixed. The hinge sits near the receiver (x≈2).
    let sxs_barrel = commands
        .spawn((Transform::from_xyz(2.0, 0.0, 0.0), Visibility::default()))
        .id();
    {
        // Two parallel barrels running forward from the hinge.
        let bl = part(commands, gun, 22.0, 2.6, 11.0, 2.1);
        let br = part(commands, gun, 22.0, 2.6, 11.0, -2.1);
        // Fore-end (wood) hugging the barrels.
        let fore = commands
            .spawn((
                Sprite::from_color(wood, Vec2::new(8.0, 6.5)),
                Transform::from_xyz(8.0, 0.0, 0.14),
            ))
            .id();
        commands.entity(sxs_barrel).add_children(&[fore, bl, br]);
    }
    let sxs_g = {
        let receiver = part(commands, gun_dark, 6.0, 7.0, 1.5, 0.0);
        // Angled wooden stock back to the shoulder.
        let stock = commands
            .spawn((
                Sprite::from_color(wood, Vec2::new(11.0, 4.5)),
                Transform::from_xyz(-3.0, -3.5, 0.15).with_rotation(Quat::from_rotation_z(0.5)),
            ))
            .id();
        let grip = part(commands, gun_dark, 4.0, 5.0, -1.0, -5.0);
        group(commands, vec![stock, grip, receiver, sxs_barrel])
    };

    let weapon_roots = [
        melee_g, pistol_g, smg_g, shotgun_g, rifle_g, launcher_g, sxs_g,
    ];
    commands.entity(weapon).add_children(&weapon_roots);
    // The bat shares the melee slot but swaps in its own model.
    commands.entity(weapon).add_child(bat_model);

    // Small square flash at the barrel tip (pixelated, not a soft glow).
    let flash = commands
        .spawn((
            Sprite::from_color(Color::srgba(1.0, 0.9, 0.5, 0.0), Vec2::splat(6.0)),
            Transform::from_xyz(39.0, 0.0, 0.3),
        ))
        .id();

    commands.entity(body).add_children(&[
        backpack_root, leg_l, leg_r, torso, arm_l, arm_r, weapon, head, flash,
    ]);

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
    commands.entity(root).insert(GearVisuals {
        cap_root,
        helmet_root,
        armor_root,
        backpack_root,
        hair,
    });
    commands.entity(root).insert(WeaponVisuals {
        roots: weapon_roots,
        pistol_slide,
        pistol_mag,
        shotgun_pump,
        rifle_mag,
        sxs_barrel,
        bat_model,
    });
    commands.entity(root).insert(PlayerArms { fore_l, fore_r });
}

fn build_zombie_rig(commands: &mut Commands, art: &Art, root: Entity, z: &Zombie) {
    let look = z.look;
    let s = z.r / 12.0;
    let crawler = look.crawler;
    let bone = Color::srgb(0.86, 0.83, 0.74);
    let blood = Color::srgb(0.35, 0.03, 0.04);
    let darker = |c: Color, f: f32| {
        let c = c.to_srgba();
        Color::srgb(c.red * f, c.green * f, c.blue * f)
    };

    // Contact shadow — flatter and longer for a body dragging on the ground.
    let (shw, shh) = if crawler { (42.0, 18.0) } else { (34.0, 24.0) };
    let shadow = commands
        .spawn((
            Sprite {
                image: art.soft.clone(),
                color: Color::srgba(0.0, 0.0, 0.0, 0.32),
                custom_size: Some(Vec2::new(shw * s, shh * s)),
                ..default()
            },
            Transform::from_xyz(1.0 * s, -5.0 * s, -0.5),
        ))
        .id();

    let body = commands.spawn((Transform::default(), Visibility::default())).id();
    let mut extras: Vec<Entity> = Vec::new();

    // ---- Legs: two-segment articulated limbs (hip → thigh → knee → shin →
    // foot), player-like proportions. A missing leg hides the whole pivot and is
    // replaced by a short bloody stump with a nub of bone. ----
    let shoe = darker(look.pants, 0.55);
    let knee_col = darker(look.pants, 0.82);
    // Returns (hip_pivot, knee_pivot). The limb is built extending +X (forward),
    // matching the arms, so it strides fore/aft cleanly.
    let build_leg = |commands: &mut Commands| -> (Entity, Entity) {
        let pivot = commands.spawn((Transform::default(), Visibility::default())).id();
        let l1 = 6.0 * s; // thigh
        let l2 = 5.5 * s; // shin
        let w = 5.2 * s;
        let thigh = commands.spawn(rect(look.pants, l1, w, -0.2)).id();
        commands.entity(thigh).insert(Transform::from_xyz(l1 * 0.5, 0.0, -0.2));
        let knee = commands.spawn(ellipse(art, knee_col, w, w, -0.19)).id();
        commands.entity(knee).insert(Transform::from_xyz(l1, 0.0, -0.19));
        let knee_pivot = commands
            .spawn((Transform::from_xyz(l1, 0.0, 0.0), Visibility::default()))
            .id();
        let shin = commands.spawn(rect(look.pants, l2, w * 0.9, -0.2)).id();
        commands.entity(shin).insert(Transform::from_xyz(l2 * 0.5, 0.0, -0.2));
        let foot = commands.spawn(rect(shoe, 4.5 * s, 5.0 * s, -0.19)).id();
        commands.entity(foot).insert(Transform::from_xyz(l2 + 1.5 * s, 0.0, -0.19));
        commands.entity(knee_pivot).add_children(&[shin, foot]);
        commands.entity(pivot).add_children(&[thigh, knee, knee_pivot]);
        (pivot, knee_pivot)
    };
    let (leg_l, shin_l) = build_leg(commands);
    let (leg_r, shin_r) = build_leg(commands);
    let mut stump = |commands: &mut Commands, x: f32, y: f32| {
        let st = commands.spawn(rect(look.skin, 4.0 * s, 5.0 * s, -0.19)).id();
        commands.entity(st).insert(Transform::from_xyz(x, y, -0.19));
        let bl = commands.spawn(rect(blood, 3.5 * s, 4.0 * s, -0.185)).id();
        commands.entity(bl).insert(Transform::from_xyz(x + 1.0 * s, y, -0.185));
        let bn = commands.spawn(rect(bone, 1.8 * s, 1.8 * s, -0.18)).id();
        commands.entity(bn).insert(Transform::from_xyz(x + 2.0 * s, y, -0.18));
        extras.push(st);
        extras.push(bl);
        extras.push(bn);
    };
    if look.missing_leg == 0 {
        commands.entity(leg_l).insert(Visibility::Hidden);
        stump(commands, -2.0 * s, 5.0 * s);
    } else if look.missing_leg == 1 {
        commands.entity(leg_r).insert(Visibility::Hidden);
        stump(commands, -2.0 * s, -5.0 * s);
    }

    // ---- Torso (rounded, distinctly thin front-to-back — they're dead, no
    // breath in the chest — but still broad shoulder-to-shoulder) with an
    // optional bloody gash/ribs. Front-to-back is the local X (facing) axis. ----
    let torso = commands.spawn(rrect(art, look.shirt, 11.0 * s, 16.0 * s, 0.0)).id();
    let back = commands.spawn(rrect(art, darker(look.shirt, 0.7), 5.0 * s, 14.0 * s, -0.01)).id();
    commands.entity(back).insert(Transform::from_xyz(-3.2 * s, 0.0, -0.01));
    commands.entity(torso).add_child(back);
    if look.gash {
        let wound = commands.spawn(rect(blood, 6.0 * s, 7.0 * s, 0.03)).id();
        commands.entity(wound).insert(Transform::from_xyz(2.0 * s, 1.5 * s, 0.03));
        let rib1 = commands.spawn(rect(bone, 5.0 * s, 1.1 * s, 0.04)).id();
        commands.entity(rib1).insert(Transform::from_xyz(2.0 * s, 0.5 * s, 0.04));
        let rib2 = commands.spawn(rect(bone, 5.0 * s, 1.1 * s, 0.04)).id();
        commands.entity(rib2).insert(Transform::from_xyz(2.0 * s, 3.0 * s, 0.04));
        commands.entity(torso).add_children(&[wound, rib1, rib2]);
    }

    // ---- Arms: two-segment articulated limbs (shoulder → upper arm → elbow →
    // forearm → hand), bare skin, longer + thinner than the torso. A missing arm
    // hides the whole pivot and is replaced by a stump + bone. ----
    let hand_col = darker(look.skin, 0.78);
    let elbow_col = darker(look.skin, 0.9);
    let build_arm = |commands: &mut Commands| -> (Entity, Entity) {
        let pivot = commands.spawn((Transform::default(), Visibility::default())).id();
        let l1 = 7.0 * s; // upper arm
        let l2 = 6.5 * s; // forearm
        let w = 4.4 * s;
        let upper = commands.spawn(rrect(art, look.skin, l1, w, 0.1)).id();
        commands.entity(upper).insert(Transform::from_xyz(l1 * 0.5, 0.0, 0.1));
        let elbow = commands.spawn(ellipse(art, elbow_col, w, w, 0.12)).id();
        commands.entity(elbow).insert(Transform::from_xyz(l1, 0.0, 0.12));
        let fore_pivot = commands
            .spawn((Transform::from_xyz(l1, 0.0, 0.0), Visibility::default()))
            .id();
        let forearm = commands.spawn(rrect(art, look.skin, l2, w * 0.88, 0.1)).id();
        commands.entity(forearm).insert(Transform::from_xyz(l2 * 0.5, 0.0, 0.1));
        let hand = commands.spawn(ellipse(art, hand_col, w * 1.15, w * 1.15, 0.13)).id();
        commands.entity(hand).insert(Transform::from_xyz(l2 + 1.0 * s, 0.0, 0.13));
        commands.entity(fore_pivot).add_children(&[forearm, hand]);
        commands.entity(pivot).add_children(&[upper, elbow, fore_pivot]);
        (pivot, fore_pivot)
    };
    let (arm_l, fore_l) = build_arm(commands);
    let (arm_r, fore_r) = build_arm(commands);
    if look.missing_arm == 0 {
        commands.entity(arm_l).insert(Visibility::Hidden);
        let st = commands.spawn(rect(look.skin, 4.0 * s, 4.5 * s, 0.11)).id();
        commands.entity(st).insert(Transform::from_xyz(4.0 * s, 5.0 * s, 0.11));
        let bn = commands.spawn(rect(bone, 1.6 * s, 1.6 * s, 0.12)).id();
        commands.entity(bn).insert(Transform::from_xyz(6.0 * s, 5.0 * s, 0.12));
        extras.push(st);
        extras.push(bn);
    } else if look.missing_arm == 1 {
        commands.entity(arm_r).insert(Visibility::Hidden);
        let st = commands.spawn(rect(look.skin, 4.0 * s, 4.5 * s, 0.11)).id();
        commands.entity(st).insert(Transform::from_xyz(4.0 * s, -5.0 * s, 0.11));
        let bn = commands.spawn(rect(bone, 1.6 * s, 1.6 * s, 0.12)).id();
        commands.entity(bn).insert(Transform::from_xyz(6.0 * s, -5.0 * s, 0.12));
        extras.push(st);
        extras.push(bn);
    }

    // ---- Torn clothing dragging behind (a couple of ragged strips off the
    // back that flap as the zombie moves). ----
    let tatters = if look.tatters {
        let rag = darker(look.shirt, 0.65);
        let root = commands.spawn((Transform::default(), Visibility::default())).id();
        let strip_a = commands.spawn(rect(rag, 10.0 * s, 3.5 * s, -0.02)).id();
        commands.entity(strip_a).insert(Transform::from_xyz(-11.0 * s, 2.5 * s, -0.02));
        let strip_b = commands.spawn(rect(darker(rag, 0.85), 8.0 * s, 3.0 * s, -0.02)).id();
        commands.entity(strip_b).insert(Transform::from_xyz(-10.0 * s, -2.5 * s, -0.02));
        commands.entity(root).add_children(&[strip_a, strip_b]);
        commands.entity(body).add_child(root);
        Some(root)
    } else {
        None
    };

    // ---- Head + hair. ----
    let head = commands.spawn(ellipse(art, look.skin, 13.0 * s, 13.0 * s, 0.25)).id();
    if look.hair >= 0 {
        let hh = if look.hair == 1 { 13.0 } else { 10.5 };
        let h = commands.spawn(ellipse(art, look.hair_col, 13.0 * s, hh * s, 0.24)).id();
        commands.entity(h).insert(Transform::from_xyz(-2.0 * s, 0.0, 0.24));
        commands.entity(head).add_child(h);
    }

    // placeholders so Rig fields are populated
    let weapon = commands.spawn((Transform::default(), Visibility::Hidden)).id();
    let flash = commands.spawn((Transform::default(), Visibility::Hidden)).id();

    commands
        .entity(body)
        .add_children(&[leg_l, leg_r, torso, arm_l, arm_r, head]);
    if !extras.is_empty() {
        commands.entity(body).add_children(&extras);
    }
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
    commands.entity(root).insert(ZombieLimbs {
        fore_l,
        fore_r,
        shin_l,
        shin_r,
        tatters,
    });
}

/// Chest fullness (0..1) over one breath cycle: quick inhale, brief hold,
/// slower exhale, then a short rest at empty — an organic, rhythmic curve.
fn breath_curve(phase: f32) -> f32 {
    let ss = |t: f32| t * t * (3.0 - 2.0 * t);
    if phase < 0.30 {
        ss(phase / 0.30)
    } else if phase < 0.42 {
        1.0
    } else if phase < 0.85 {
        1.0 - ss((phase - 0.42) / 0.43)
    } else {
        0.0
    }
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

/// Right-elbow bend for the knife: held bent for a slash, straightening out
/// through a stab so the blade lunges forward. Shared by the pose and the hand-
/// tracking so the gripped knife lines up with the actual hand.
fn melee_fore_bend_r(stab: bool, arc: f32) -> f32 {
    // Rest keeps the elbow cocked (~1.5). A stab straightens it hard to lunge the
    // blade out; a slash straightens it only a little as the whole arm sweeps.
    if stab {
        1.5 - arc * 1.6
    } else {
        1.5 - arc * 0.6
    }
}

/// Break-action open amount (0 = closed/level, 1 = fully broken open, muzzles at
/// the ground) across a reload cycle. It snaps open fast, holds open while the
/// spent shells eject and two fresh shells go in, then raises shut so the shooter
/// can fire again.
fn sxs_open(rl: f32) -> f32 {
    let ss = |t: f32| t * t * (3.0 - 2.0 * t);
    if rl < 0.16 {
        ss(rl / 0.16)
    } else if rl < 0.72 {
        1.0
    } else {
        1.0 - ss((rl - 0.72) / 0.28)
    }
}

/// Two-bone inverse kinematics for an arm: given the shoulder pivot and a target
/// for the fist, return (shoulder_rotation, elbow_bend) that lands the hand on
/// the target. `side` (±1) picks which way the elbow bows. Segment lengths match
/// the player rig (upper arm 12.5, forearm 13 + 1 to the fist ≈ 14).
fn ik_arm(shoulder: Vec2, target: Vec2, side: f32) -> (f32, f32) {
    let a: f32 = 12.5;
    let b: f32 = 14.0;
    let to = target - shoulder;
    let d = to.length().clamp((a - b).abs() + 0.5, a + b - 0.5);
    let base = to.y.atan2(to.x);
    let cos_a = ((a * a + d * d - b * b) / (2.0 * a * d)).clamp(-1.0, 1.0);
    let r1 = base + side * cos_a.acos();
    // Point the forearm straight at the target from the resolved elbow.
    let elbow = shoulder + Vec2::new(r1.cos(), r1.sin()) * a;
    let to_hand = target - elbow;
    let r2 = to_hand.y.atan2(to_hand.x) - r1;
    (r1, r2)
}

pub fn animate_player(
    player_q: Query<(&Player, &Rig, &WeaponVisuals, &PlayerArms)>,
    mut tf_q: Query<&mut Transform>,
    mut sprite_q: Query<&mut Sprite>,
    mut vis_q: Query<&mut Visibility>,
) {
    let Ok((p, rig, wv, pa)) = player_q.single() else {
        return;
    };
    let angle = p.angle;
    if let Ok(mut b) = tf_q.get_mut(rig.body) {
        b.rotation = Quat::from_rotation_z(angle);
    }

    let stamina_frac = (p.stamina / p.max_stamina).clamp(0.0, 1.0);
    let stride = p.walk_frame.sin();
    // Bigger, springier gait; running throws the legs much further.
    let leg_amp = if p.running { 13.0 } else { 8.5 };
    let lift = if p.running { 3.0 } else { 1.8 };
    let sway = if p.running { 0.16 } else { 0.10 };

    if p.moving {
        // Legs scissor fore/aft (local X) and lift a little as they swing.
        if let Ok(mut l) = tf_q.get_mut(rig.leg_l) {
            l.translation.x = -3.0 + stride * leg_amp;
            l.translation.y = 5.0 + stride.max(0.0) * lift;
        }
        if let Ok(mut r) = tf_q.get_mut(rig.leg_r) {
            r.translation.x = -3.0 - stride * leg_amp;
            r.translation.y = -5.0 - (-stride).max(0.0) * lift;
        }
        // Torso bobs vertically and rocks side to side (twice per stride).
        let bob = p.walk_frame.sin().abs() * if p.running { 2.4 } else { 1.5 };
        if let Ok(mut t) = tf_q.get_mut(rig.torso) {
            t.translation.y = bob;
            t.scale = Vec3::ONE;
            t.rotation = Quat::from_rotation_z((p.walk_frame).sin() * sway);
        }
        if let Ok(mut h) = tf_q.get_mut(rig.head) {
            h.translation.x = 0.0; // head sits at the player's centre
            h.translation.y = bob * 0.5;
        }
    } else {
        // Idle: rhythmic breathing. A steady breaths-per-second cadence (faster
        // and deeper as stamina drops) with a shaped inhale/hold/exhale/rest —
        // not a plain sine.
        let bps = 0.28 + (1.0 - stamina_frac) * 0.55 + if p.exhausted { 0.35 } else { 0.0 };
        let phase = (p.idle_t * bps).fract();
        let fullness = breath_curve(phase); // 0 (empty) .. 1 (full chest)
        let depth = 0.05 + (1.0 - stamina_frac) * 0.08;
        if let Ok(mut l) = tf_q.get_mut(rig.leg_l) {
            l.translation.x = -3.0;
            l.translation.y = 5.0;
        }
        if let Ok(mut r) = tf_q.get_mut(rig.leg_r) {
            r.translation.x = -3.0;
            r.translation.y = -5.0;
        }
        if let Ok(mut t) = tf_q.get_mut(rig.torso) {
            // Chest swells on the inhale and settles on the exhale.
            t.scale = Vec3::splat(1.0 + fullness * depth);
            t.translation.y = fullness * 0.7;
            t.rotation = Quat::IDENTITY;
        }
        if let Ok(mut h) = tf_q.get_mut(rig.head) {
            h.translation.x = 0.0;
            h.translation.y = fullness * 0.6;
        }
    }

    // Arms + weapon depend on weapon type / recoil / swing / reload.
    let w = p.weapon();
    let melee = w.kind == WeaponKind::Melee;
    // The bat shares the Melee kind but is a two-handed club with its own swings.
    let is_bat = melee && w.name == "Bat";
    let recoil = p.recoil;
    let mag_fed = matches!(
        w.kind,
        WeaponKind::Pistol | WeaponKind::Smg | WeaponKind::Rifle
    );

    // Show only the equipped weapon's model.
    let cur_kind = w.kind.index();
    for (i, &e) in wv.roots.iter().enumerate() {
        if let Ok(mut v) = vis_q.get_mut(e) {
            *v = if i == cur_kind && !(is_bat && i == 0) {
                // The bat swaps its own model in for the melee (knife) slot.
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
    if let Ok(mut v) = vis_q.get_mut(wv.bat_model) {
        *v = if is_bat { Visibility::Inherited } else { Visibility::Hidden };
    }
    // The knife is worked one-handed (hide the left arm); the bat is two-handed.
    if let Ok(mut v) = vis_q.get_mut(rig.arm_l) {
        *v = if melee && !is_bat {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }

    // Reload progress (0..1) drives a per-gun reload animation whose length
    // automatically matches each weapon's own reload cycle time.
    let reloading = p.reloading > 0.0;
    let rl = p.reload_progress();
    // Support hand fetches/seats a fresh magazine: a single dip-and-return that
    // peaks at mid-cycle.
    let swap = if reloading {
        (rl * std::f32::consts::PI).sin()
    } else {
        0.0
    };
    // Slide/pump racks back near the start and slams home at the very end.
    let rack = if reloading {
        if rl < 0.15 {
            rl / 0.15
        } else if rl > 0.82 {
            ((1.0 - rl) / 0.18).clamp(0.0, 1.0)
        } else {
            1.0
        }
    } else {
        0.0
    };

    // Elbow bends the bat computes via IK, applied in the forearm-bend section.
    let mut bat_fore: Option<(f32, f32)> = None;

    // Arms are shoulder pivots at the shoulders; the forearm bend (baked in)
    // brings both hands onto the gun. We drive the shoulder position + rotation.
    if is_bat {
        // Two-handed club. Attacks alternate a horizontal BASEBALL swing (the bat
        // whips across the front) and an overhead EXECUTIONER chop (raised behind
        // the head, then driven down forward). Both hands are placed on the handle
        // and the arms are solved by IK so they actually grip it.
        let sw = if p.swing_dur > 0.0 {
            (p.swing_t / p.swing_dur).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let arc = (sw * std::f32::consts::PI).sin(); // 0 rest .. 1 full strike
        let chop = p.melee_stab;
        // Grip point (bat handle origin) and bat heading.
        let (g, theta) = if chop {
            // Overhead: raised behind the head (bat points back), chops to forward.
            (
                Vec2::new(-2.0 + arc * 15.0, 0.0),
                std::f32::consts::PI * (1.0 - arc),
            )
        } else {
            // Baseball: cocked over the right shoulder, sweeps across to front-left.
            (
                Vec2::new(2.0 + arc * 14.0, -6.0 + arc * 12.0),
                2.4 - arc * 3.1,
            )
        };
        let dir = Vec2::new(theta.cos(), theta.sin());
        let hand_r = g + dir * 4.0; // rear hand on the handle
        let hand_l = g + dir * 1.0; // front hand, just ahead of it
        let sh_r = Vec2::new(1.0, -7.5);
        let sh_l = Vec2::new(1.0, 7.5);
        let (r1r, r2r) = ik_arm(sh_r, hand_r, -1.0);
        let (r1l, r2l) = ik_arm(sh_l, hand_l, 1.0);
        if let Ok(mut a) = tf_q.get_mut(rig.arm_r) {
            a.translation = Vec3::new(sh_r.x, sh_r.y, 0.1);
            a.rotation = Quat::from_rotation_z(r1r);
        }
        if let Ok(mut a) = tf_q.get_mut(rig.arm_l) {
            a.translation = Vec3::new(sh_l.x, sh_l.y, 0.1);
            a.rotation = Quat::from_rotation_z(r1l);
        }
        if let Ok(mut wt) = tf_q.get_mut(rig.weapon) {
            wt.translation = Vec3::new(g.x, g.y, 0.15);
            wt.rotation = Quat::from_rotation_z(theta);
        }
        bat_fore = Some((r2l, r2r));
    } else if melee {
        // One-armed knife work in the RIGHT hand (the left arm is hidden for
        // melee — see below). Attacks alternate a wide SLASH (the arm swings the
        // blade across the chest) and a forward STAB (the elbow straightens to
        // drive the blade out along the aim). The knife is pinned to the hand:
        // its position/rotation are derived from the arm + elbow angles so it
        // stays gripped instead of floating.
        let sw = if p.swing_dur > 0.0 {
            (p.swing_t / p.swing_dur).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let arc = (sw * std::f32::consts::PI).sin(); // 0 at rest, 1 at full extension
        let stab = p.melee_stab;
        // Shoulder mount, arm angle (r1) and elbow bend (r2). A stab aims the arm
        // forward and straightens the elbow to lunge; a slash sweeps the shoulder
        // across with the elbow held bent.
        let sh = Vec2::new(2.0, -7.0);
        let r1 = if stab { -0.5 + arc * 0.2 } else { -0.5 + arc * 1.9 };
        let r2 = melee_fore_bend_r(stab, arc);
        if let Ok(mut a) = tf_q.get_mut(rig.arm_r) {
            a.translation = Vec3::new(sh.x, sh.y, 0.1);
            a.rotation = Quat::from_rotation_z(r1);
        }
        // Follow the hand at the end of the two-segment arm (upper arm 12.5 +
        // forearm 13 + 1 to the fist) and grip the knife there, blade pointing
        // out along the forearm.
        let elbow = sh + Vec2::new(r1.cos(), r1.sin()) * 12.5;
        let hand = elbow + Vec2::new((r1 + r2).cos(), (r1 + r2).sin()) * 14.0;
        if let Ok(mut wt) = tf_q.get_mut(rig.weapon) {
            wt.translation = Vec3::new(hand.x, hand.y, 0.15);
            wt.rotation = Quat::from_rotation_z(r1 + r2);
        }
    } else if w.kind == WeaponKind::Launcher {
        // Shoulder-mounted bazooka: the tube rests up on the RIGHT shoulder and
        // points forward down the aim line. The right hand grips the trigger
        // under the tube; the left hand steadies the tube out front.
        let back = recoil * 5.0;
        if let Ok(mut a) = tf_q.get_mut(rig.arm_r) {
            a.translation = Vec3::new(1.0 - back, -7.5, 0.1);
            a.rotation = Quat::from_rotation_z(-0.15);
        }
        if let Ok(mut a) = tf_q.get_mut(rig.arm_l) {
            // Reaches across to brace the tube (which sits on the right).
            a.translation = Vec3::new(1.0 - back, 6.5, 0.1);
            a.rotation = Quat::from_rotation_z(-0.6);
        }
        if let Ok(mut wt) = tf_q.get_mut(rig.weapon) {
            // Tube hoisted onto the right shoulder, muzzle forward; kicks back on fire.
            wt.translation = Vec3::new(12.0 - back, -6.0, 0.15);
            wt.rotation = Quat::from_rotation_z(-0.05);
        }
    } else if w.kind == WeaponKind::Shotgun {
        // Shouldered pump shotgun. The barrel runs level along the aim line so it
        // fires straight; the stock drops to the shoulder. The left hand works
        // the pump (racking back on each shot to eject + chamber a shell) and the
        // right elbow cocks back to the trigger. (Forearm folds applied below.)
        let back = recoil * 2.5;
        let pump = recoil.max(rack);
        // Left/support hand on the pump — pulls back as it's racked.
        if let Ok(mut a) = tf_q.get_mut(rig.arm_l) {
            a.translation = Vec3::new(1.0 - pump * 6.0, 7.5, 0.1);
            a.rotation = Quat::from_rotation_z(0.31);
        }
        // Right/trigger hand, cocked back to the grip.
        if let Ok(mut a) = tf_q.get_mut(rig.arm_r) {
            a.translation = Vec3::new(1.0 - back, -7.5, 0.1);
            a.rotation = Quat::from_rotation_z(-1.12);
        }
        if let Ok(mut wt) = tf_q.get_mut(rig.weapon) {
            // Barrel level and centred; recoils straight back on fire.
            wt.translation = Vec3::new(12.0 - back, 0.0, 0.15);
            wt.rotation = Quat::IDENTITY;
        }
    } else if w.kind == WeaponKind::Sxs {
        // Shouldered side-by-side. Held level to fire; on firing it kicks up and
        // back. On a reload the action breaks open — the barrels hinge down to
        // point at the ground while the support hand feeds two fresh shells into
        // the breech, then the barrels raise shut so it can fire again.
        let back = recoil * 2.3;
        let open = if reloading { sxs_open(rl) } else { 0.0 };
        // Support/left hand: on the fore-end when closed, dropping to the open
        // breech to load shells.
        if let Ok(mut a) = tf_q.get_mut(rig.arm_l) {
            a.translation = Vec3::new(1.0 - open * 3.0, 7.5 - open * 3.5, 0.1);
            a.rotation = Quat::from_rotation_z(0.31 - open * 0.55);
        }
        // Right/trigger hand cocked back at the grip.
        if let Ok(mut a) = tf_q.get_mut(rig.arm_r) {
            a.translation = Vec3::new(1.0 - back, -7.5, 0.1);
            a.rotation = Quat::from_rotation_z(-1.05);
        }
        if let Ok(mut wt) = tf_q.get_mut(rig.weapon) {
            // Level when firing; the muzzles kick up and the gun jumps back on
            // recoil; the whole gun tips down as it breaks open to reload.
            wt.translation = Vec3::new(12.0 - back, -open * 4.0 + recoil * 2.2, 0.15);
            wt.rotation = Quat::from_rotation_z(-open * 0.5 + recoil * 0.45);
        }
    } else {
        // Two-handed grip: the grip sits at the hands (~x=24), recoiling back on
        // fire. On a reload the left hand keeps the gun steady while the right
        // hand fetches a magazine and drives it up into the well at the bottom
        // of the grip (forearm folds handle the reach — applied below).
        let back = recoil * 5.0;
        if let Ok(mut a) = tf_q.get_mut(rig.arm_l) {
            a.translation = Vec3::new(1.0 - back, 7.5, 0.1);
            a.rotation = Quat::from_rotation_z(0.10 * swap);
        }
        if let Ok(mut a) = tf_q.get_mut(rig.arm_r) {
            // Drops straight down under the grip to seat the magazine.
            a.translation = Vec3::new(1.0 - back, -7.5 - 1.5 * swap, 0.1);
            a.rotation = Quat::from_rotation_z(-0.5 * swap);
        }
        if let Ok(mut wt) = tf_q.get_mut(rig.weapon) {
            // Tips toward the shooter a touch while reloading.
            wt.translation = Vec3::new(24.0 - back, -1.0 * swap, 0.15);
            wt.rotation = Quat::from_rotation_z(0.14 * swap);
        }
    }

    // Forearm (elbow) bends. Default to the baked resting bend; the shotgun folds
    // the elbows for its pump/trigger, and a mag-fed reload folds the right elbow
    // down so the hand reaches the magazine well under the grip.
    let (fore_bend_l, mut fore_bend_r) = if let Some((fl, fr)) = bat_fore {
        // The bat's elbows come straight from the two-handed grip IK.
        (fl, fr)
    } else if melee {
        // Left elbow folds hard so the hand tucks low at the waist. The right
        // elbow stays bent (knife cocked) for a slash, and straightens out as it
        // drives forward on a stab.
        let sw = if p.swing_dur > 0.0 {
            (p.swing_t / p.swing_dur).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let arc = (sw * std::f32::consts::PI).sin();
        (-1.6, melee_fore_bend_r(p.melee_stab, arc))
    } else if w.kind == WeaponKind::Launcher {
        // Left arm folds across to brace the tube; right elbow bent up to the grip.
        (-1.1, 0.5)
    } else if matches!(w.kind, WeaponKind::Shotgun | WeaponKind::Sxs) {
        (-1.26, 2.36)
    } else {
        (-0.42, 0.42)
    };
    if mag_fed && reloading && w.kind != WeaponKind::Shotgun {
        fore_bend_r = 0.42 + 1.15 * swap;
    }
    if let Ok(mut f) = tf_q.get_mut(pa.fore_l) {
        f.rotation = Quat::from_rotation_z(fore_bend_l);
    }
    if let Ok(mut f) = tf_q.get_mut(pa.fore_r) {
        f.rotation = Quat::from_rotation_z(fore_bend_r);
    }

    // Shotgun pump slides back as it's racked (on fire and through the reload).
    if let Ok(mut pt) = tf_q.get_mut(wv.shotgun_pump) {
        let pump = recoil.max(rack);
        pt.translation.x = 9.0 - 6.0 * pump;
    }

    // Side-by-side barrels hinge open on a reload (break action), then snap shut.
    if let Ok(mut bt) = tf_q.get_mut(wv.sxs_barrel) {
        let open = if reloading && w.kind == WeaponKind::Sxs {
            sxs_open(rl)
        } else {
            0.0
        };
        bt.rotation = Quat::from_rotation_z(-open * 0.9);
    }

    // Recoil kick: the head and upper body rock back a touch when firing.
    // Absolute assignment (torso.x is otherwise never reset, so `-=` would drift).
    if let Ok(mut h) = tf_q.get_mut(rig.head) {
        // The head's x was reset to 0 above this frame, so offset from there.
        h.translation.x -= recoil * 2.2;
    }
    if let Ok(mut t) = tf_q.get_mut(rig.torso) {
        t.translation.x = -recoil * 1.3;
    }

    // Pistol slide racks back a little as the fresh magazine is chambered.
    if let Ok(mut st) = tf_q.get_mut(wv.pistol_slide) {
        st.translation.x = 7.0 - 4.5 * rack;
    }

    // Magazine drop/insert. The mag sits in the gun normally; on a reload it
    // drops away (a spent-mag particle handles the fall) for the first half,
    // then a fresh one rises up into the well at the bottom of the grip.
    // (rest_y = seated position, rise = how far below it comes from.)
    let out = reloading && rl < 0.5;
    let seat = if reloading && rl >= 0.5 {
        ((rl - 0.5) / 0.28).clamp(0.0, 1.0)
    } else {
        1.0
    };
    // Pistol mag (rest at grip bottom).
    {
        let show = w.kind == WeaponKind::Pistol && !out;
        if let Ok(mut mv) = vis_q.get_mut(wv.pistol_mag) {
            *mv = if show { Visibility::Inherited } else { Visibility::Hidden };
        }
        if let Ok(mut mt) = tf_q.get_mut(wv.pistol_mag) {
            mt.translation.y = -4.5 - (1.0 - seat) * 10.0;
        }
    }
    // Rifle mag (bigger, further forward under the receiver).
    {
        let show = w.kind == WeaponKind::Rifle && !out;
        if let Ok(mut mv) = vis_q.get_mut(wv.rifle_mag) {
            *mv = if show { Visibility::Inherited } else { Visibility::Hidden };
        }
        if let Ok(mut mt) = tf_q.get_mut(wv.rifle_mag) {
            mt.translation.y = -6.0 - (1.0 - seat) * 13.0;
        }
    }

    // Muzzle flash: sit it at each weapon's barrel tip and make the rifle's
    // bigger (and the bazooka's), so it reads clear of the body.
    let (flash_x, flash_sz) = match w.kind {
        WeaponKind::Rifle => (50.0, 12.0),
        WeaponKind::Smg => (42.0, 7.0),
        WeaponKind::Shotgun => (37.0, 8.0),
        WeaponKind::Sxs => (36.0, 9.0),
        WeaponKind::Launcher => (50.0, 11.0),
        _ => (39.0, 6.0), // pistol
    };
    if let Ok(mut ft) = tf_q.get_mut(rig.flash) {
        ft.translation.x = flash_x;
        // The bazooka's muzzle sits over the right shoulder, so drop its flash.
        ft.translation.y = if w.kind == WeaponKind::Launcher { -6.0 } else { 0.0 };
    }
    if let Ok(mut fs) = sprite_q.get_mut(rig.flash) {
        fs.custom_size = Some(Vec2::splat(flash_sz));
        fs.color = Color::srgba(1.0, 0.85, 0.4, (p.muzzle * 6.0).clamp(0.0, 1.0) * 0.9);
    }

    // Hurt flash tint.
    let flash = (p.hurt_flash * 5.0).clamp(0.0, 1.0);
    let shirt = PLAYER_SHIRT;
    let skin = PLAYER_SKIN;
    if let Ok(mut s) = sprite_q.get_mut(rig.torso) {
        s.color = mix(shirt, Color::WHITE, flash * 0.7);
    }
    if let Ok(mut s) = sprite_q.get_mut(rig.head) {
        s.color = mix(skin, Color::WHITE, flash * 0.7);
    }
}

/// Show/hide swappable gear groups to match the player's equipped gear.
pub fn update_gear_visuals(
    player_q: Query<(&Player, &GearVisuals)>,
    mut vis_q: Query<&mut Visibility>,
) {
    let Ok((p, g)) = player_q.single() else {
        return;
    };
    let mut set = |e: Entity, on: bool| {
        if let Ok(mut v) = vis_q.get_mut(e) {
            *v = if on { Visibility::Inherited } else { Visibility::Hidden };
        }
    };
    set(g.cap_root, p.head_gear == HeadGear::Cap);
    set(g.helmet_root, p.head_gear == HeadGear::Helmet);
    set(g.hair, p.head_gear == HeadGear::Bare);
    set(g.armor_root, p.body_gear == BodyGear::Armor);
    set(g.backpack_root, p.has_backpack);
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

/// Set a joint pivot's bend (rotation about z), ignoring a missing entity.
fn set_bend(tf_q: &mut Query<&mut Transform>, e: Entity, r: f32) {
    if let Ok(mut t) = tf_q.get_mut(e) {
        t.rotation = Quat::from_rotation_z(r);
    }
}

pub fn animate_zombies(
    zombies: Query<(&Zombie, &Rig, &ZombieLimbs)>,
    mut tf_q: Query<&mut Transform>,
    mut sprite_q: Query<&mut Sprite>,
) {
    for (z, rig, limbs) in zombies.iter() {
        let s = z.r / 12.0;
        let crawler = z.look.crawler;
        let moving = z.vel.length_squared() > 4.0;
        let stride = if moving { (z.frame * z.stride_rate * 2.0).sin() } else { 0.0 };

        // ---- Ragdoll death: as hp hits zero the body topples, limbs flop limp
        // and splay out, and it flattens front-to-back as it drops to the ground
        // (visible from the top-down view), before the corpse decal takes over.
        if z.death_t > 0.0 {
            let p = (z.death_t / crate::combat::DEATH_DUR).clamp(0.0, 1.0);
            let fall = p * p * (3.0 - 2.0 * p); // smoothstep
            let sgn = if z.death_spin >= 0.0 { 1.0 } else { -1.0 };
            if let Ok(mut b) = tf_q.get_mut(rig.body) {
                b.rotation = Quat::from_rotation_z(z.angle + z.death_spin * fall);
                // Sprawl a touch wider and flatten front-to-back as it goes down.
                b.scale = Vec3::new(1.0 + 0.14 * fall, 1.0 - 0.24 * fall, 1.0);
            }
            // Arms flop out limp to the sides.
            if let Ok(mut a) = tf_q.get_mut(rig.arm_l) {
                a.translation = Vec3::new(-1.0 * s, (8.0 + 4.0 * fall) * s, 0.1);
                a.rotation = Quat::from_rotation_z(0.5 + 1.2 * fall);
            }
            if let Ok(mut a) = tf_q.get_mut(rig.arm_r) {
                a.translation = Vec3::new(-1.0 * s, (-8.0 - 4.0 * fall) * s, 0.1);
                a.rotation = Quat::from_rotation_z(-0.5 - 1.2 * fall);
            }
            set_bend(&mut tf_q, limbs.fore_l, -0.5 + 0.8 * fall);
            set_bend(&mut tf_q, limbs.fore_r, 0.5 - 0.8 * fall);
            // Legs splay out and go slack.
            if let Ok(mut l) = tf_q.get_mut(rig.leg_l) {
                l.translation = Vec3::new((-4.0 - 2.0 * fall) * s, (5.0 + 3.0 * fall) * s, -0.2);
                l.rotation = Quat::from_rotation_z(0.35 * fall);
            }
            if let Ok(mut r) = tf_q.get_mut(rig.leg_r) {
                r.translation = Vec3::new((-4.0 - 2.0 * fall) * s, (-5.0 - 3.0 * fall) * s, -0.2);
                r.rotation = Quat::from_rotation_z(-0.35 * fall);
            }
            set_bend(&mut tf_q, limbs.shin_l, 0.22 * fall);
            set_bend(&mut tf_q, limbs.shin_r, -0.22 * fall);
            // Head lolls to the side as it drops.
            if let Ok(mut h) = tf_q.get_mut(rig.head) {
                h.translation.x = (4.0 + 2.0 * fall) * s;
                h.translation.y = sgn * 3.0 * fall * s;
            }
            continue;
        }

        if let Ok(mut b) = tf_q.get_mut(rig.body) {
            // Shambling body sway around the facing angle (calmer when crawling).
            let sway = (z.frame * 1.5).sin() * if crawler { 0.06 } else { 0.18 };
            b.rotation = Quat::from_rotation_z(z.angle + sway);
            // A crawler lies flatter to the ground.
            b.scale = if crawler {
                Vec3::new(0.92, 0.8, 1.0)
            } else {
                Vec3::ONE
            };
        }

        if crawler {
            // Dragging: the arms crawl — each one in turn reaches out ahead, plants,
            // and hauls the body forward while the other recovers, legs trailing.
            let c = z.frame * 2.4;
            let la = c.sin(); // left-arm cycle
            let ra = (c + std::f32::consts::PI).sin(); // right opposite
            let reach_l = la.max(0.0); // 0 = pulling back, 1 = reaching ahead
            let reach_r = ra.max(0.0);
            if let Ok(mut a) = tf_q.get_mut(rig.arm_l) {
                a.translation = Vec3::new((3.0 + reach_l * 7.0) * s, 6.0 * s, 0.1);
                a.rotation = Quat::from_rotation_z(0.15 - la * 0.35);
            }
            if let Ok(mut a) = tf_q.get_mut(rig.arm_r) {
                a.translation = Vec3::new((3.0 + reach_r * 7.0) * s, -6.0 * s, 0.1);
                a.rotation = Quat::from_rotation_z(-0.15 + ra * 0.35);
            }
            // Elbows straighten as the arm reaches out, fold as it pulls back.
            set_bend(&mut tf_q, limbs.fore_l, -0.7 + reach_l * 0.55);
            set_bend(&mut tf_q, limbs.fore_r, 0.7 - reach_r * 0.55);
            // Legs trail behind the body (pivots rotated to point backward).
            if let Ok(mut l) = tf_q.get_mut(rig.leg_l) {
                l.translation = Vec3::new(-5.0 * s, 3.5 * s, -0.2);
                l.rotation = Quat::from_rotation_z(std::f32::consts::PI + 0.12 + stride * 0.12);
            }
            if let Ok(mut r) = tf_q.get_mut(rig.leg_r) {
                r.translation = Vec3::new(-5.0 * s, -3.5 * s, -0.2);
                r.rotation = Quat::from_rotation_z(std::f32::consts::PI - 0.12 - stride * 0.12);
            }
            set_bend(&mut tf_q, limbs.shin_l, 0.3);
            set_bend(&mut tf_q, limbs.shin_r, -0.3);
            if let Ok(mut h) = tf_q.get_mut(rig.head) {
                h.translation.x = 6.0 * s;
                h.translation.y = 0.0;
            }
        } else {
            // Upright shamble. Missing a leg makes the gait limp (the good leg
            // works harder) and bobs the head.
            let amp = 4.0 * s;
            let limp_l = if z.look.missing_leg == 1 { 1.7 } else { 1.0 };
            let limp_r = if z.look.missing_leg == 0 { 1.7 } else { 1.0 };
            // Hips scissor fore/aft; knees flex on the recovering leg so the
            // stride reads as a jointed walk, not a stiff plank.
            if let Ok(mut l) = tf_q.get_mut(rig.leg_l) {
                l.translation.x = -3.0 * s + stride * amp * limp_l;
                l.translation.y = 5.0 * s;
                l.rotation = Quat::from_rotation_z(stride * 0.12);
            }
            if let Ok(mut r) = tf_q.get_mut(rig.leg_r) {
                r.translation.x = -3.0 * s - stride * amp * limp_r;
                r.translation.y = -5.0 * s;
                r.rotation = Quat::from_rotation_z(-stride * 0.12);
            }
            set_bend(&mut tf_q, limbs.shin_l, -(0.18 + 0.32 * (-stride).max(0.0)));
            set_bend(&mut tf_q, limbs.shin_r, -(0.18 + 0.32 * stride.max(0.0)));

            // A dragged leg doesn't stride: it trails stiff and splayed behind,
            // scraping along (the gore-trail system smears blood from it).
            if z.look.drag_leg == 0 {
                if let Ok(mut l) = tf_q.get_mut(rig.leg_l) {
                    l.translation.x = -6.0 * s;
                    l.translation.y = 6.0 * s;
                    l.rotation = Quat::from_rotation_z(0.4);
                }
                set_bend(&mut tf_q, limbs.shin_l, 0.15);
            } else if z.look.drag_leg == 1 {
                if let Ok(mut r) = tf_q.get_mut(rig.leg_r) {
                    r.translation.x = -6.0 * s;
                    r.translation.y = -6.0 * s;
                    r.rotation = Quat::from_rotation_z(-0.4);
                }
                set_bend(&mut tf_q, limbs.shin_r, -0.15);
            }

            // Arms vary per zombie AND per arm: each arm blends between a
            // fore/aft swing and an outstretched grasp by its own reach amount,
            // at the zombie's own frequency + phase, and the two arms run in
            // opposite phase — so no two shamble alike and some swing one arm
            // while clawing forward with the other.
            let f = z.frame * z.arm_freq;
            let swing_l = (f + z.arm_phase).sin() * 0.32 * z.arm_amp;
            let swing_r = (f + z.arm_phase + std::f32::consts::PI).sin() * 0.32 * z.arm_amp;
            let grasp = (z.frame * 2.4 + z.arm_phase).sin() * 0.14; // twitchy reach
            let ll = |a: f32, b: f32| a + (b - a) * z.reach_l;
            let lr = |a: f32, b: f32| a + (b - a) * z.reach_r;
            // Arms hang from under the shoulders (the torso edges) and spread
            // wider out to the sides.
            if let Ok(mut a) = tf_q.get_mut(rig.arm_l) {
                a.translation = Vec3::new(1.0 * s, 8.0 * s, 0.1);
                a.rotation = Quat::from_rotation_z(ll(0.45 + swing_l, -0.1 + grasp));
            }
            if let Ok(mut a) = tf_q.get_mut(rig.arm_r) {
                a.translation = Vec3::new(1.0 * s, -8.0 * s, 0.1);
                a.rotation = Quat::from_rotation_z(lr(-0.45 + swing_r, 0.1 - grasp));
            }
            // Elbows fold in when shambling, straighten out when reaching.
            set_bend(&mut tf_q, limbs.fore_l, ll(-0.85 - swing_l * 0.4, -0.2));
            set_bend(&mut tf_q, limbs.fore_r, lr(0.85 + swing_r * 0.4, 0.2));

            if let Ok(mut h) = tf_q.get_mut(rig.head) {
                h.translation.x = 4.0 * s;
                h.translation.y = if z.look.missing_leg >= 0 {
                    (z.frame * z.stride_rate * 2.0).cos() * 0.9 * s
                } else {
                    0.0
                };
            }
        }

        // Torn clothing flaps behind: a gentle sway, stronger while moving.
        if let Some(t) = limbs.tatters {
            let flap = (z.frame * 2.0).sin() * if moving { 0.5 } else { 0.12 };
            set_bend(&mut tf_q, t, flap);
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
