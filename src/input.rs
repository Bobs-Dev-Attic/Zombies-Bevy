use bevy::prelude::*;
use bevy::input::touch::Touches;
use crate::camera::MainCamera;

/// Distilled per-frame intent, filled from keyboard+mouse or touch.
#[derive(Resource, Default)]
pub struct InputState {
    pub move_dir: Vec2, // unit-ish direction
    pub move_mag: f32,  // 0..1 (1 == sprint)
    pub aim_world: Vec2,
    pub have_aim: bool,
    pub fire: bool,
    pub reload: bool,
    pub throw: bool,
    pub next_weapon: bool,
    pub prev_weapon: bool,
    pub weapon_slot: Option<usize>,
}

/// State for the on-screen virtual sticks (mobile).
#[derive(Resource, Default)]
pub struct TouchSticks {
    pub move_id: Option<u64>,
    pub move_origin: Vec2,
    pub move_cur: Vec2,
    pub aim_id: Option<u64>,
    pub aim_origin: Vec2,
    pub aim_cur: Vec2,
}

pub fn gather_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut sticks: ResMut<TouchSticks>,
    mut input: ResMut<InputState>,
) {
    *input = InputState::default();

    // ---- Keyboard movement ----
    let mut kb = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        kb.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        kb.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        kb.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        kb.x += 1.0;
    }
    if kb != Vec2::ZERO {
        input.move_dir = kb.normalize();
        input.move_mag = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            1.0
        } else {
            0.7
        };
    }

    // ---- Mouse aim + fire ----
    let window = windows.iter().next();
    let cam = camera_q.iter().next();
    if let (Some(window), Some((camera, cam_tf))) = (window, cam) {
        if let Some(cursor) = window.cursor_position() {
            if let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) {
                input.aim_world = world;
                input.have_aim = true;
            }
        }
        if mouse_buttons.pressed(MouseButton::Left) {
            input.fire = true;
        }

        // ---- Touch sticks ----
        let w = window.width();
        // Assign new touches to a stick based on which half they started in.
        for t in touches.iter_just_pressed() {
            let p = t.position();
            if p.x < w * 0.5 && sticks.move_id.is_none() {
                sticks.move_id = Some(t.id());
                sticks.move_origin = p;
                sticks.move_cur = p;
            } else if sticks.aim_id.is_none() {
                sticks.aim_id = Some(t.id());
                sticks.aim_origin = p;
                sticks.aim_cur = p;
            }
        }
        for t in touches.iter() {
            if Some(t.id()) == sticks.move_id {
                sticks.move_cur = t.position();
            } else if Some(t.id()) == sticks.aim_id {
                sticks.aim_cur = t.position();
            }
        }
        for t in touches.iter_just_released() {
            if Some(t.id()) == sticks.move_id {
                sticks.move_id = None;
            }
            if Some(t.id()) == sticks.aim_id {
                sticks.aim_id = None;
            }
        }

        // Movement from left stick.
        if sticks.move_id.is_some() {
            let mut d = sticks.move_cur - sticks.move_origin;
            d.y = -d.y; // screen y is down
            let len = d.length();
            if len > 6.0 {
                input.move_dir = d / len;
                input.move_mag = (len / 55.0).clamp(0.0, 1.0);
            }
        }
        // Aim + fire from right stick.
        if sticks.aim_id.is_some() {
            if let Ok(world) = camera.viewport_to_world_2d(cam_tf, sticks.aim_cur) {
                input.aim_world = world;
                input.have_aim = true;
                input.fire = true;
            }
        }
    }

    // ---- Discrete keys ----
    input.reload = keys.just_pressed(KeyCode::KeyR);
    input.throw = keys.just_pressed(KeyCode::KeyG) || keys.just_pressed(KeyCode::KeyF);
    input.next_weapon = keys.just_pressed(KeyCode::KeyE) || keys.just_pressed(KeyCode::KeyQ);
    for (i, k) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
    ]
    .iter()
    .enumerate()
    {
        if keys.just_pressed(*k) {
            input.weapon_slot = Some(i);
        }
    }
}
