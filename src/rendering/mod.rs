use bevy::{
    asset::RenderAssetUsages,
    camera::Hdr,
    image::{Image, ImageSampler},
    light::NotShadowCaster,
    math::Affine2,
    pbr::{DistanceFog, FogFalloff},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    render::view::{ColorGrading, Msaa},
};

use crate::{
    game::{DayNightCycle, GameSettings, GameState},
    player::{Player, PlayerLook},
    world::{
        WorldMaterial, block_texture_atlas, update_water_normal_texture, water_normal_texture,
        water_texture,
    },
};

const SUN_DIRECTION: Vec3 = Vec3::new(-20.0, 40.0, -110.0);
const SUN_DISTANCE: f32 = 250.0;
const SUN_RADIUS: f32 = 6.0;
const SUN_CORONA_SIZE: f32 = 26.0;
const SUN_FLARE_DISTANCE: f32 = 2.5;
const DAY_LENGTH_SECONDS: f32 = 600.0;
const DAYLIGHT_ILLUMINANCE: f32 = 13_000.0;
const MOONLIGHT_ILLUMINANCE: f32 = 700.0;

#[derive(Component)]
struct VisibleSun;

#[derive(Component)]
struct SunCorona;

#[derive(Component)]
struct SunFlare {
    axis_position: f32,
    size: Vec2,
    tint: Vec3,
}

#[derive(Component)]
struct VisibleMoon;

#[derive(Component)]
struct SunLight;

#[derive(Component)]
struct MoonLight;

pub struct RenderingPlugin;

#[derive(Resource)]
struct WaterNormalAnimation {
    texture: Handle<Image>,
    elapsed: f32,
    frame_accumulator: f32,
}

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_scene).add_systems(
            Update,
            (
                update_day_night,
                sync_anti_aliasing,
                sync_camera_graphics_settings,
                animate_water_normals,
                sync_underwater_fog,
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
        base_color_texture: Some(block_texture.clone()),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });
    let glass_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.78, 0.91, 0.98, 0.32),
        base_color_texture: Some(block_texture),
        perceptual_roughness: 0.12,
        metallic: 0.0,
        reflectance: 0.42,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let water_surface = images.add(water_texture());
    let water_normals = images.add(water_normal_texture());
    let water_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.32, 0.59, 0.70, 0.76),
        base_color_texture: Some(water_surface),
        normal_map_texture: Some(water_normals.clone()),
        perceptual_roughness: 0.38,
        metallic: 0.0,
        reflectance: 0.48,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    commands.insert_resource(WorldMaterial {
        terrain: material,
        water: water_material,
        glass: glass_material,
    });
    commands.insert_resource(WaterNormalAnimation {
        texture: water_normals,
        elapsed: 0.0,
        frame_accumulator: 0.0,
    });

    let celestial_mesh = meshes.add(Sphere::new(SUN_RADIUS).mesh().uv(32, 20));
    let sun_surface = images.add(sun_surface_texture());
    let sun_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(sun_surface),
        emissive: LinearRgba::rgb(2.6, 1.4, 0.42),
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

    let glow_texture = images.add(radial_glow_texture(128, true));
    let corona_mesh = meshes.add(Rectangle::new(SUN_CORONA_SIZE, SUN_CORONA_SIZE));
    let corona_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.55, 0.20, 0.72),
        base_color_texture: Some(glow_texture.clone()),
        emissive: LinearRgba::rgb(1.7, 0.58, 0.12),
        alpha_mode: AlphaMode::Add,
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(corona_mesh),
        MeshMaterial3d(corona_material),
        Transform::IDENTITY,
        NotShadowCaster,
        Visibility::Hidden,
        VisibleSun,
        SunCorona,
    ));

    let flare_mesh = meshes.add(Rectangle::new(1.0, 1.0));
    let flare_specs = [
        (0.0, Vec2::splat(0.30), Vec3::new(1.0, 0.72, 0.38)),
        (-0.48, Vec2::splat(0.38), Vec3::new(0.72, 0.90, 1.0)),
        (-1.0, Vec2::splat(0.64), Vec3::new(1.0, 0.48, 0.25)),
        (0.56, Vec2::new(1.10, 0.13), Vec3::new(1.0, 0.78, 0.42)),
        (-1.62, Vec2::splat(0.24), Vec3::new(0.54, 0.78, 1.0)),
    ];
    for (axis_position, size, tint) in flare_specs {
        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(tint.x, tint.y, tint.z, 0.0),
            base_color_texture: Some(glow_texture.clone()),
            emissive: LinearRgba::BLACK,
            alpha_mode: AlphaMode::Add,
            unlit: true,
            ..default()
        });
        commands.spawn((
            Mesh3d(flare_mesh.clone()),
            MeshMaterial3d(material),
            Transform::IDENTITY,
            NotShadowCaster,
            Visibility::Hidden,
            SunFlare {
                axis_position,
                size,
                tint,
            },
        ));
    }

    let moon_surface = images.add(moon_surface_texture());
    let moon_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.72, 0.80, 0.96),
        base_color_texture: Some(moon_surface),
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

fn animate_water_normals(
    time: Res<Time>,
    state: Res<State<GameState>>,
    mut animation: ResMut<WaterNormalAnimation>,
    mut images: ResMut<Assets<Image>>,
    world_material: Res<WorldMaterial>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if matches!(*state.get(), GameState::MainMenu | GameState::Options) {
        return;
    }
    animation.frame_accumulator += time.delta_secs();
    if animation.frame_accumulator < 1.0 / 30.0 {
        return;
    }

    animation.elapsed += animation.frame_accumulator;
    animation.frame_accumulator = 0.0;
    if let Some(mut image) = images.get_mut(&animation.texture) {
        update_water_normal_texture(&mut image, animation.elapsed);
    }
    if let Some(mut material) = materials.get_mut(&world_material.water) {
        material.uv_transform = Affine2::from_translation(Vec2::new(
            (animation.elapsed * 0.018).rem_euclid(1.0),
            (animation.elapsed * 0.006).rem_euclid(1.0),
        ));
    }
}

fn sync_underwater_fog(
    mut commands: Commands,
    cameras: Query<(Entity, &PlayerLook, Option<&DistanceFog>), With<Player>>,
) {
    for (entity, look, fog) in &cameras {
        if look.underwater && fog.is_none() {
            commands.entity(entity).insert(DistanceFog {
                color: Color::srgb(0.025, 0.19, 0.27),
                directional_light_color: Color::srgba(0.12, 0.48, 0.65, 0.10),
                directional_light_exponent: 8.0,
                falloff: FogFalloff::Exponential { density: 0.055 },
            });
        } else if !look.underwater && fog.is_some() {
            commands.entity(entity).remove::<DistanceFog>();
        }
    }
}

fn update_day_night(
    time: Res<Time>,
    state: Res<State<GameState>>,
    settings: Res<GameSettings>,
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
            Has<SunCorona>,
        ),
        (
            Or<(
                With<VisibleSun>,
                With<VisibleMoon>,
                With<SunLight>,
                With<MoonLight>,
            )>,
            Without<Player>,
            Without<SunFlare>,
        ),
    >,
    mut flares: Query<
        (
            &mut Transform,
            &mut Visibility,
            &MeshMaterial3d<StandardMaterial>,
            &SunFlare,
        ),
        Without<Player>,
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut clear_color: ResMut<ClearColor>,
) {
    if *state.get() == GameState::Playing {
        let total_phase =
            cycle.phase + time.delta_secs() * settings.day_night_speed / DAY_LENGTH_SECONDS;
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
    let player_transform = player.single().ok();
    let player_position = player_transform
        .map(|transform| transform.translation)
        .unwrap_or(Vec3::ZERO);
    let sun_visible = in_world && sun_height > -0.08;
    let moon_visible = in_world && sun_height < 0.08;
    let sun_rotation = Quat::from_rotation_arc(Vec3::NEG_Z, -sun_direction);
    let moon_rotation = Quat::from_rotation_arc(Vec3::NEG_Z, sun_direction);

    for (
        mut transform,
        visibility,
        light,
        is_sun,
        is_moon,
        is_sun_light,
        is_moon_light,
        is_sun_corona,
    ) in &mut celestial
    {
        if is_sun {
            transform.translation = player_position
                + sun_direction
                    * if is_sun_corona {
                        SUN_DISTANCE + 0.5
                    } else {
                        SUN_DISTANCE
                    };
            if is_sun_corona {
                transform.rotation = Quat::from_rotation_arc(Vec3::Z, -sun_direction);
            }
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

    let camera_basis = player_transform.map(|transform| {
        (
            transform.rotation * Vec3::NEG_Z,
            transform.rotation * Vec3::X,
            transform.rotation * Vec3::Y,
        )
    });
    let flare_alignment = camera_basis
        .map(|(forward, _, _)| forward.dot(sun_direction))
        .unwrap_or(-1.0);
    let flare_intensity = if sun_visible {
        smoothstep(
            20.0_f32.to_radians().cos(),
            1.5_f32.to_radians().cos(),
            flare_alignment,
        )
    } else {
        0.0
    };
    if let (Some(camera_transform), Some((forward, right, up))) = (player_transform, camera_basis) {
        let forward_dot = forward.dot(sun_direction);
        if forward_dot > 0.0 {
            let sun_screen_position = Vec2::new(
                right.dot(sun_direction) / forward_dot,
                up.dot(sun_direction) / forward_dot,
            ) * SUN_FLARE_DISTANCE;
            for (mut transform, mut visibility, material_handle, flare) in &mut flares {
                let alpha = flare_intensity * 0.72;
                transform.translation = camera_transform.translation
                    + forward * SUN_FLARE_DISTANCE
                    + right * (sun_screen_position.x * flare.axis_position)
                    + up * (sun_screen_position.y * flare.axis_position);
                transform.rotation = camera_transform.rotation;
                transform.scale = Vec3::new(flare.size.x, flare.size.y, 1.0);
                *visibility = if alpha > 0.005 {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
                if let Some(mut material) = materials.get_mut(&material_handle.0) {
                    material.base_color =
                        Color::srgba(flare.tint.x, flare.tint.y, flare.tint.z, alpha);
                    material.emissive = LinearRgba::rgb(
                        flare.tint.x * alpha * 0.65,
                        flare.tint.y * alpha * 0.65,
                        flare.tint.z * alpha * 0.65,
                    );
                }
            }
        } else {
            for (_, mut visibility, material_handle, _) in &mut flares {
                *visibility = Visibility::Hidden;
                if let Some(mut material) = materials.get_mut(&material_handle.0) {
                    material.base_color = Color::srgba(0.0, 0.0, 0.0, 0.0);
                    material.emissive = LinearRgba::BLACK;
                }
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

fn sun_surface_texture() -> Image {
    const WIDTH: u32 = 256;
    const HEIGHT: u32 = 128;
    let mut pixels = vec![0; (WIDTH * HEIGHT * 4) as usize];

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let u = x as f32 / WIDTH as f32;
            let v = y as f32 / HEIGHT as f32;
            let broad = (u * std::f32::consts::TAU * 3.0 + (v * std::f32::consts::TAU * 2.0).sin())
                .sin()
                * (v * std::f32::consts::TAU * 4.0).sin();
            let medium = (u * std::f32::consts::TAU * 11.0
                + (v * std::f32::consts::TAU * 7.0).cos())
            .sin()
                * (v * std::f32::consts::TAU * 9.0 + (u * std::f32::consts::TAU * 5.0).sin()).cos();
            let fine =
                (u * std::f32::consts::TAU * 29.0 + (v * std::f32::consts::TAU * 17.0).cos()).sin()
                    * (v * std::f32::consts::TAU * 19.0).cos();
            let heat = (0.57 + broad * 0.17 + medium * 0.11 + fine * 0.06).clamp(0.0, 1.0);
            let index = ((y * WIDTH + x) * 4) as usize;
            pixels[index] = 255;
            pixels[index + 1] = (112.0 + heat * 136.0) as u8;
            pixels[index + 2] = (20.0 + heat * 150.0) as u8;
            pixels[index + 3] = 255;
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::linear();
    image
}

fn moon_surface_texture() -> Image {
    const WIDTH: u32 = 256;
    const HEIGHT: u32 = 128;
    const CRATER_COUNT: u32 = 320;
    const GOLDEN_ANGLE: f32 = 2.399_963_1;

    let craters: Vec<_> = (0..CRATER_COUNT)
        .map(|index| {
            let y = 1.0 - 2.0 * (index as f32 + 0.5) / CRATER_COUNT as f32;
            let ring = (1.0 - y * y).sqrt();
            let angle = index as f32 * GOLDEN_ANGLE + (moon_random(index, 1) - 0.5) * 0.22;
            let center = Vec3::new(ring * angle.cos(), y, ring * angle.sin());
            let size = moon_random(index, 2);
            let radius = if size < 0.06 {
                0.075 + moon_random(index, 3) * 0.055
            } else if size < 0.30 {
                0.034 + moon_random(index, 3) * 0.041
            } else {
                0.012 + moon_random(index, 3) * 0.025
            };
            let depth = 0.14 + moon_random(index, 4) * 0.24;
            let chord_radius = 2.0 * (radius * 0.5).sin();
            (center, chord_radius, depth, radius > 0.075)
        })
        .collect();

    let mut pixels = vec![0; (WIDTH * HEIGHT * 4) as usize];
    for y in 0..HEIGHT {
        let latitude = (y as f32 / HEIGHT as f32 - 0.5) * std::f32::consts::PI;
        let latitude_cos = latitude.cos();
        for x in 0..WIDTH {
            let longitude = (x as f32 / WIDTH as f32 - 0.5) * std::f32::consts::TAU;
            let direction = Vec3::new(
                latitude_cos * longitude.cos(),
                latitude.sin(),
                latitude_cos * longitude.sin(),
            );

            let maria = (direction.dot(Vec3::new(3.1, 1.2, -2.4)).sin()
                * direction.dot(Vec3::new(-1.8, 3.5, 2.1)).cos())
                * 0.075;
            let surface_grain = (direction.dot(Vec3::new(16.0, -11.0, 9.0)).sin()
                * direction.dot(Vec3::new(7.0, 19.0, 13.0)).cos())
                * 0.025;
            let mut luminance = 0.77 + maria + surface_grain;

            for (center, crater_radius, depth, has_central_peak) in &craters {
                let chord_squared = (2.0 * (1.0 - direction.dot(*center))).max(0.0);
                let outer_radius = crater_radius * 1.18;
                if chord_squared >= outer_radius * outer_radius {
                    continue;
                }

                let crater_distance = chord_squared.sqrt() / crater_radius;
                if crater_distance < 0.84 {
                    let bowl = 1.0 - (crater_distance / 0.84).powi(2);
                    luminance -= depth * bowl;
                }
                let rim_distance = (crater_distance - 0.93) / 0.13;
                luminance += (-rim_distance * rim_distance).exp() * depth * 0.42;
                if *has_central_peak && crater_distance < 0.13 {
                    luminance += (1.0 - crater_distance / 0.13) * 0.045;
                }
            }

            let shade = (luminance.clamp(0.24, 0.98) * 255.0) as u8;
            let index = ((y * WIDTH + x) * 4) as usize;
            pixels[index] = shade;
            pixels[index + 1] = shade;
            pixels[index + 2] = shade;
            pixels[index + 3] = 255;
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::linear();
    image
}

fn moon_random(index: u32, salt: u32) -> f32 {
    let mut value = index
        .wrapping_add(salt.wrapping_mul(0x9e37_79b9))
        .wrapping_mul(0x85eb_ca6b);
    value ^= value >> 13;
    value = value.wrapping_mul(0xc2b2_ae35);
    value ^= value >> 16;
    (value as f32) / (u32::MAX as f32)
}

fn radial_glow_texture(size: u32, corona_rays: bool) -> Image {
    let mut pixels = vec![0; (size * size * 4) as usize];
    let center = (size as f32 - 1.0) * 0.5;

    for y in 0..size {
        for x in 0..size {
            let dx = (x as f32 - center) / center;
            let dy = (y as f32 - center) / center;
            let radius = (dx * dx + dy * dy).sqrt();
            let mut alpha = (1.0 - smoothstep(0.12, 1.0, radius)) * 0.78;
            if corona_rays {
                let angle = dy.atan2(dx);
                let ray = (angle * 12.0).cos().max(0.0).powf(14.0)
                    * (1.0 - radius).clamp(0.0, 1.0)
                    * 0.22;
                alpha = (alpha + ray).clamp(0.0, 1.0);
            }
            let index = ((y * size + x) * 4) as usize;
            pixels[index] = 255;
            pixels[index + 1] = 255;
            pixels[index + 2] = 255;
            pixels[index + 3] = (alpha * 255.0) as u8;
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::linear();
    image
}
