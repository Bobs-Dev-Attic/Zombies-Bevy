use crate::camera::MainCamera;
use bevy::input::touch::Touches;
use bevy::prelude::*;

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

    // On-screen touch controls (mobile).
    pub touch_mode: bool,     // player is using touch → show sticks, auto-aim
    pub joy_base: Vec2,       // screen-space centre of the movement stick
    pub knob: Vec2,           // screen-space position of the stick knob
    pub attack_center: Vec2,  // screen-space centre of the attack button
    pub attack_down: bool,    // attack button held (for the pressed look)
}

/// Layout constants for the on-screen controls (screen pixels).
pub const JOY_R: f32 = 70.0;
pub const KNOB_R: f32 = 30.0;
pub const BTN_R: f32 = 58.0;

/// Retained so the menu/game-over "press to start" can clear stale touch state.
#[derive(Resource, Default)]
pub struct TouchSticks;

pub fn gather_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut prev_touch: Local<bool>,
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

    let window = windows.iter().next();
    let cam = camera_q.iter().next();

    // Compute control anchors from the window each frame.
    let (w, h) = window.map(|win| (win.width(), win.height())).unwrap_or((1280.0, 720.0));
    let joy_base = Vec2::new(24.0 + JOY_R, h - 24.0 - JOY_R);
    let attack_center = Vec2::new(w - 28.0 - BTN_R, h - 28.0 - BTN_R);
    input.joy_base = joy_base;
    input.knob = joy_base;
    input.attack_center = attack_center;

    // ---- Mouse aim + fire (desktop) ----
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
    }

    // ---- On-screen touch controls ----
    // A quick tap can begin+end within a frame, so also count just-pressed.
    let mut any_touch = touches.iter_just_pressed().next().is_some();
    for t in touches.iter() {
        let p = t.position(); // screen px, origin top-left, y down
        any_touch = true;
        if p.distance(attack_center) < BTN_R + 24.0 {
            input.fire = true;
            input.attack_down = true;
        } else {
            // Movement joystick (anywhere else on screen drives it).
            let mut off = p - joy_base;
            let len = off.length();
            if len > JOY_R {
                off = off / len * JOY_R;
            }
            input.knob = joy_base + off;
            if len > 8.0 {
                let dir = Vec2::new(off.x, -off.y); // flip screen-y to world-up
                input.move_dir = dir.normalize();
                input.move_mag = (len / (JOY_R * 0.82)).clamp(0.0, 1.0);
            }
        }
    }
    if any_touch {
        input.touch_mode = true;
        *prev_touch = true;
    }
    // Once touch has ever been used this session, keep showing the controls.
    if *prev_touch {
        input.touch_mode = true;
        // Mouse aim is meaningless on touch; clear it so auto-aim takes over.
        input.have_aim = false;
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
