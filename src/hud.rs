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
pub struct MenuUi;
#[derive(Component)]
pub struct GameOverUi;

#[derive(Component)]
pub struct HealthFill;
#[derive(Component)]
pub struct StaminaFill;
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
                    "WASD move  •  Mouse aim  •  Click fire  •  Shift sprint\nR reload  •  1-7 or E swap weapon  •  Mobile: dual touch sticks",
                ),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::srgb(0.5, 0.55, 0.6)),
                Node { margin: UiRect::top(Val::Px(20.0)), ..default() },
            ));
        });
}

pub fn teardown_menu(mut commands: Commands, q: Query<Entity, With<MenuUi>>) {
    for e in q.iter() {
        commands.entity(e).despawn();
    }
}

pub fn start_game(mut commands: Commands, mut score: ResMut<Score>, mut waves: ResMut<WaveState>) {
    *score = Score::default();
    *waves = WaveState::default();

    let world = generate_world();
    spawn_world_sprites_tagged(&mut commands, &world);
    let spawn = world.spawn;
    commands.insert_resource(world);

    // Player.
    commands.spawn((
        Player::default(),
        Transform::from_xyz(spawn.x, spawn.y, depth_z(Z_CHAR, spawn.y)),
        Visibility::default(),
        crate::art::NeedsRig,
        Cleanup,
    ));

    spawn_hud(&mut commands);
}

fn spawn_world_sprites_tagged(commands: &mut Commands, world: &World) {
    // Wrap world sprites with the Cleanup tag by spawning then... simplest: spawn
    // via the world module, then the tiles carry WorldTile — tag those. We instead
    // re-implement by tagging at spawn time here.
    spawn_world_sprites(commands, world);
}

fn spawn_hud(commands: &mut Commands) {
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
                    HealthFill,
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
                    StaminaFill,
                ));
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
}

pub fn update_hud(
    player_q: Query<&Player>,
    score: Res<Score>,
    waves: Res<WaveState>,
    mut health_q: Query<&mut Node, (With<HealthFill>, Without<StaminaFill>)>,
    mut stamina_q: Query<&mut Node, (With<StaminaFill>, Without<HealthFill>)>,
    mut ammo_q: Query<&mut Text, (With<AmmoText>, Without<WaveText>)>,
    mut wave_q: Query<&mut Text, (With<WaveText>, Without<AmmoText>)>,
) {
    let Ok(p) = player_q.single() else {
        return;
    };
    if let Ok(mut n) = health_q.single_mut() {
        n.width = Val::Percent((p.health / p.max_health * 100.0).clamp(0.0, 100.0));
    }
    if let Ok(mut n) = stamina_q.single_mut() {
        n.width = Val::Percent((p.stamina / p.max_stamina * 100.0).clamp(0.0, 100.0));
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
) {
    for e in q.iter().chain(tiles.iter()).chain(zombies.iter()).chain(projectiles.iter()).chain(particles.iter()).chain(decals.iter()) {
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
