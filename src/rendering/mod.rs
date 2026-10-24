use bevy::{light::NotShadowCaster, prelude::*, render::view::Msaa};

use crate::{
    game::{GameSettings, GameState},
    player::Player,
    world::{WorldMaterial, block_texture_atlas},
};

const SUN_DIRECTION: Vec3 = Vec3::new(-20.0, 40.0, -110.0);
const SUN_DISTANCE: f32 = 250.0;
const SUN_RADIUS: f32 = 6.0;

#[derive(Component)]
struct VisibleSun;

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_scene)
            .add_systems(PostUpdate, follow_sun)
            .add_systems(Update, (sync_sun_visibility, sync_anti_aliasing));
    }
}

fn sync_anti_aliasing(settings: Res<GameSettings>, mut cameras: Query<&mut Msaa, With<Player>>) {
    if !settings.is_changed() {
        return;
    }
    let msaa = Msaa::from_samples(settings.anti_aliasing_samples);
    for mut camera_msaa in &mut cameras {
        *camera_msaa = msaa;
    }
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let block_texture = images.add(block_texture_atlas());
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(block_texture),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });
    commands.insert_resource(WorldMaterial(material));

    let sun_mesh = meshes.add(Sphere::new(SUN_RADIUS).mesh().uv(32, 20));
    let sun_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.88, 0.58),
        emissive: LinearRgba::rgb(3.0, 2.2, 1.2),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(sun_mesh),
        MeshMaterial3d(sun_material),
        Transform::from_translation(SUN_DIRECTION.normalize() * SUN_DISTANCE),
        NotShadowCaster,
        Visibility::Hidden,
        VisibleSun,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 13_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-40.0, 80.0, 35.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.78, 0.84, 1.0),
        brightness: 260.0,
        ..default()
    });
    commands.insert_resource(ClearColor(Color::srgb(0.48, 0.72, 0.94)));
}

fn follow_sun(
    player: Query<&Transform, (With<Player>, Without<VisibleSun>)>,
    mut sun: Query<&mut Transform, (With<VisibleSun>, Without<Player>)>,
) {
    let (Ok(player), Ok(mut sun)) = (player.single(), sun.single_mut()) else {
        return;
    };
    sun.translation = player.translation + SUN_DIRECTION.normalize() * SUN_DISTANCE;
}

fn sync_sun_visibility(
    state: Res<State<GameState>>,
    mut sun: Query<&mut Visibility, With<VisibleSun>>,
) {
    if !state.is_changed() {
        return;
    }
    let visibility = if matches!(
        *state.get(),
        GameState::Playing | GameState::Paused | GameState::Inventory
    ) {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut sun_visibility in &mut sun {
        *sun_visibility = visibility;
    }
}
