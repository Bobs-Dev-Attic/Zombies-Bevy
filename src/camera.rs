use crate::common::*;
use crate::player::Player;
use bevy::input::touch::Touches;
use bevy::prelude::*;
use rand::Rng;

/// Base orthographic scale at zoom level 1.0 (smaller = closer).
const BASE_SCALE: f32 = 0.62;

/// Zoom the camera: pinch with two fingers on touch, or the -/= keys on desktop.
pub fn zoom_control(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    touches: Res<Touches>,
    mut zoom: ResMut<Zoom>,
    mut prev_pinch: Local<f32>,
    mut proj_q: Query<&mut Projection, With<MainCamera>>,
) {
    let dt = time.delta_secs();
    if keys.pressed(KeyCode::Minus) || keys.pressed(KeyCode::NumpadSubtract) {
        zoom.level -= dt * 1.3;
    }
    if keys.pressed(KeyCode::Equal) || keys.pressed(KeyCode::NumpadAdd) {
        zoom.level += dt * 1.3;
    }
    // Pinch-to-zoom: change in the gap between two fingers scales the zoom.
    let pts: Vec<Vec2> = touches.iter().map(|t| t.position()).collect();
    if pts.len() >= 2 {
        let d = pts[0].distance(pts[1]);
        if *prev_pinch > 1.0 {
            zoom.level *= 1.0 + (d - *prev_pinch) * 0.004;
        }
        *prev_pinch = d;
    } else {
        *prev_pinch = 0.0;
    }
    zoom.level = zoom.level.clamp(0.6, 2.4);
    if let Ok(mut proj) = proj_q.single_mut() {
        if let Projection::Orthographic(o) = &mut *proj {
            o.scale = BASE_SCALE / zoom.level;
        }
    }
}

#[derive(Component)]
pub struct MainCamera;

pub fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::from(OrthographicProjection {
            scale: 0.62,
            ..OrthographicProjection::default_2d()
        }),
        MainCamera,
    ));
}

pub fn camera_follow(
    time: Res<Time>,
    mut shake: ResMut<Shake>,
    player_q: Query<&Transform, (With<Player>, Without<MainCamera>)>,
    mut cam_q: Query<&mut Transform, With<MainCamera>>,
) {
    let Ok(ptf) = player_q.single() else {
        return;
    };
    let Ok(mut cam) = cam_q.single_mut() else {
        return;
    };
    let dt = time.delta_secs();
    let target = ptf.translation.truncate();
    let cur = cam.translation.truncate();
    let smoothed = cur + (target - cur) * (dt * 6.0).clamp(0.0, 1.0);

    // Screen shake from trauma (quadratic falloff).
    let mut rng = rand::thread_rng();
    let s = shake.trauma * shake.trauma;
    let offset = Vec2::new(
        rng.gen_range(-1.0..1.0) * s * 14.0,
        rng.gen_range(-1.0..1.0) * s * 14.0,
    );
    shake.trauma = (shake.trauma - dt * 1.6).max(0.0);

    cam.translation.x = smoothed.x + offset.x;
    cam.translation.y = smoothed.y + offset.y;
    // Keep the 2D camera at z=0 so the ortho near/far window (-1000..1000)
    // spans all our sprite layers; moving it in z would clip the floor.
    cam.translation.z = 0.0;
}
