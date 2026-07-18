mod ambient;
mod art;
mod camera;
mod combat;
mod common;
mod enemy;
mod gear;
mod hud;
mod input;
mod nav;
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
        .init_resource::<gear::PickupSpawner>()
        .init_resource::<hud::HurtFx>()
        .init_resource::<hud::Concussion>()
        .init_resource::<nav::Objective>()
        .init_resource::<Settings>()
        .add_event::<enemy::SpitEvent>()
        .add_event::<combat::Explosion>()
        .add_systems(Startup, (camera::setup_camera, art::setup_art))
        // menu / options / gameover screens
        .add_systems(OnEnter(GameState::Menu), hud::setup_menu)
        .add_systems(OnExit(GameState::Menu), hud::teardown_menu)
        .add_systems(OnEnter(GameState::Options), hud::setup_options)
        .add_systems(OnExit(GameState::Options), hud::teardown_options)
        .add_systems(OnEnter(GameState::Playing), hud::start_game)
        .add_systems(OnEnter(GameState::GameOver), hud::setup_gameover)
        .add_systems(
            OnExit(GameState::GameOver),
            (hud::teardown_gameover, hud::cleanup_run),
        )
        .add_systems(
            Update,
            hud::menu_buttons.run_if(in_state(GameState::Menu).or(in_state(GameState::Options))),
        )
        .add_systems(
            Update,
            (hud::options_slider, hud::options_cheats).run_if(in_state(GameState::Options)),
        )
        .add_systems(
            Update,
            hud::press_any_key.run_if(in_state(GameState::GameOver)),
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
                (
                    player::player_update,
                    player::apply_cheats.after(player::player_update),
                ),
                combat::firing_system,
                (
                    enemy::zombie_ai,
                    enemy::zombie_separation.after(enemy::zombie_ai),
                    enemy::zombie_gore_trail.after(enemy::zombie_ai),
                ),
                enemy::wave_system,
                combat::spit_system,
                (
                    combat::projectile_system,
                    combat::zombie_disfigure.after(combat::projectile_system),
                ),
                combat::explosion_system,
                combat::particle_system,
                combat::decal_system,
                combat::zombie_death_system,
                combat::muzzle_flash_system,
                combat::reload_fx,
                gear::pickup_collect,
                gear::pickup_spawn_over_time,
                gear::pickup_icon_bob,
                camera::camera_follow,
                hud::update_hud,
                (hud::update_hurt_fx, hud::update_concussion),
                hud::update_touch_controls,
                hud::check_death,
            )
                .after(input::gather_input)
                .run_if(in_state(GameState::Playing)),
        )
        // animation runs after movement/AI so parts track the latest state
        .add_systems(
            Update,
            (
                player::touch_autoaim,
                art::animate_player,
                art::animate_reload_ring,
                art::animate_zombies,
                art::update_gear_visuals,
                nav::objective_system,
                nav::minimap_system,
            )
                .after(player::player_update)
                .run_if(in_state(GameState::Playing)),
        )
        // ambient atmosphere: flies, twitching corpses, feeding crows
        .add_systems(
            Update,
            (
                ambient::fly_system,
                ambient::twitch_system,
                ambient::crow_system,
            )
                .run_if(in_state(GameState::Playing)),
        )
        .run();
}
