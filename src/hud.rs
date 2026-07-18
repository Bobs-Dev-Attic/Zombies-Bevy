use crate::common::*;
use crate::enemy::WaveState;
use crate::input::TouchSticks;
use crate::player::Player;
use crate::weapons::{Ammo, WeaponKind};
use crate::world::{generate_world, spawn_world_sprites, World};
use bevy::input::touch::Touches;
use bevy::prelude::*;

#[derive(Component)]
pub struct Cleanup;
#[derive(Component)]
pub struct HurtVignette;
/// Current strength of the red damage vignette (0..~0.9), decaying each frame.
#[derive(Resource, Default)]
pub struct HurtFx {
    pub intensity: f32,
}
#[derive(Component)]
pub struct MenuUi;
#[derive(Component)]
pub struct GameOverUi;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BarKind {
    Health,
    Stamina,
    Helmet,
    Armor,
}
#[derive(Component)]
pub struct Bar(pub BarKind);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GearKind {
    Helmet,
    Armor,
}
#[derive(Component)]
pub struct GearRow(pub GearKind);

#[derive(Component)]
pub struct JoyBase;
#[derive(Component)]
pub struct JoyKnob;
#[derive(Component)]
pub struct AttackBtn;
#[derive(Component)]
pub struct AmmoText;
#[derive(Component)]
pub struct WaveText;
#[derive(Component)]
pub struct FinalScoreText;

const PANEL: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);

pub fn setup_menu(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.04, 0.85)),
            MenuUi,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("ZOMBIES"),
                TextFont { font_size: 76.0, ..default() },
                TextColor(Color::srgb(0.85, 0.15, 0.12)),
            ));
            p.spawn((
                Text::new("Escape the Horde"),
                TextFont { font_size: 26.0, ..default() },
                TextColor(Color::srgb(0.8, 0.8, 0.85)),
            ));
            p.spawn((
                Text::new("Click / Tap / Press any key to begin"),
                TextFont { font_size: 20.0, ..default() },
                TextColor(Color::srgb(0.6, 0.65, 0.7)),
            ));
            p.spawn((
                Text::new(
                    "WASD move  •  Mouse aim  •  Click fire  •  Shift sprint\nR reload (auto when empty)  •  1-7 or E swap weapon  •  Mobile: on-screen stick + FIRE button",
                ),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::srgb(0.5, 0.55, 0.6)),
                Node { margin: UiRect::top(Val::Px(20.0)), ..default() },
            ));
            p.spawn((
                Text::new(format!("v{}", VERSION)),
                TextFont { font_size: 15.0, ..default() },
                TextColor(Color::srgb(0.4, 0.42, 0.48)),
                Node { margin: UiRect::top(Val::Px(24.0)), ..default() },
            ));
        });
}

pub fn teardown_menu(mut commands: Commands, q: Query<Entity, With<MenuUi>>) {
    for e in q.iter() {
        commands.entity(e).despawn();
    }
}

pub fn start_game(
    mut commands: Commands,
    art: Res<crate::art::Art>,
    mut score: ResMut<Score>,
    mut waves: ResMut<WaveState>,
    mut spawner: ResMut<crate::gear::PickupSpawner>,
) {
    *score = Score::default();
    *waves = WaveState::default();
    *spawner = crate::gear::PickupSpawner::default();

    let world = generate_world();
    spawn_world_sprites(&mut commands, &world, &art.soft);
    let spawn = world.spawn;
    // Scatter starter gear before we hand the world to the ECS as a resource.
    crate::gear::scatter_pickups(&mut commands, &art, &world, spawn);
    commands.insert_resource(world);

    // Player.
    commands.spawn((
        Player::default(),
        Transform::from_xyz(spawn.x, spawn.y, depth_z(Z_CHAR, spawn.y)),
        Visibility::default(),
        crate::art::NeedsRig,
        Cleanup,
    ));

    spawn_hud(&mut commands, &art);
}

fn spawn_hud(commands: &mut Commands, art: &crate::art::Art) {
    // Full-screen red damage vignette (transparent until the player is hit).
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        ImageNode {
            image: art.vignette.clone(),
            color: Color::srgba(0.85, 0.05, 0.05, 0.0),
            ..default()
        },
        GlobalZIndex(30),
        HurtVignette,
        Cleanup,
    ));

    // ---- On-screen touch controls (hidden until touch is used) ----
    use crate::input::{BTN_R, JOY_R, KNOB_R};
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(JOY_R * 2.0),
            height: Val::Px(JOY_R * 2.0),
            display: Display::None,
            ..default()
        },
        ImageNode {
            image: art.circle.clone(),
            color: Color::srgba(0.8, 0.85, 0.95, 0.16),
            ..default()
        },
        GlobalZIndex(60),
        JoyBase,
        Cleanup,
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(KNOB_R * 2.0),
            height: Val::Px(KNOB_R * 2.0),
            display: Display::None,
            ..default()
        },
        ImageNode {
            image: art.circle.clone(),
            color: Color::srgba(0.85, 0.9, 1.0, 0.45),
            ..default()
        },
        GlobalZIndex(61),
        JoyKnob,
        Cleanup,
    ));
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(BTN_R * 2.0),
                height: Val::Px(BTN_R * 2.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
            ImageNode {
                image: art.circle.clone(),
                color: Color::srgba(0.85, 0.25, 0.2, 0.35),
                ..default()
            },
            GlobalZIndex(60),
            AttackBtn,
            Cleanup,
        ))
        .with_children(|b| {
            b.spawn((
                Text::new("FIRE"),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::srgba(1.0, 0.95, 0.9, 0.8)),
            ));
        });

    // Health + stamina bars (top-left).
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                top: Val::Px(16.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            Cleanup,
        ))
        .with_children(|p| {
            // Health
            p.spawn((
                Node { width: Val::Px(240.0), height: Val::Px(18.0), ..default() },
                BackgroundColor(PANEL),
            ))
            .with_children(|b| {
                b.spawn((
                    Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                    BackgroundColor(Color::srgb(0.80, 0.18, 0.16)),
                    Bar(BarKind::Health),
                ));
            });
            // Stamina
            p.spawn((
                Node { width: Val::Px(240.0), height: Val::Px(10.0), ..default() },
                BackgroundColor(PANEL),
            ))
            .with_children(|b| {
                b.spawn((
                    Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                    BackgroundColor(Color::srgb(0.85, 0.75, 0.25)),
                    Bar(BarKind::Stamina),
                ));
            });
            // Helmet durability (wear-and-tear); hidden when not worn.
            p.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    display: Display::None,
                    ..default()
                },
                GearRow(GearKind::Helmet),
            ))
            .with_children(|r| {
                r.spawn((
                    Text::new("HELM"),
                    TextFont { font_size: 11.0, ..default() },
                    TextColor(Color::srgb(0.65, 0.7, 0.8)),
                ));
                r.spawn((
                    Node { width: Val::Px(200.0), height: Val::Px(8.0), ..default() },
                    BackgroundColor(PANEL),
                ))
                .with_children(|b| {
                    b.spawn((
                        Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                        BackgroundColor(Color::srgb(0.45, 0.6, 0.85)),
                        Bar(BarKind::Helmet),
                    ));
                });
            });
            // Body-armour durability; hidden when not worn.
            p.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    display: Display::None,
                    ..default()
                },
                GearRow(GearKind::Armor),
            ))
            .with_children(|r| {
                r.spawn((
                    Text::new("ARMR"),
                    TextFont { font_size: 11.0, ..default() },
                    TextColor(Color::srgb(0.6, 0.8, 0.65)),
                ));
                r.spawn((
                    Node { width: Val::Px(200.0), height: Val::Px(8.0), ..default() },
                    BackgroundColor(PANEL),
                ))
                .with_children(|b| {
                    b.spawn((
                        Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                        BackgroundColor(Color::srgb(0.35, 0.75, 0.5)),
                        Bar(BarKind::Armor),
                    ));
                });
            });
            // Weapon / ammo
            p.spawn((
                Text::new(""),
                TextFont { font_size: 20.0, ..default() },
                TextColor(Color::srgb(0.9, 0.9, 0.95)),
                AmmoText,
            ));
        });

    // Wave + score (top-right).
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            top: Val::Px(16.0),
            ..default()
        },
        Text::new(""),
        TextFont { font_size: 22.0, ..default() },
        TextColor(Color::srgb(0.9, 0.9, 0.95)),
        WaveText,
        Cleanup,
    ));

    // Version tag (bottom-right).
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(12.0),
            bottom: Val::Px(10.0),
            ..default()
        },
        Text::new(format!("v{}", VERSION)),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgba(0.7, 0.7, 0.75, 0.6)),
        Cleanup,
    ));
}

pub fn update_hud(
    player_q: Query<&Player>,
    score: Res<Score>,
    waves: Res<WaveState>,
    mut bars_q: Query<(&Bar, &mut Node, &mut BackgroundColor), Without<GearRow>>,
    mut rows_q: Query<(&GearRow, &mut Node), Without<Bar>>,
    mut ammo_q: Query<&mut Text, (With<AmmoText>, Without<WaveText>)>,
    mut wave_q: Query<&mut Text, (With<WaveText>, Without<AmmoText>)>,
) {
    let Ok(p) = player_q.single() else {
        return;
    };
    // Fills (width, and wear-based colour for gear).
    for (bar, mut node, mut bg) in bars_q.iter_mut() {
        let frac = match bar.0 {
            BarKind::Health => p.health / p.max_health,
            BarKind::Stamina => p.stamina / p.max_stamina,
            BarKind::Helmet => {
                if p.helmet_max > 0.0 { p.helmet_dura / p.helmet_max } else { 0.0 }
            }
            BarKind::Armor => {
                if p.armor_max > 0.0 { p.armor_dura / p.armor_max } else { 0.0 }
            }
        }
        .clamp(0.0, 1.0);
        node.width = Val::Percent(frac * 100.0);
        // Gear bars shift green -> amber -> red as they wear down.
        if matches!(bar.0, BarKind::Helmet | BarKind::Armor) {
            let (r, g, b) = if frac > 0.5 {
                let t = (frac - 0.5) / 0.5;
                (0.85 - 0.5 * t, 0.55 + 0.2 * t, 0.25)
            } else {
                let t = frac / 0.5;
                (0.85, 0.20 + 0.35 * t, 0.20)
            };
            bg.0 = Color::srgb(r, g, b);
        }
    }
    // Rows shown only when the gear is worn.
    for (row, mut node) in rows_q.iter_mut() {
        let show = match row.0 {
            GearKind::Helmet => p.head_gear == crate::player::HeadGear::Helmet,
            GearKind::Armor => p.body_gear == crate::player::BodyGear::Armor,
        };
        node.display = if show { Display::Flex } else { Display::None };
    }
    if let Ok(mut t) = ammo_q.single_mut() {
        let w = p.weapon();
        if w.kind == WeaponKind::Melee {
            **t = format!("{}", w.name);
        } else {
            let reserve = p.ammo_for(w.ammo);
            let clip = p.clip[p.current];
            let extra = if p.reloading > 0.0 { "  (reloading…)" } else { "" };
            **t = format!("{}  {} / {}{}", w.name, clip, reserve.min(999), extra);
        }
    }
    if let Ok(mut t) = wave_q.single_mut() {
        let phase = if waves.active {
            format!("Wave {}", waves.wave.max(1))
        } else if waves.wave == 0 {
            "Get ready…".to_string()
        } else {
            "Wave cleared!".to_string()
        };
        **t = format!("{}\nScore {}   Kills {}", phase, score.points, score.kills);
    }
    let _ = &Ammo::Rounds;
}

/// Drive the red edge vignette: spike on damage (scaled by amount), then fade.
pub fn update_hurt_fx(
    time: Res<Time>,
    mut fx: ResMut<HurtFx>,
    mut player_q: Query<&mut Player>,
    mut vig_q: Query<&mut ImageNode, With<HurtVignette>>,
) {
    let dt = time.delta_secs();
    if let Ok(mut p) = player_q.single_mut() {
        if p.hurt_amount > 0.0 {
            let add = (0.18 + p.hurt_amount / 20.0).clamp(0.0, 0.9);
            fx.intensity = fx.intensity.max(add);
            p.hurt_amount = 0.0;
        }
    }
    fx.intensity = (fx.intensity - dt * 1.8).max(0.0);
    if let Ok(mut img) = vig_q.single_mut() {
        img.color = Color::srgba(0.85, 0.05, 0.05, fx.intensity);
    }
}

/// Position and show/hide the on-screen touch controls.
pub fn update_touch_controls(
    input: Res<crate::input::InputState>,
    mut base_q: Query<&mut Node, (With<JoyBase>, Without<JoyKnob>, Without<AttackBtn>)>,
    mut knob_q: Query<&mut Node, (With<JoyKnob>, Without<JoyBase>, Without<AttackBtn>)>,
    mut btn_q: Query<(&mut Node, &mut ImageNode), (With<AttackBtn>, Without<JoyBase>, Without<JoyKnob>)>,
) {
    use crate::input::{BTN_R, JOY_R, KNOB_R};
    let show = input.touch_mode;
    let disp = if show { Display::Flex } else { Display::None };
    if let Ok(mut n) = base_q.single_mut() {
        n.display = disp;
        n.left = Val::Px(input.joy_base.x - JOY_R);
        n.top = Val::Px(input.joy_base.y - JOY_R);
    }
    if let Ok(mut n) = knob_q.single_mut() {
        n.display = disp;
        n.left = Val::Px(input.knob.x - KNOB_R);
        n.top = Val::Px(input.knob.y - KNOB_R);
    }
    if let Ok((mut n, mut img)) = btn_q.single_mut() {
        n.display = disp;
        n.left = Val::Px(input.attack_center.x - BTN_R);
        n.top = Val::Px(input.attack_center.y - BTN_R);
        let a = if input.attack_down { 0.6 } else { 0.32 };
        img.color = Color::srgba(0.9, 0.28, 0.22, a);
    }
}

pub fn setup_gameover(mut commands: Commands, score: Res<Score>) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.15, 0.0, 0.0, 0.6)),
            GameOverUi,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("YOU DIED"),
                TextFont { font_size: 72.0, ..default() },
                TextColor(Color::srgb(0.85, 0.12, 0.10)),
            ));
            p.spawn((
                Text::new(format!(
                    "Reached wave {}  •  {} kills  •  {} points",
                    score.wave.max(1),
                    score.kills,
                    score.points
                )),
                TextFont { font_size: 24.0, ..default() },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                FinalScoreText,
            ));
            p.spawn((
                Text::new("Click / Tap / Press any key to try again"),
                TextFont { font_size: 20.0, ..default() },
                TextColor(Color::srgb(0.7, 0.7, 0.75)),
            ));
        });
}

pub fn teardown_gameover(mut commands: Commands, q: Query<Entity, With<GameOverUi>>) {
    for e in q.iter() {
        commands.entity(e).despawn();
    }
}

/// Remove everything spawned for a run (world tiles, player, zombies, hud, fx).
pub fn cleanup_run(
    mut commands: Commands,
    q: Query<Entity, With<Cleanup>>,
    tiles: Query<Entity, With<crate::world::WorldTile>>,
    zombies: Query<Entity, With<crate::enemy::Zombie>>,
    projectiles: Query<Entity, With<crate::combat::Projectile>>,
    particles: Query<Entity, With<crate::combat::Particle>>,
    decals: Query<Entity, With<crate::combat::Decal>>,
    pickups: Query<Entity, With<crate::gear::Pickup>>,
) {
    for e in q
        .iter()
        .chain(tiles.iter())
        .chain(zombies.iter())
        .chain(projectiles.iter())
        .chain(particles.iter())
        .chain(decals.iter())
        .chain(pickups.iter())
    {
        commands.entity(e).try_despawn();
    }
}

pub fn check_death(player_q: Query<&Player>, mut next: ResMut<NextState<GameState>>) {
    if let Ok(p) = player_q.single() {
        if p.health <= 0.0 {
            next.set(GameState::GameOver);
        }
    }
}

/// Advance from Menu/GameOver on any input.
pub fn press_any_key(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut sticks: ResMut<TouchSticks>,
) {
    let pressed = keys.get_just_pressed().next().is_some()
        || mouse.just_pressed(MouseButton::Left)
        || touches.iter_just_pressed().next().is_some();
    if !pressed {
        return;
    }
    // Avoid the release-touch from the fire stick immediately restarting.
    *sticks = TouchSticks::default();
    match state.get() {
        GameState::Menu | GameState::GameOver => next.set(GameState::Playing),
        _ => {}
    }
}
