use bevy::{
    camera::Hdr,
    light::NotShadowCaster,
    prelude::*,
    render::view::{ColorGrading, Msaa},
};

use crate::{
    game::{DayNightCycle, GameSettings, GameState},
    player::Player,
    world::{WorldMaterial, block_texture_atlas},
};

const SUN_DIRECTION: Vec3 = Vec3::new(-20.0, 40.0, -110.0);
const SUN_DISTANCE: f32 = 250.0;
const SUN_RADIUS: f32 = 6.0;
const DAY_LENGTH_SECONDS: f32 = 600.0;
const DAYLIGHT_ILLUMINANCE: f32 = 13_000.0;
const MOONLIGHT_ILLUMINANCE: f32 = 700.0;

#[derive(Component)]
struct VisibleSun;

#[derive(Component)]
struct VisibleMoon;

#[derive(Component)]
struct SunLight;

#[derive(Component)]
struct MoonLight;

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_scene).add_systems(
            Update,
            (
                update_day_night,
                sync_anti_aliasing,
                sync_camera_graphics_settings,
            ),
        );
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

fn sync_camera_graphics_settings(
    mut commands: Commands,
    settings: Res<GameSettings>,
    mut cameras: Query<(Entity, &mut Projection, &mut ColorGrading, Has<Hdr>), With<Player>>,
) {
    if !settings.is_changed() {
        return;
    }
    for (entity, mut projection, mut grading, has_hdr) in &mut cameras {
        if let Projection::Perspective(perspective) = &mut *projection {
            perspective.fov = settings.field_of_view_degrees.to_radians();
        }
        grading.global.exposure = settings.brightness.log2();
        for section in grading.all_sections_mut() {
            section.contrast = settings.contrast;
            section.gamma = settings.gamma;
        }
        if settings.hdr_enabled != has_hdr {
            if settings.hdr_enabled {
                commands.entity(entity).insert(Hdr);
            } else {
                commands.entity(entity).remove::<Hdr>();
            }
        }
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

    let celestial_mesh = meshes.add(Sphere::new(SUN_RADIUS).mesh().uv(32, 20));
    let sun_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.88, 0.58),
        emissive: LinearRgba::rgb(3.0, 2.2, 1.2),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(celestial_mesh.clone()),
        MeshMaterial3d(sun_material),
        Transform::from_translation(SUN_DIRECTION.normalize() * SUN_DISTANCE),
        NotShadowCaster,
        Visibility::Hidden,
        VisibleSun,
    ));

    let moon_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.72, 0.80, 0.96),
        emissive: LinearRgba::rgb(0.16, 0.21, 0.34),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(celestial_mesh),
        MeshMaterial3d(moon_material),
        Transform::from_translation(-SUN_DIRECTION.normalize() * SUN_DISTANCE)
            .with_scale(Vec3::splat(0.72)),
        NotShadowCaster,
        Visibility::Hidden,
        VisibleMoon,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 13_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-40.0, 80.0, 35.0).looking_at(Vec3::ZERO, Vec3::Y),
        SunLight,
    ));
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.48, 0.60, 0.88),
            illuminance: 0.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::IDENTITY,
        MoonLight,
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.78, 0.84, 1.0),
        brightness: 260.0,
        ..default()
    });
    commands.insert_resource(ClearColor(Color::srgb(0.48, 0.72, 0.94)));
}

fn update_day_night(
    time: Res<Time>,
    state: Res<State<GameState>>,
    mut cycle: ResMut<DayNightCycle>,
    player: Query<&Transform, With<Player>>,
    mut celestial: Query<
        (
            &mut Transform,
            Option<&mut Visibility>,
            Option<&mut DirectionalLight>,
            Has<VisibleSun>,
            Has<VisibleMoon>,
            Has<SunLight>,
            Has<MoonLight>,
        ),
        (
            Or<(
                With<VisibleSun>,
                With<VisibleMoon>,
                With<SunLight>,
                With<MoonLight>,
            )>,
            Without<Player>,
        ),
    >,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut clear_color: ResMut<ClearColor>,
) {
    if *state.get() == GameState::Playing {
        let total_phase = cycle.phase + time.delta_secs() / DAY_LENGTH_SECONDS;
        let days_passed = total_phase.floor() as u32;
        cycle.day = cycle.day.saturating_add(days_passed);
        cycle.phase = total_phase.fract();
    }

    let angle = cycle.phase * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
    let sun_height = angle.sin();
    let sun_direction = Vec3::new(angle.cos(), sun_height, -0.18).normalize();
    let moon_direction = -sun_direction;
    let daylight = smoothstep(-0.12, 0.22, sun_height);
    let night = 1.0 - daylight;
    let twilight = (1.0 - sun_height.abs() / 0.24).clamp(0.0, 1.0);
    let in_world = matches!(
        *state.get(),
        GameState::Playing | GameState::Paused | GameState::Inventory
    );
    let player_position = player
        .single()
        .map(|transform| transform.translation)
        .unwrap_or(Vec3::ZERO);
    let sun_visible = in_world && sun_height > -0.08;
    let moon_visible = in_world && sun_height < 0.08;
    let sun_rotation = Quat::from_rotation_arc(Vec3::NEG_Z, -sun_direction);
    let moon_rotation = Quat::from_rotation_arc(Vec3::NEG_Z, sun_direction);

    for (mut transform, visibility, light, is_sun, is_moon, is_sun_light, is_moon_light) in
        &mut celestial
    {
        if is_sun {
            transform.translation = player_position + sun_direction * SUN_DISTANCE;
            if let Some(mut visibility) = visibility {
                *visibility = if sun_visible {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }
        } else if is_moon {
            transform.translation = player_position + moon_direction * SUN_DISTANCE;
            if let Some(mut visibility) = visibility {
                *visibility = if moon_visible {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }
        } else if is_sun_light {
            transform.rotation = sun_rotation;
            if let Some(mut light) = light {
                light.illuminance = DAYLIGHT_ILLUMINANCE * daylight;
                let sun_color =
                    Vec3::new(1.0, 0.72, 0.48).lerp(Vec3::new(1.0, 0.96, 0.86), daylight);
                light.color = Color::srgb(sun_color.x, sun_color.y, sun_color.z);
            }
        } else if is_moon_light {
            transform.rotation = moon_rotation;
            if let Some(mut light) = light {
                light.illuminance = MOONLIGHT_ILLUMINANCE * night;
            }
        }
    }

    let ambient_color = Vec3::new(0.12, 0.16, 0.30)
        .lerp(Vec3::new(0.78, 0.84, 1.0), daylight)
        .lerp(Vec3::new(1.0, 0.47, 0.26), twilight * 0.28);
    ambient.color = Color::srgb(ambient_color.x, ambient_color.y, ambient_color.z);
    ambient.brightness = 38.0 + 222.0 * daylight;

    let sky_color = Vec3::new(0.012, 0.018, 0.065)
        .lerp(Vec3::new(0.48, 0.72, 0.94), daylight)
        .lerp(Vec3::new(0.96, 0.36, 0.20), twilight * 0.72);
    clear_color.0 = Color::srgb(sky_color.x, sky_color.y, sky_color.z);
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
