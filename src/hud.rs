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
/// Concussion from a close blast: drives a disorienting full-screen haze +
/// screen shake while it decays. Set by the explosion system.
#[derive(Resource, Default)]
pub struct Concussion {
    pub intensity: f32,
}
#[derive(Component)]
pub struct ConcussionVeil;
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
pub struct SwapBtn;
#[derive(Component)]
pub struct AmmoText;
#[derive(Component)]
pub struct WaveText;
#[derive(Component)]
pub struct FinalScoreText;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum MenuButton {
    Play,
    Options,
    Back,
    CycleScene,
}

/// The text inside the scenario-select button, rewritten as it cycles.
#[derive(Component)]
pub struct SceneChoiceText;
#[derive(Component)]
pub struct OptionsUi;
#[derive(Component)]
pub struct AimSliderTrack;
#[derive(Component)]
pub struct AimSliderFill;
#[derive(Component)]
pub struct AimSliderHandle;
#[derive(Component)]
pub struct AimValueText;
/// A Game-Settings toggle button and the label text inside it.
#[derive(Component)]
pub struct CheatButton(pub Cheat);
#[derive(Component)]
pub struct CheatStateText(pub Cheat);

const PANEL: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);
const BTN_BG: Color = Color::srgb(0.16, 0.17, 0.22);
const BTN_BG_HOVER: Color = Color::srgb(0.26, 0.28, 0.36);

fn menu_button(parent: &mut ChildSpawnerCommands, label: &str, action: MenuButton) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(240.0),
                height: Val::Px(52.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(BTN_BG),
            BorderRadius::all(Val::Px(8.0)),
            action,
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(label),
                TextFont { font_size: 24.0, ..default() },
                TextColor(Color::srgb(0.9, 0.9, 0.95)),
            ));
        });
}

fn scene_label(c: Option<crate::world::Scene>) -> String {
    match c {
        None => "SCENARIO:  Random".to_string(),
        Some(s) => format!("SCENARIO:  {}", s.label()),
    }
}

pub fn setup_menu(mut commands: Commands, choice: Res<crate::world::SceneChoice>) {
    let scene_text = scene_label(choice.0);
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
            p.spawn((Node { margin: UiRect::top(Val::Px(8.0)), ..default() },));
            menu_button(p, "PLAY", MenuButton::Play);
            menu_button(p, "OPTIONS", MenuButton::Options);
            // Scenario selector — cycles Random / Streets / Park / Neighborhood.
            p.spawn((
                Button,
                Node {
                    width: Val::Px(300.0),
                    height: Val::Px(44.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(BTN_BG),
                BorderRadius::all(Val::Px(8.0)),
                MenuButton::CycleScene,
            ))
            .with_children(|b| {
                b.spawn((
                    Text::new(scene_text.clone()),
                    TextFont { font_size: 19.0, ..default() },
                    TextColor(Color::srgb(0.75, 0.78, 0.85)),
                    SceneChoiceText,
                ));
            });
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
                Node { margin: UiRect::top(Val::Px(16.0)), ..default() },
            ));
        });
}

pub fn teardown_menu(mut commands: Commands, q: Query<Entity, With<MenuUi>>) {
    for e in q.iter() {
        commands.entity(e).despawn();
    }
}

#[derive(Component)]
pub struct AimAdjust(pub f32);

const SLIDER_W: f32 = 300.0;

pub fn setup_options(mut commands: Commands, settings: Res<Settings>) {
    let frac = settings.aim_assist.clamp(0.0, 1.0);
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(18.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.04, 0.9)),
            OptionsUi,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("OPTIONS"),
                TextFont { font_size: 52.0, ..default() },
                TextColor(Color::srgb(0.85, 0.85, 0.9)),
            ));
            p.spawn((
                Text::new("Aim Assist accuracy"),
                TextFont { font_size: 22.0, ..default() },
                TextColor(Color::srgb(0.75, 0.8, 0.85)),
            ));
            p.spawn((
                Text::new(format!("{}%", (frac * 100.0).round() as i32)),
                TextFont { font_size: 26.0, ..default() },
                TextColor(Color::srgb(0.5, 0.85, 0.6)),
                AimValueText,
            ));
            // [ - ]  [ ==== slider ==== ]  [ + ]
            p.spawn((Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(14.0),
                ..default()
            },))
                .with_children(|row| {
                    small_button(row, "-", AimAdjust(-0.1));
                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(SLIDER_W),
                            height: Val::Px(18.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.18, 0.19, 0.24)),
                        BorderRadius::all(Val::Px(9.0)),
                        bevy::ui::RelativeCursorPosition::default(),
                        AimSliderTrack,
                    ))
                    .with_children(|t| {
                        t.spawn((
                            Node {
                                width: Val::Percent(frac * 100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.35, 0.75, 0.5)),
                            BorderRadius::all(Val::Px(9.0)),
                            AimSliderFill,
                        ));
                        t.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(frac * SLIDER_W - 9.0),
                                top: Val::Px(-6.0),
                                width: Val::Px(18.0),
                                height: Val::Px(30.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.92, 0.95, 1.0)),
                            BorderRadius::all(Val::Px(5.0)),
                            AimSliderHandle,
                        ));
                    });
                    small_button(row, "+", AimAdjust(0.1));
                });
            p.spawn((
                Text::new("Drag the slider, tap - / +, or use Left / Right\n0% = manual aiming    100% = instant lock-on"),
                TextFont { font_size: 15.0, ..default() },
                TextColor(Color::srgb(0.5, 0.55, 0.6)),
                Node { margin: UiRect::top(Val::Px(6.0)), ..default() },
            ));
            // ---- Game Settings (cheat toggles) ----
            p.spawn((
                Text::new("GAME SETTINGS"),
                TextFont { font_size: 26.0, ..default() },
                TextColor(Color::srgb(0.8, 0.8, 0.85)),
                Node { margin: UiRect::top(Val::Px(18.0)), ..default() },
            ));
            for c in CHEATS {
                cheat_row(p, c, settings.cheat(c));
            }
            p.spawn((Node { margin: UiRect::top(Val::Px(6.0)), ..default() },));
            menu_button(p, "BACK", MenuButton::Back);
        });
}

const CHEAT_ON: Color = Color::srgb(0.20, 0.52, 0.30);
const CHEAT_OFF: Color = Color::srgb(0.16, 0.17, 0.22);

fn cheat_row(parent: &mut ChildSpawnerCommands, cheat: Cheat, on: bool) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(430.0),
                height: Val::Px(44.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::horizontal(Val::Px(16.0)),
                ..default()
            },
            BackgroundColor(if on { CHEAT_ON } else { CHEAT_OFF }),
            BorderRadius::all(Val::Px(8.0)),
            CheatButton(cheat),
        ))
        .with_children(|b| {
            // Left: name + hint.
            b.spawn((Node {
                flex_direction: FlexDirection::Column,
                ..default()
            },))
                .with_children(|col| {
                    col.spawn((
                        Text::new(cheat.label()),
                        TextFont { font_size: 20.0, ..default() },
                        TextColor(Color::srgb(0.92, 0.94, 0.98)),
                    ));
                    col.spawn((
                        Text::new(cheat.hint()),
                        TextFont { font_size: 13.0, ..default() },
                        TextColor(Color::srgb(0.62, 0.66, 0.72)),
                    ));
                });
            // Right: ON / OFF state pill.
            b.spawn((
                Text::new(if on { "ON" } else { "OFF" }),
                TextFont { font_size: 20.0, ..default() },
                TextColor(if on {
                    Color::srgb(0.6, 1.0, 0.72)
                } else {
                    Color::srgb(0.55, 0.58, 0.64)
                }),
                CheatStateText(cheat),
            ));
        });
}

/// Toggle Game-Settings cheats when their rows are clicked, and keep each row's
/// colour + ON/OFF label in sync with the setting.
pub fn options_cheats(
    mut settings: ResMut<Settings>,
    mut btn_q: Query<(&Interaction, &CheatButton, &mut BackgroundColor), Changed<Interaction>>,
    mut label_q: Query<(&CheatStateText, &mut Text, &mut TextColor)>,
) {
    for (interaction, btn, mut bg) in btn_q.iter_mut() {
        if *interaction == Interaction::Pressed {
            settings.toggle_cheat(btn.0);
        }
        let on = settings.cheat(btn.0);
        bg.0 = match (*interaction, on) {
            (Interaction::Hovered, true) => Color::srgb(0.26, 0.62, 0.38),
            (Interaction::Hovered, false) => BTN_BG_HOVER,
            (_, true) => CHEAT_ON,
            (_, false) => CHEAT_OFF,
        };
    }
    // Refresh the ON/OFF text for every row (cheap; few rows).
    for (tag, mut text, mut color) in label_q.iter_mut() {
        let on = settings.cheat(tag.0);
        **text = (if on { "ON" } else { "OFF" }).to_string();
        color.0 = if on {
            Color::srgb(0.6, 1.0, 0.72)
        } else {
            Color::srgb(0.55, 0.58, 0.64)
        };
    }
}

fn small_button(parent: &mut ChildSpawnerCommands, label: &str, adjust: AimAdjust) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(44.0),
                height: Val::Px(44.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(BTN_BG),
            BorderRadius::all(Val::Px(8.0)),
            adjust,
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(label),
                TextFont { font_size: 28.0, ..default() },
                TextColor(Color::srgb(0.9, 0.9, 0.95)),
            ));
        });
}

pub fn teardown_options(mut commands: Commands, q: Query<Entity, With<OptionsUi>>) {
    for e in q.iter() {
        commands.entity(e).despawn();
    }
}

/// Handle PLAY / OPTIONS / BACK buttons (Menu + Options states).
pub fn menu_buttons(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut choice: ResMut<crate::world::SceneChoice>,
    mut scene_txt: Query<&mut Text, With<SceneChoiceText>>,
    mut q: Query<(&Interaction, &MenuButton, &mut BackgroundColor), Changed<Interaction>>,
) {
    for (interaction, action, mut bg) in q.iter_mut() {
        if *interaction == Interaction::Pressed {
            match action {
                MenuButton::Play => next.set(GameState::Playing),
                MenuButton::Options => next.set(GameState::Options),
                MenuButton::Back => next.set(GameState::Menu),
                MenuButton::CycleScene => {
                    use crate::world::Scene;
                    // Random → Streets → Park → Neighborhood → Random …
                    choice.0 = match choice.0 {
                        None => Some(Scene::Streets),
                        Some(Scene::Streets) => Some(Scene::Park),
                        Some(Scene::Park) => Some(Scene::Neighborhood),
                        Some(Scene::Neighborhood) => None,
                    };
                    if let Ok(mut t) = scene_txt.single_mut() {
                        **t = scene_label(choice.0);
                    }
                }
            }
        }
        bg.0 = match *interaction {
            Interaction::Hovered | Interaction::Pressed => BTN_BG_HOVER,
            Interaction::None => BTN_BG,
        };
    }
    // Keyboard shortcuts.
    match state.get() {
        GameState::Menu => {
            if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
                next.set(GameState::Playing);
            }
            if keys.just_pressed(KeyCode::KeyO) {
                next.set(GameState::Options);
            }
        }
        GameState::Options => {
            if keys.just_pressed(KeyCode::Escape) {
                next.set(GameState::Menu);
            }
        }
        _ => {}
    }
}

/// Drag / adjust the aim-assist slider and reflect it in the UI.
pub fn options_slider(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<Settings>,
    track_q: Query<(&Interaction, &bevy::ui::RelativeCursorPosition), With<AimSliderTrack>>,
    adjust_q: Query<(&Interaction, &AimAdjust), Changed<Interaction>>,
    mut fill_q: Query<&mut Node, (With<AimSliderFill>, Without<AimSliderHandle>)>,
    mut handle_q: Query<&mut Node, (With<AimSliderHandle>, Without<AimSliderFill>)>,
    mut value_q: Query<&mut Text, With<AimValueText>>,
) {
    let mut v = settings.aim_assist;
    // Drag: while the track is pressed, take the cursor's normalized x.
    for (interaction, rel) in track_q.iter() {
        if *interaction == Interaction::Pressed {
            if let Some(n) = rel.normalized {
                v = n.x.clamp(0.0, 1.0);
            }
        }
    }
    // − / + buttons.
    for (interaction, adj) in adjust_q.iter() {
        if *interaction == Interaction::Pressed {
            v = (v + adj.0).clamp(0.0, 1.0);
        }
    }
    // Arrow keys.
    if keys.just_pressed(KeyCode::ArrowLeft) {
        v = (v - 0.05).clamp(0.0, 1.0);
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        v = (v + 0.05).clamp(0.0, 1.0);
    }

    if (v - settings.aim_assist).abs() > 0.0001 {
        settings.aim_assist = v;
    }
    let frac = settings.aim_assist.clamp(0.0, 1.0);
    if let Ok(mut n) = fill_q.single_mut() {
        n.width = Val::Percent(frac * 100.0);
    }
    if let Ok(mut n) = handle_q.single_mut() {
        n.left = Val::Px(frac * SLIDER_W - 9.0);
    }
    if let Ok(mut t) = value_q.single_mut() {
        **t = format!("{}%", (frac * 100.0).round() as i32);
    }
}

pub fn start_game(
    mut commands: Commands,
    art: Res<crate::art::Art>,
    mut images: ResMut<Assets<Image>>,
    mut obj: ResMut<crate::nav::Objective>,
    mut score: ResMut<Score>,
    mut waves: ResMut<WaveState>,
    mut spawner: ResMut<crate::gear::PickupSpawner>,
    choice: Res<crate::world::SceneChoice>,
) {
    *score = Score::default();
    *waves = WaveState::default();
    *spawner = crate::gear::PickupSpawner::default();

    let world = generate_world(choice.0);
    spawn_world_sprites(&mut commands, &world, &art.soft);
    crate::world::spawn_props(&mut commands, &world, &art);
    let spawn = world.spawn;
    // Scatter starter gear before we hand the world to the ECS as a resource.
    crate::gear::scatter_pickups(&mut commands, &art, &world, spawn);
    // Dress the arena: debris, blood pools, corpses, flies, crows.
    crate::ambient::scatter_ambient(&mut commands, &art, &world, spawn);
    // Objective, direction arrow and minimap (needs the world in hand).
    crate::nav::build_nav(&mut commands, &mut images, &world, &mut obj, spawn);
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
    // Fog of war: a dark radial vignette that keeps only the area around the
    // player lit, so the streets fade into gloom at the edges of sight.
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
            color: Color::srgba(0.01, 0.01, 0.02, 0.92),
            ..default()
        },
        GlobalZIndex(25),
        Cleanup,
    ));

    // Concussion haze: a soft, cloudy full-screen veil that clouds in when a blast
    // goes off in your face, then clears as you come round. Uses the soft radial
    // gradient (not a hard disc) so it reads as a hazy blur, not pixels.
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
            image: art.soft.clone(),
            color: Color::srgba(0.82, 0.84, 0.9, 0.0),
            ..default()
        },
        GlobalZIndex(40),
        ConcussionVeil,
        Cleanup,
    ));

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
    use crate::input::{BTN_R, JOY_R, KNOB_R, SWAP_R};
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
    // Weapon-swap button (just above FIRE).
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(SWAP_R * 2.0),
                height: Val::Px(SWAP_R * 2.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
            ImageNode {
                image: art.circle.clone(),
                color: Color::srgba(0.35, 0.55, 0.85, 0.35),
                ..default()
            },
            GlobalZIndex(60),
            SwapBtn,
            Cleanup,
        ))
        .with_children(|b| {
            b.spawn((
                Text::new("SWAP"),
                TextFont { font_size: 15.0, ..default() },
                TextColor(Color::srgba(0.95, 0.97, 1.0, 0.85)),
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
    world: Option<Res<crate::world::World>>,
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
        let nades = format!("\nGrenades  {}  [G]", p.grenades);
        if w.kind == WeaponKind::Melee {
            **t = format!("{}{}", w.name, nades);
        } else {
            let reserve = p.ammo_for(w.ammo);
            let clip = p.clip[p.current];
            let extra = if p.reloading > 0.0 { "  (reloading…)" } else { "" };
            **t = format!("{}  {} / {}{}{}", w.name, clip, reserve.min(999), extra, nades);
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
        let scene = world.as_ref().map(|w| w.scene.label()).unwrap_or("");
        **t = format!(
            "{}   {}\nScore {}   Kills {}",
            phase, scene, score.points, score.kills
        );
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

/// Drive the concussion haze: a jittering washed-out veil plus screen shake
/// while the player is dazed by a close blast, fading as they recover.
pub fn update_concussion(
    time: Res<Time>,
    mut conc: ResMut<Concussion>,
    mut shake: ResMut<Shake>,
    player_q: Query<&crate::player::Player>,
    mut veil_q: Query<&mut ImageNode, With<ConcussionVeil>>,
) {
    let dt = time.delta_secs();
    // While knocked out, keep the haze pinned high; otherwise let it fade.
    if let Ok(p) = player_q.single() {
        if p.stun > 0.0 {
            conc.intensity = conc.intensity.max((p.stun / 1.4).clamp(0.3, 1.0));
        }
    }
    conc.intensity = (conc.intensity - dt * 0.9).max(0.0);
    if conc.intensity > 0.001 {
        shake.add(conc.intensity * dt * 6.0);
    }
    if let Ok(mut img) = veil_q.single_mut() {
        // A slow, cloudy swell (not a hard flicker) so it feels like a woozy blur
        // closing in rather than visual static.
        let t = time.elapsed_secs();
        let swell = 0.82 + 0.18 * (t * 2.3).sin() + 0.06 * (t * 5.7).sin();
        let a = (conc.intensity * 0.9 * swell).clamp(0.0, 0.92);
        img.color = Color::srgba(0.82, 0.84, 0.9, a);
    }
}

/// Position and show/hide the on-screen touch controls.
pub fn update_touch_controls(
    input: Res<crate::input::InputState>,
    mut base_q: Query<&mut Node, (With<JoyBase>, Without<JoyKnob>, Without<AttackBtn>, Without<SwapBtn>)>,
    mut knob_q: Query<&mut Node, (With<JoyKnob>, Without<JoyBase>, Without<AttackBtn>, Without<SwapBtn>)>,
    mut btn_q: Query<(&mut Node, &mut ImageNode), (With<AttackBtn>, Without<JoyBase>, Without<JoyKnob>, Without<SwapBtn>)>,
    mut swap_q: Query<(&mut Node, &mut ImageNode), (With<SwapBtn>, Without<JoyBase>, Without<JoyKnob>, Without<AttackBtn>)>,
) {
    use crate::input::{BTN_R, JOY_R, KNOB_R, SWAP_R};
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
    if let Ok((mut n, mut img)) = swap_q.single_mut() {
        n.display = disp;
        n.left = Val::Px(input.swap_center.x - SWAP_R);
        n.top = Val::Px(input.swap_center.y - SWAP_R);
        let a = if input.swap_down { 0.65 } else { 0.35 };
        img.color = Color::srgba(0.35, 0.55, 0.85, a);
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
