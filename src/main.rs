mod art;
mod camera;
mod combat;
mod common;
mod enemy;
mod hud;
mod input;
mod player;
mod weapons;
mod world;

use bevy::prelude::*;
use bevy::window::WindowResolution;

use common::*;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Zombies: Escape the Horde".into(),
                        canvas: Some("#game-canvas".into()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        resolution: WindowResolution::new(1280.0, 720.0),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(ClearColor(Color::srgb(0.06, 0.06, 0.08)))
        .init_state::<GameState>()
        .init_resource::<input::InputState>()
        .init_resource::<input::TouchSticks>()
        .init_resource::<Score>()
        .init_resource::<Shake>()
        .init_resource::<enemy::WaveState>()
        .init_resource::<combat::FireLatch>()
        .add_event::<enemy::SpitEvent>()
        .add_event::<combat::Explosion>()
        .add_systems(Startup, (camera::setup_camera, art::setup_art))
        // menu / gameover screens
        .add_systems(OnEnter(GameState::Menu), hud::setup_menu)
        .add_systems(OnExit(GameState::Menu), hud::teardown_menu)
        .add_systems(OnEnter(GameState::Playing), hud::start_game)
        .add_systems(OnEnter(GameState::GameOver), hud::setup_gameover)
        .add_systems(
            OnExit(GameState::GameOver),
            (hud::teardown_gameover, hud::cleanup_run),
        )
        .add_systems(
            Update,
            hud::press_any_key.run_if(in_state(GameState::Menu).or(in_state(GameState::GameOver))),
        )
        // rig building runs in every state so freshly-spawned characters get visuals
        .add_systems(Update, art::build_rigs)
        // gameplay
        .add_systems(
            Update,
            input::gather_input.run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            (
                player::player_update,
                combat::firing_system,
                enemy::zombie_ai,
                enemy::wave_system,
                combat::spit_system,
                combat::projectile_system,
                combat::explosion_system,
                combat::particle_system,
                combat::decal_system,
                combat::zombie_death_system,
                camera::camera_follow,
                hud::update_hud,
                hud::check_death,
            )
                .after(input::gather_input)
                .run_if(in_state(GameState::Playing)),
        )
        // animation runs after movement/AI so parts track the latest state
        .add_systems(
            Update,
            (art::animate_player, art::animate_reload_ring, art::animate_zombies)
                .after(player::player_update)
                .run_if(in_state(GameState::Playing)),
        )
        .run();
}
