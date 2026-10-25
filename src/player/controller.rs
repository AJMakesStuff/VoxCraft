use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    render::view::{ColorGrading, Msaa},
    window::{CursorGrabMode, CursorOptions},
};

use crate::{
    game::{GameSettings, GameState, OptionsReturn},
    world::{WorldData, terrain_height},
};

const EYE_HEIGHT: f32 = 1.7;
const PLAYER_RADIUS: f32 = 0.30;
const PLAYER_HEADROOM: f32 = 0.18;
const FLY_TAP_WINDOW_SECS: f32 = 0.35;
const WALK_SPEED: f32 = 5.0;
const SPRINT_SPEED: f32 = 8.0;
const FLY_SPRINT_SPEED: f32 = 12.0;

#[derive(Component)]
pub struct Player;

#[derive(Component, Default)]
pub struct PlayerVelocity(pub Vec3);

#[derive(Resource, Default)]
pub struct PendingPlayerRestore(pub Option<([f32; 3], [f32; 4])>);

#[derive(Component, Default)]
struct PlayerLook {
    yaw: f32,
    pitch: f32,
    grounded: bool,
    flying: bool,
    last_space_press: Option<f32>,
}

#[derive(Component)]
struct PlayerSpawnSeed(u32);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(OnEnter(GameState::MainMenu), reset_player)
            .init_resource::<PendingPlayerRestore>()
            .add_systems(
                OnEnter(GameState::Playing),
                (reset_player_for_new_world_seed, restore_loaded_player).chain(),
            )
            .add_systems(
                Update,
                (game_shortcuts, capture_mouse, look, movement).chain(),
            );
    }
}

fn restore_loaded_player(
    mut pending: ResMut<PendingPlayerRestore>,
    mut player: Query<(&mut Transform, &mut PlayerVelocity, &mut PlayerLook), With<Player>>,
) {
    let Some((position, rotation)) = pending.0.take() else {
        return;
    };
    let Ok((mut transform, mut velocity, mut look)) = player.single_mut() else {
        return;
    };

    transform.translation = Vec3::from_array(position);
    let rotation = Quat::from_array(rotation);
    let scale = rotation
        .to_array()
        .into_iter()
        .map(f32::abs)
        .fold(0.0, f32::max);
    transform.rotation = if scale == 0.0 {
        Quat::IDENTITY
    } else {
        (rotation / scale).normalize()
    };
    let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
    look.yaw = yaw;
    look.pitch = pitch;
    look.grounded = false;
    look.flying = false;
    look.last_space_press = None;
    velocity.0 = Vec3::ZERO;
}

pub fn player_forward(transform: &Transform) -> Vec3 {
    let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
    Vec3::new(
        -yaw.sin() * pitch.cos(),
        pitch.sin(),
        -yaw.cos() * pitch.cos(),
    )
    .normalize_or_zero()
}

fn spawn_player(mut commands: Commands, settings: Res<crate::game::GameSettings>) {
    let ground = terrain_height(settings.seed, 0, 0) as f32;
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: settings.field_of_view_degrees.to_radians(),
            ..default()
        }),
        ColorGrading::default(),
        Msaa::from_samples(settings.anti_aliasing_samples),
        Transform::from_xyz(0.0, ground + 1.0 + EYE_HEIGHT, 0.0),
        Player,
        PlayerVelocity::default(),
        PlayerLook::default(),
        PlayerSpawnSeed(settings.seed),
    ));
}

fn reset_player(
    settings: Res<GameSettings>,
    mut player: Query<
        (
            &mut Transform,
            &mut PlayerVelocity,
            &mut PlayerLook,
            &mut PlayerSpawnSeed,
        ),
        With<Player>,
    >,
) {
    let Ok((mut transform, mut velocity, mut look, mut spawn_seed)) = player.single_mut() else {
        return;
    };
    reset_player_state(
        settings.seed,
        &mut transform,
        &mut velocity,
        &mut look,
        &mut spawn_seed,
    );
}

fn reset_player_for_new_world_seed(
    settings: Res<GameSettings>,
    mut player: Query<
        (
            &mut Transform,
            &mut PlayerVelocity,
            &mut PlayerLook,
            &mut PlayerSpawnSeed,
        ),
        With<Player>,
    >,
) {
    let Ok((mut transform, mut velocity, mut look, mut spawn_seed)) = player.single_mut() else {
        return;
    };
    if spawn_seed.0 == settings.seed {
        return;
    }
    reset_player_state(
        settings.seed,
        &mut transform,
        &mut velocity,
        &mut look,
        &mut spawn_seed,
    );
}

fn reset_player_state(
    seed: u32,
    transform: &mut Transform,
    velocity: &mut PlayerVelocity,
    look: &mut PlayerLook,
    spawn_seed: &mut PlayerSpawnSeed,
) {
    let ground = terrain_height(seed, 0, 0) as f32;
    transform.translation = Vec3::new(0.0, ground + 1.0 + EYE_HEIGHT, 0.0);
    transform.rotation = Quat::IDENTITY;
    velocity.0 = Vec3::ZERO;
    look.yaw = 0.0;
    look.pitch = 0.0;
    look.grounded = false;
    look.flying = false;
    look.last_space_press = None;
    spawn_seed.0 = seed;
}

fn game_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    options_return: Res<OptionsReturn>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        match *state.get() {
            GameState::Playing => next_state.set(GameState::Paused),
            GameState::Paused | GameState::Inventory => next_state.set(GameState::Playing),
            GameState::Options => next_state.set(options_return.0),
            GameState::MainMenu => {}
        }
    } else if keys.just_pressed(KeyCode::KeyE) {
        match *state.get() {
            GameState::Playing => next_state.set(GameState::Inventory),
            GameState::Inventory => next_state.set(GameState::Playing),
            _ => {}
        }
    }
}

fn capture_mouse(state: Res<State<GameState>>, mut cursor_options: Single<&mut CursorOptions>) {
    let in_game = *state.get() == GameState::Playing;
    cursor_options.visible = !in_game;
    cursor_options.grab_mode = if in_game {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
}

fn look(
    mut motion: MessageReader<MouseMotion>,
    state: Res<State<GameState>>,
    settings: Res<GameSettings>,
    mut player: Query<(&mut Transform, &mut PlayerLook), With<Player>>,
) {
    if *state.get() != GameState::Playing {
        motion.clear();
        return;
    }
    let delta: Vec2 = motion.read().map(|event| event.delta).sum();
    if let Ok((mut transform, mut look)) = player.single_mut() {
        look.yaw -= delta.x * settings.mouse_sensitivity;
        look.pitch = (look.pitch - delta.y * settings.mouse_sensitivity).clamp(-1.54, 1.54);
        transform.rotation = Quat::from_rotation_y(look.yaw) * Quat::from_rotation_x(look.pitch);
    }
}

fn movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    world: Res<WorldData>,
    mut player: Query<(&mut Transform, &mut PlayerVelocity, &mut PlayerLook), With<Player>>,
) {
    let Ok((mut transform, mut velocity, mut look)) = player.single_mut() else {
        return;
    };
    if *state.get() != GameState::Playing {
        velocity.0 = Vec3::ZERO;
        look.last_space_press = None;
        return;
    }
    let dt = time.delta_secs().min(0.05);
    let yaw = look.yaw;
    let forward = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    let right = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
    let mut wish = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        wish += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        wish -= forward;
    }
    if keys.pressed(KeyCode::KeyD) {
        wish += right;
    }
    if keys.pressed(KeyCode::KeyA) {
        wish -= right;
    }
    if wish.length_squared() > 0.0 {
        wish = wish.normalize();
    }
    let sprinting = keys.pressed(KeyCode::ShiftLeft);
    let speed = if sprinting {
        if look.flying {
            FLY_SPRINT_SPEED
        } else {
            SPRINT_SPEED
        }
    } else {
        WALK_SPEED
    };
    velocity.0.x = wish.x * speed;
    velocity.0.z = wish.z * speed;
    if keys.just_pressed(KeyCode::Space) {
        let toggled_flight = register_space_press(&mut look, time.elapsed_secs());
        if toggled_flight {
            velocity.0.y = 0.0;
            look.grounded = false;
        } else if !look.flying && look.grounded {
            velocity.0.y = 8.5;
            look.grounded = false;
        }
    }
    if look.flying {
        let ascend = if keys.pressed(KeyCode::Space) {
            1.0
        } else {
            0.0
        };
        let descend = if keys.pressed(KeyCode::ControlLeft) {
            1.0
        } else {
            0.0
        };
        velocity.0.y = (ascend - descend) * speed;
    } else {
        velocity.0.y = (velocity.0.y - 24.0 * dt).max(-50.0);
    }

    let mut next = transform.translation;
    if move_axis(&mut next, velocity.0.x * dt, 0, &world) {
        velocity.0.x = 0.0;
    }
    if move_axis(&mut next, velocity.0.z * dt, 2, &world) {
        velocity.0.z = 0.0;
    }
    look.grounded = false;
    if move_axis(&mut next, velocity.0.y * dt, 1, &world) {
        if velocity.0.y < 0.0 {
            look.grounded = true;
        }
        velocity.0.y = 0.0;
    }
    if stop_flying_when_grounded(&mut look, keys.pressed(KeyCode::ControlLeft)) {
        velocity.0.y = 0.0;
    }
    transform.translation = next;
}

fn register_space_press(look: &mut PlayerLook, now: f32) -> bool {
    if look
        .last_space_press
        .is_some_and(|previous| now - previous <= FLY_TAP_WINDOW_SECS)
    {
        look.flying = !look.flying;
        look.last_space_press = None;
        true
    } else {
        look.last_space_press = Some(now);
        false
    }
}

fn stop_flying_when_grounded(look: &mut PlayerLook, descend_held: bool) -> bool {
    if look.flying && descend_held && look.grounded {
        look.flying = false;
        true
    } else {
        false
    }
}

fn move_axis(position: &mut Vec3, distance: f32, axis: usize, world: &WorldData) -> bool {
    if distance == 0.0 {
        return false;
    }
    let steps = (distance.abs() / 0.2).ceil().max(1.0) as u32;
    let step = distance / steps as f32;
    for _ in 0..steps {
        match axis {
            0 => position.x += step,
            1 => position.y += step,
            _ => position.z += step,
        }
        if overlaps_world(*position, world) {
            match axis {
                0 => position.x -= step,
                1 => position.y -= step,
                _ => position.z -= step,
            }
            return true;
        }
    }
    false
}

fn overlaps_world(eye: Vec3, world: &WorldData) -> bool {
    let min = IVec3::new(
        (eye.x - PLAYER_RADIUS).floor() as i32,
        (eye.y - EYE_HEIGHT).floor() as i32,
        (eye.z - PLAYER_RADIUS).floor() as i32,
    );
    let max = IVec3::new(
        (eye.x + PLAYER_RADIUS - 0.0001).floor() as i32,
        (eye.y + PLAYER_HEADROOM).floor() as i32,
        (eye.z + PLAYER_RADIUS - 0.0001).floor() as i32,
    );
    for y in min.y..=max.y {
        for z in min.z..=max.z {
            for x in min.x..=max.x {
                if world.block_at(IVec3::new(x, y, z)).is_solid() {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Block;

    #[test]
    fn extreme_finite_rotation_normalizes_to_unit_quaternion() {
        let mut app = App::new();
        app.insert_resource(PendingPlayerRestore(Some(([0.0; 3], [3.4e38; 4]))));
        let player = app
            .world_mut()
            .spawn((
                Transform::default(),
                PlayerVelocity::default(),
                PlayerLook::default(),
                Player,
            ))
            .id();

        app.add_systems(Update, restore_loaded_player);
        app.update();

        let rotation = app.world().get::<Transform>(player).unwrap().rotation;
        assert!(
            (rotation.length() - 1.0).abs() < 1e-6,
            "normalized rotation was {rotation:?}"
        );
    }

    #[test]
    fn zero_saved_rotation_restores_identity_quaternion() {
        let mut app = App::new();
        app.insert_resource(PendingPlayerRestore(Some(([0.0; 3], [0.0; 4]))));
        let player = app
            .world_mut()
            .spawn((
                Transform::default(),
                PlayerVelocity::default(),
                PlayerLook::default(),
                Player,
            ))
            .id();

        app.add_systems(Update, restore_loaded_player);
        app.update();

        let rotation = app.world().get::<Transform>(player).unwrap().rotation;
        assert_eq!(rotation, Quat::IDENTITY);
    }

    #[test]
    fn swept_axis_motion_stops_at_walls_and_prevents_falling_through_blocks() {
        let mut world = WorldData::new(23_041, 2);
        world.overrides.insert(IVec3::new(1, 21, 0), Block::Stone);
        world.overrides.insert(IVec3::new(0, 20, 0), Block::Stone);

        let mut horizontal = Vec3::new(0.5, 22.62, 0.5);
        assert!(move_axis(&mut horizontal, 2.0, 0, &world));
        assert!(horizontal.x < 0.71);

        let mut falling = Vec3::new(0.5, 30.0, 0.5);
        assert!(move_axis(&mut falling, -15.0, 1, &world));
        assert!(falling.y - EYE_HEIGHT >= 21.0);
    }

    #[test]
    fn double_tap_space_toggles_flight() {
        let mut look = PlayerLook::default();

        assert!(!register_space_press(&mut look, 1.0));
        assert!(register_space_press(&mut look, 1.2));
        assert!(look.flying);
        assert!(!register_space_press(&mut look, 2.0));
        assert!(!register_space_press(&mut look, 2.5));
        assert!(look.flying);
        assert!(!register_space_press(&mut look, 3.0));
        assert!(register_space_press(&mut look, 3.1));
        assert!(!look.flying);
    }

    #[test]
    fn holding_descend_disables_flight_on_landing() {
        let mut look = PlayerLook {
            flying: true,
            grounded: true,
            ..default()
        };

        assert!(stop_flying_when_grounded(&mut look, true));
        assert!(!look.flying);
    }
}
