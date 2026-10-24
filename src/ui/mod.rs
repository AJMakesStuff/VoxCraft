use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    picking::Pickable,
    picking::hover::Hovered,
    picking::pointer::PointerButton,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    ui::Pressed,
    ui_widgets::{
        Activate, Button, ScrollArea, Slider, SliderPrecision, SliderRange, SliderStep,
        SliderThumb, SliderValue, TrackClick, ValueChange,
    },
    window::{
        Monitor, MonitorSelection, PresentMode, PrimaryMonitor, PrimaryWindow, VideoMode,
        VideoModeSelection, WindowMode, WindowPosition,
    },
};

use crate::{
    game::{
        DISPLAY_RESOLUTIONS, DisplayMode, GameSettings, GameState, OptionsReturn, SaveFile,
        SaveStatus, load_from_disk, save_mouse_sensitivity, save_to_disk,
    },
    interaction::{HOTBAR_SLOTS, Hotbar, Selection},
    player::{PendingPlayerRestore, Player, PlayerVelocity},
    world::{Block, CHUNK_SIZE, WorldData, block_texture_pixel_for_face},
};

const PANEL_COLOR: Color = Color::srgba(0.055, 0.075, 0.085, 0.96);
const BUTTON_COLOR: Color = Color::srgb(0.16, 0.22, 0.23);
const BUTTON_HOVER_COLOR: Color = Color::srgb(0.22, 0.30, 0.30);
const BUTTON_PRESSED_COLOR: Color = Color::srgb(0.30, 0.39, 0.36);
const HOTBAR_SELECTED_COLOR: Color = Color::srgb(0.35, 0.46, 0.38);
const TEXT_COLOR: Color = Color::srgb(0.91, 0.93, 0.88);
const MUTED_TEXT_COLOR: Color = Color::srgb(0.68, 0.75, 0.72);

#[derive(Component)]
struct ScreenRoot(GameState);

#[derive(Component)]
struct CrosshairUi;

#[derive(Component)]
struct HotbarRoot;

#[derive(Component)]
struct HotbarSelectionPopup;

#[derive(Component)]
struct InventoryHoverTooltip;

#[derive(Component)]
struct InventoryDragPreview;

#[derive(Component)]
struct DebugText;

#[derive(Component)]
struct SensitivityReadout;

#[derive(Component, Default)]
struct RenderDistanceReadout;

#[derive(Component, Default)]
struct AntiAliasingReadout;

#[derive(Component)]
struct InventoryHint;

#[derive(Component)]
struct HotbarSlot(usize);

#[derive(Component)]
struct HotbarIcon(usize);

#[derive(Component)]
struct InventoryBlockPicker(Block);

#[derive(Resource, Default)]
struct DraggedBlock(Option<Block>);

#[derive(Component)]
struct VsyncToggleLabel;

#[derive(Component)]
struct AdaptiveSyncToggleLabel;

#[derive(Component)]
struct HdrToggleLabel;

#[derive(Component, Default)]
struct DisplayModeReadout;

#[derive(Component, Default)]
struct ResolutionReadout;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GraphicsSliderKind {
    FieldOfView,
    Brightness,
    Contrast,
    Gamma,
}

#[derive(Component)]
struct GraphicsSlider(GraphicsSliderKind);

#[derive(Component)]
struct GraphicsSliderReadout(GraphicsSliderKind);

#[derive(Component)]
struct CalibrationSwatch(f32);

#[derive(Component)]
struct SaveStatusLabel;

#[derive(Component)]
struct ButtonTint(Color);

#[derive(Component, Clone, Copy)]
struct MenuButtonAction(UiAction);

#[derive(Clone, Copy)]
enum UiAction {
    StartGame,
    ResumeGame,
    OpenMainOptions,
    OpenPauseOptions,
    BackFromOptions,
    ReturnToMainMenu,
    Quit,
    SensitivityStep(i8),
    RenderDistanceStep(i8),
    AntiAliasingStep(i8),
    DisplayModeStep(i8),
    ResolutionStep(i8),
    ToggleVsync,
    ToggleAdaptiveSync,
    ToggleHdr,
    SaveGame,
    LoadGame,
    SelectHotbar(usize),
}

#[derive(Resource, Default)]
struct DebugEnabled(bool);

#[derive(Resource, Clone, Default)]
struct BlockIconAssets(HashMap<Block, Handle<Image>>);

impl BlockIconAssets {
    fn get(&self, block: Block) -> Handle<Image> {
        self.0.get(&block).cloned().unwrap_or_default()
    }
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugEnabled>()
            .init_resource::<DraggedBlock>()
            .init_resource::<SaveStatus>()
            .add_systems(Startup, setup_ui)
            .add_systems(
                Update,
                (
                    toggle_debug,
                    update_screen_visibility,
                    update_button_appearance,
                    update_hotbar_icons,
                    update_hotbar_selection_popup,
                    update_inventory_hover_tooltip,
                    update_inventory_drag_preview,
                    update_sensitivity_readout,
                    update_render_distance_readout,
                    update_anti_aliasing_readout,
                    update_vsync_toggle_label,
                    update_adaptive_sync_toggle_label,
                    update_hdr_toggle_label,
                    update_display_mode_readout,
                    update_resolution_readout,
                    update_graphics_slider_readouts,
                    update_calibration_pattern,
                    update_save_status,
                    update_inventory_hint,
                    update_debug_text,
                )
                    .chain(),
            );
    }
}

fn setup_ui(mut commands: Commands, mut images: ResMut<Assets<Image>>, hotbar: Res<Hotbar>) {
    let icons = create_block_icons(&mut images);
    commands.insert_resource(icons.clone());
    setup_main_menu(&mut commands);
    setup_options_menu(&mut commands);
    setup_pause_menu(&mut commands);
    setup_inventory(&mut commands, &icons, &hotbar);
    setup_hotbar(&mut commands, &icons);
    setup_crosshair(&mut commands);
    setup_debug_text(&mut commands);
}

fn create_screen(
    commands: &mut Commands,
    state: GameState,
    backdrop: Color,
    panel_width: f32,
) -> Entity {
    let scrollable = state == GameState::Options;
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(0.0),
                right: Val::Percent(0.0),
                top: Val::Percent(0.0),
                bottom: Val::Percent(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(backdrop),
            ScreenRoot(state),
            Visibility::Hidden,
            GlobalZIndex(20),
        ))
        .id();

    let mut panel = None;
    commands.entity(root).with_children(|screen| {
        let mut menu_panel = screen.spawn((
            Node {
                width: Val::Px(panel_width),
                max_width: Val::Percent(92.0),
                max_height: if scrollable {
                    Val::Percent(90.0)
                } else {
                    Val::Auto
                },
                overflow: if scrollable {
                    Overflow::scroll_y()
                } else {
                    Overflow::visible()
                },
                scrollbar_width: if scrollable { 10.0 } else { 0.0 },
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(12.0),
                padding: UiRect::all(Val::Px(28.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(PANEL_COLOR),
            BorderColor::all(Color::srgba(0.69, 0.78, 0.73, 0.24)),
        ));
        if scrollable {
            menu_panel.insert(ScrollArea);
        }
        panel = Some(menu_panel.id());
    });
    panel.expect("screen panel is created with its root")
}

fn setup_main_menu(commands: &mut Commands) {
    let panel = create_screen(
        commands,
        GameState::MainMenu,
        Color::srgba(0.015, 0.025, 0.032, 0.94),
        440.0,
    );
    commands.entity(panel).with_children(|panel| {
        add_label(panel, "VOXCRAFT", 38.0, TEXT_COLOR);
        add_label(
            panel,
            "A quiet world of blocks, hills, and open sky",
            16.0,
            MUTED_TEXT_COLOR,
        );
        add_button(
            panel,
            "Start Game",
            UiAction::StartGame,
            340.0,
            50.0,
            None,
            None,
        );
        add_button(
            panel,
            "Load Game",
            UiAction::LoadGame,
            340.0,
            50.0,
            None,
            None,
        );
        add_button(
            panel,
            "Options",
            UiAction::OpenMainOptions,
            340.0,
            50.0,
            None,
            None,
        );
        add_button(panel, "Quit", UiAction::Quit, 340.0, 50.0, None, None);
        add_save_status_label(panel);
        add_label(
            panel,
            "WASD move  |  Space jump  |  Mouse look",
            14.0,
            MUTED_TEXT_COLOR,
        );
        add_label(
            panel,
            "Double-tap Space to fly  |  Ctrl descends and lands",
            14.0,
            MUTED_TEXT_COLOR,
        );
    });
}

fn setup_options_menu(commands: &mut Commands) {
    let panel = create_screen(
        commands,
        GameState::Options,
        Color::srgba(0.015, 0.025, 0.032, 0.92),
        680.0,
    );
    commands.entity(panel).with_children(|panel| {
        add_label(panel, "OPTIONS", 34.0, TEXT_COLOR);
        add_label(
            panel,
            "Adjust mouse look speed. Changes apply immediately.",
            16.0,
            MUTED_TEXT_COLOR,
        );
        panel.spawn((
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(20.0),
                ..default()
            },
            TextColor(TEXT_COLOR),
            TextLayout::justify(Justify::Center),
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(42.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            SensitivityReadout,
        ));
        panel
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(12.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|row| {
                add_button(
                    row,
                    "-",
                    UiAction::SensitivityStep(-1),
                    72.0,
                    46.0,
                    None,
                    None,
                );
                add_button(
                    row,
                    "+",
                    UiAction::SensitivityStep(1),
                    72.0,
                    46.0,
                    None,
                    None,
                );
            });
        add_label(panel, "GRAPHICS", 24.0, TEXT_COLOR);
        add_label(panel, "DISPLAY", 16.0, MUTED_TEXT_COLOR);
        add_label(panel, "Display Mode", 14.0, MUTED_TEXT_COLOR);
        add_graphics_stepper::<DisplayModeReadout>(
            panel,
            UiAction::DisplayModeStep(-1),
            UiAction::DisplayModeStep(1),
        );
        add_label(panel, "Resolution", 14.0, MUTED_TEXT_COLOR);
        add_graphics_stepper::<ResolutionReadout>(
            panel,
            UiAction::ResolutionStep(-1),
            UiAction::ResolutionStep(1),
        );
        add_graphics_slider(
            panel,
            "Field of View",
            GraphicsSliderKind::FieldOfView,
            60.0,
            120.0,
            75.0,
            1.0,
            0,
        );
        add_label(panel, "Render Distance", 14.0, MUTED_TEXT_COLOR);
        add_graphics_stepper::<RenderDistanceReadout>(
            panel,
            UiAction::RenderDistanceStep(-1),
            UiAction::RenderDistanceStep(1),
        );
        add_label(panel, "Anti-Aliasing", 14.0, MUTED_TEXT_COLOR);
        add_graphics_stepper::<AntiAliasingReadout>(
            panel,
            UiAction::AntiAliasingStep(-1),
            UiAction::AntiAliasingStep(1),
        );
        add_label(panel, "SYNC", 16.0, MUTED_TEXT_COLOR);
        add_button(
            panel,
            "V-Sync: Off",
            UiAction::ToggleVsync,
            420.0,
            44.0,
            None,
            None,
        );
        add_button(
            panel,
            "Adaptive V-Sync: Off",
            UiAction::ToggleAdaptiveSync,
            420.0,
            44.0,
            None,
            None,
        );
        add_label(
            panel,
            "Adaptive presentation is used where supported. G-Sync / FreeSync is controlled by your display driver.",
            13.0,
            MUTED_TEXT_COLOR,
        );
        add_label(panel, "COLOR CALIBRATION", 16.0, MUTED_TEXT_COLOR);
        add_graphics_slider(
            panel,
            "Brightness",
            GraphicsSliderKind::Brightness,
            0.5,
            1.5,
            1.0,
            0.05,
            2,
        );
        add_graphics_slider(
            panel,
            "Contrast",
            GraphicsSliderKind::Contrast,
            0.5,
            1.5,
            1.0,
            0.05,
            2,
        );
        add_graphics_slider(
            panel,
            "Gamma",
            GraphicsSliderKind::Gamma,
            0.5,
            2.0,
            1.0,
            0.05,
            2,
        );
        add_label(panel, "Live calibration pattern", 13.0, MUTED_TEXT_COLOR);
        add_calibration_pattern(panel);
        add_button(
            panel,
            "HDR Rendering: Off",
            UiAction::ToggleHdr,
            420.0,
            44.0,
            None,
            None,
        );
        add_label(
            panel,
            "HDR rendering uses a higher precision camera buffer. Bevy does not currently output HDR to HDR monitors.",
            13.0,
            MUTED_TEXT_COLOR,
        );
        add_button(
            panel,
            "Back",
            UiAction::BackFromOptions,
            420.0,
            50.0,
            None,
            None,
        );
        add_label(
            panel,
            "Escape returns to the previous screen",
            14.0,
            MUTED_TEXT_COLOR,
        );
    });
}

fn add_graphics_stepper<T: Component + Default>(
    parent: &mut ChildSpawnerCommands<'_>,
    decrease: UiAction,
    increase: UiAction,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|row| {
            add_button(row, "−", decrease, 64.0, 42.0, None, None);
            row.spawn((
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(TEXT_COLOR),
                TextLayout::justify(Justify::Center),
                Node {
                    width: Val::Px(270.0),
                    height: Val::Px(42.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                T::default(),
            ));
            add_button(row, "+", increase, 64.0, 42.0, None, None);
        });
}

fn add_graphics_slider(
    parent: &mut ChildSpawnerCommands<'_>,
    label: &str,
    kind: GraphicsSliderKind,
    min: f32,
    max: f32,
    value: f32,
    step: f32,
    precision: i32,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(MUTED_TEXT_COLOR),
                TextLayout::no_wrap(),
                Node {
                    width: Val::Px(128.0),
                    ..default()
                },
            ));
            let mut slider = row.spawn((
                Slider {
                    track_click: TrackClick::Snap,
                    ..default()
                },
                SliderValue(value),
                SliderRange::new(min, max),
                SliderStep(step),
                SliderPrecision(precision),
                GraphicsSlider(kind),
                Hovered::default(),
                Node {
                    width: Val::Px(250.0),
                    height: Val::Px(26.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
            ));
            slider.observe(on_graphics_slider_change);
            slider.with_children(|slider| {
                slider.spawn((
                    Node {
                        height: Val::Px(6.0),
                        width: Val::Percent(100.0),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.045, 0.06, 0.065)),
                ));
                slider.spawn((
                    SliderThumb,
                    Node {
                        width: Val::Px(14.0),
                        height: Val::Px(20.0),
                        position_type: PositionType::Absolute,
                        left: Val::Percent(0.0),
                        top: Val::Px(3.0),
                        border_radius: BorderRadius::all(Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(HOTBAR_SELECTED_COLOR),
                ));
            });
            row.spawn((
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(TEXT_COLOR),
                TextLayout::no_wrap().with_justify(Justify::Right),
                Node {
                    width: Val::Px(56.0),
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                },
                GraphicsSliderReadout(kind),
            ));
        });
}

fn add_calibration_pattern(parent: &mut ChildSpawnerCommands<'_>) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(38.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|row| {
            for shade in [
                0.0, 0.01, 0.025, 0.05, 0.10, 0.18, 0.30, 0.45, 0.60, 0.72, 0.84, 0.93, 0.98, 1.0,
            ] {
                row.spawn((
                    Node {
                        flex_grow: 1.0,
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(shade, shade, shade)),
                    BorderColor::all(Color::srgba(0.7, 0.75, 0.75, 0.16)),
                    CalibrationSwatch(shade),
                ));
            }
        });
}

fn setup_pause_menu(commands: &mut Commands) {
    let panel = create_screen(
        commands,
        GameState::Paused,
        Color::srgba(0.015, 0.025, 0.032, 0.82),
        440.0,
    );
    commands.entity(panel).with_children(|panel| {
        add_label(panel, "PAUSED", 36.0, TEXT_COLOR);
        add_label(
            panel,
            "The world is waiting for you.",
            16.0,
            MUTED_TEXT_COLOR,
        );
        add_button(
            panel,
            "Resume",
            UiAction::ResumeGame,
            340.0,
            50.0,
            None,
            None,
        );
        add_button(
            panel,
            "Save Game",
            UiAction::SaveGame,
            340.0,
            50.0,
            None,
            None,
        );
        add_button(
            panel,
            "Options",
            UiAction::OpenPauseOptions,
            340.0,
            50.0,
            None,
            None,
        );
        add_save_status_label(panel);
        add_button(
            panel,
            "Main Menu",
            UiAction::ReturnToMainMenu,
            340.0,
            50.0,
            None,
            None,
        );
        add_label(
            panel,
            "Escape resumes, then E opens the block picker",
            14.0,
            MUTED_TEXT_COLOR,
        );
        add_label(
            panel,
            "Double-tap Space to fly  |  Ctrl descends and lands",
            14.0,
            MUTED_TEXT_COLOR,
        );
    });
}

fn setup_inventory(commands: &mut Commands, icons: &BlockIconAssets, hotbar: &Hotbar) {
    let panel = create_screen(
        commands,
        GameState::Inventory,
        Color::srgba(0.015, 0.025, 0.032, 0.86),
        620.0,
    );
    commands.entity(panel).with_children(|panel| {
        add_label(panel, "BLOCK PICKER", 34.0, TEXT_COLOR);
        panel.spawn((
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(16.0),
                ..default()
            },
            TextColor(MUTED_TEXT_COLOR),
            TextLayout::justify(Justify::Center),
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(32.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            InventoryHint,
        ));
        panel
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(8.0),
                    row_gap: Val::Px(8.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|row| {
                for block in Block::SOLID_PICKER {
                    add_block_picker_button(row, block, icons.get(block));
                }
            });
        add_label(panel, "HOTBAR", 16.0, MUTED_TEXT_COLOR);
        panel
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(4.0),
                    padding: UiRect::all(Val::Px(5.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.025, 0.035, 0.04, 0.78)),
                BorderColor::all(Color::srgba(0.8, 0.84, 0.77, 0.28)),
            ))
            .with_children(|row| {
                for (slot, block) in hotbar.slots.iter().copied().enumerate() {
                    add_hotbar_button(row, slot, icons.get(block));
                }
            });
        add_button(
            panel,
            "Back to Game",
            UiAction::ResumeGame,
            300.0,
            48.0,
            None,
            None,
        );
        add_label(
            panel,
            "Click or drag a block, then click or drag a hotbar slot. E or Escape closes the picker.",
            14.0,
            MUTED_TEXT_COLOR,
        );
    });

    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(TEXT_COLOR),
        TextLayout::no_wrap().with_justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            padding: UiRect::axes(Val::Px(9.0), Val::Px(5.0)),
            ..default()
        },
        BackgroundColor(PANEL_COLOR),
        BorderColor::all(Color::srgba(0.75, 0.82, 0.76, 0.35)),
        InventoryHoverTooltip,
        Pickable::IGNORE,
        Visibility::Hidden,
        GlobalZIndex(30),
    ));

    commands.spawn((
        ImageNode::new(icons.get(Block::Grass)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Px(56.0),
            height: Val::Px(56.0),
            padding: UiRect::all(Val::Px(3.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(PANEL_COLOR),
        BorderColor::all(Color::srgba(0.85, 0.92, 0.86, 0.8)),
        InventoryDragPreview,
        Pickable::IGNORE,
        Visibility::Hidden,
        GlobalZIndex(40),
    ));
}

fn setup_hotbar(commands: &mut Commands, icons: &BlockIconAssets) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(16.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            HotbarRoot,
            Visibility::Hidden,
            GlobalZIndex(5),
        ))
        .id();
    commands.entity(root).with_children(|root| {
        root.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|layout| {
            layout.spawn((
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(TEXT_COLOR),
                TextLayout::no_wrap().with_justify(Justify::Center),
                Node {
                    width: Val::Auto,
                    min_height: Val::Px(24.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.025, 0.035, 0.04, 0.78)),
                BorderColor::all(Color::srgba(0.8, 0.84, 0.77, 0.28)),
                HotbarSelectionPopup,
                Visibility::Hidden,
            ));
            layout
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        column_gap: Val::Px(4.0),
                        padding: UiRect::all(Val::Px(5.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.025, 0.035, 0.04, 0.78)),
                    BorderColor::all(Color::srgba(0.8, 0.84, 0.77, 0.28)),
                ))
                .with_children(|row| {
                    for slot in 0..HOTBAR_SLOTS {
                        let block = Block::SOLID_PICKER[slot];
                        add_hotbar_button(row, slot, icons.get(block));
                    }
                });
        });
    });
}

fn setup_crosshair(commands: &mut Commands) {
    commands.spawn((
        Text::new("+"),
        TextFont {
            font_size: FontSize::Px(26.0),
            ..default()
        },
        TextColor(TEXT_COLOR),
        TextLayout::no_wrap().with_justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            ..default()
        },
        CrosshairUi,
        Visibility::Hidden,
    ));
}

fn setup_debug_text(commands: &mut Commands) {
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextColor(TEXT_COLOR),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(12.0),
            ..default()
        },
        DebugText,
        Visibility::Hidden,
    ));
}

fn add_label(
    parent: &mut ChildSpawnerCommands<'_>,
    text: impl Into<String>,
    size: f32,
    color: Color,
) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        TextLayout::justify(Justify::Center),
        Node {
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
    ));
}

fn add_save_status_label(parent: &mut ChildSpawnerCommands<'_>) {
    parent.spawn((
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(MUTED_TEXT_COLOR),
        TextLayout::no_wrap().with_justify(Justify::Center),
        Node {
            width: Val::Percent(100.0),
            min_height: Val::Px(22.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        SaveStatusLabel,
    ));
}

fn add_button(
    parent: &mut ChildSpawnerCommands<'_>,
    label: &str,
    action: UiAction,
    width: f32,
    height: f32,
    tint: Option<Color>,
    hotbar_slot: Option<usize>,
) {
    let mut button = parent.spawn((
        Button,
        Hovered::default(),
        MenuButtonAction(action),
        Node {
            width: Val::Px(width),
            height: Val::Px(height),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BorderColor::all(Color::srgba(0.75, 0.82, 0.76, 0.35)),
        BackgroundColor(tint.unwrap_or(BUTTON_COLOR)),
        Text::new(label),
        TextFont {
            font_size: FontSize::Px(17.0),
            ..default()
        },
        TextColor(TEXT_COLOR),
        TextLayout::justify(Justify::Center),
    ));
    if let Some(tint) = tint {
        button.insert(ButtonTint(tint));
    }
    if let Some(slot) = hotbar_slot {
        button.insert(HotbarSlot(slot));
    }
    if matches!(action, UiAction::ToggleVsync) {
        button.insert(VsyncToggleLabel);
    }
    if matches!(action, UiAction::ToggleAdaptiveSync) {
        button.insert(AdaptiveSyncToggleLabel);
    }
    if matches!(action, UiAction::ToggleHdr) {
        button.insert(HdrToggleLabel);
    }
    button.observe(on_menu_action);
}

fn add_hotbar_button(parent: &mut ChildSpawnerCommands<'_>, slot: usize, icon: Handle<Image>) {
    let mut button = parent.spawn((
        Button,
        Hovered::default(),
        MenuButtonAction(UiAction::SelectHotbar(slot)),
        HotbarSlot(slot),
        Node {
            width: Val::Px(60.0),
            height: Val::Px(60.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(4.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BorderColor::all(Color::srgba(0.75, 0.82, 0.76, 0.35)),
        BackgroundColor(BUTTON_COLOR),
    ));
    button.with_children(|button| {
        button.spawn((
            ImageNode::new(icon),
            Node {
                width: Val::Px(48.0),
                height: Val::Px(48.0),
                ..default()
            },
            HotbarIcon(slot),
            Pickable::IGNORE,
        ));
    });
    button.observe(on_menu_action);
    button.observe(on_inventory_hotbar_drop);
}

fn add_block_picker_button(
    parent: &mut ChildSpawnerCommands<'_>,
    block: Block,
    icon: Handle<Image>,
) {
    let tint = block_tint(block);
    let mut button = parent.spawn((
        Button,
        Hovered::default(),
        InventoryBlockPicker(block),
        ButtonTint(tint),
        Node {
            width: Val::Px(96.0),
            height: Val::Px(96.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(3.0),
            padding: UiRect::all(Val::Px(5.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BorderColor::all(Color::srgba(0.75, 0.82, 0.76, 0.35)),
        BackgroundColor(tint),
    ));
    button.with_children(|button| {
        button.spawn((
            ImageNode::new(icon),
            Node {
                width: Val::Px(76.0),
                height: Val::Px(76.0),
                ..default()
            },
            Pickable::IGNORE,
        ));
    });
    button.observe(pick_up_inventory_block);
    button.observe(begin_inventory_block_drag);
    button.observe(end_inventory_block_drag);
}

fn apply_present_mode(
    settings: &GameSettings,
    windows: &mut Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    window.present_mode = if !settings.vsync_enabled {
        PresentMode::AutoNoVsync
    } else if settings.adaptive_sync_enabled {
        // AutoVsync tries the platform's adaptive FIFO mode before standard FIFO.
        PresentMode::AutoVsync
    } else {
        PresentMode::Fifo
    };
}

fn apply_display_settings(
    settings: &GameSettings,
    windows: &mut Query<&mut Window, With<PrimaryWindow>>,
    monitors: &Query<&Monitor, With<PrimaryMonitor>>,
) {
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    let monitor = monitors.single().ok();
    let fallback_size = window.resolution.physical_size();
    let (width, height) = selected_resolution(settings.resolution_index, monitor, fallback_size);
    window.present_mode = if !settings.vsync_enabled {
        PresentMode::AutoNoVsync
    } else if settings.adaptive_sync_enabled {
        PresentMode::AutoVsync
    } else {
        PresentMode::Fifo
    };

    match settings.display_mode {
        DisplayMode::Windowed => {
            window.mode = WindowMode::Windowed;
            window.set_maximized(false);
            window.resolution.set_physical_resolution(width, height);
            window.position = WindowPosition::Centered(MonitorSelection::Current);
        }
        DisplayMode::Borderless => {
            window.set_maximized(false);
            window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
        }
        DisplayMode::Fullscreen => {
            window.set_maximized(false);
            let video_mode = selected_video_mode(settings.resolution_index, monitor)
                .map(VideoModeSelection::Specific)
                .unwrap_or(VideoModeSelection::Current);
            window.mode = WindowMode::Fullscreen(MonitorSelection::Current, video_mode);
        }
    }
}

fn selected_resolution(index: usize, monitor: Option<&Monitor>, fallback: UVec2) -> (u32, u32) {
    if index == 0 {
        return monitor
            .map(|monitor| (monitor.physical_width, monitor.physical_height))
            .filter(|(width, height)| *width > 0 && *height > 0)
            .unwrap_or((fallback.x.max(800), fallback.y.max(600)));
    }
    DISPLAY_RESOLUTIONS
        .get(index - 1)
        .copied()
        .unwrap_or((fallback.x, fallback.y))
}

fn selected_video_mode(index: usize, monitor: Option<&Monitor>) -> Option<VideoMode> {
    if index == 0 {
        return None;
    }
    let (width, height) = *DISPLAY_RESOLUTIONS.get(index - 1)?;
    monitor?
        .video_modes
        .iter()
        .copied()
        .filter(|mode| mode.physical_size == UVec2::new(width, height))
        .max_by_key(|mode| (mode.refresh_rate_millihertz, mode.bit_depth))
}

fn on_menu_action(
    event: On<Activate>,
    actions: Query<&MenuButtonAction>,
    mut next_state: ResMut<NextState<GameState>>,
    mut options_return: ResMut<OptionsReturn>,
    mut settings: ResMut<GameSettings>,
    mut hotbar: ResMut<Hotbar>,
    state: Res<State<GameState>>,
    mut dragged: ResMut<DraggedBlock>,
    mut world: ResMut<WorldData>,
    player: Query<&Transform, With<Player>>,
    mut pending_player_restore: ResMut<PendingPlayerRestore>,
    mut save_status: ResMut<SaveStatus>,
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,
    primary_monitor: Query<&Monitor, With<PrimaryMonitor>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    let Ok(MenuButtonAction(action)) = actions.get(event.entity) else {
        return;
    };
    match *action {
        UiAction::StartGame => {
            settings.prepare_new_world_seed();
            next_state.set(GameState::Playing);
        }
        UiAction::ResumeGame => next_state.set(GameState::Playing),
        UiAction::OpenMainOptions => {
            options_return.0 = GameState::MainMenu;
            next_state.set(GameState::Options);
        }
        UiAction::OpenPauseOptions => {
            options_return.0 = GameState::Paused;
            next_state.set(GameState::Options);
        }
        UiAction::BackFromOptions => next_state.set(options_return.0),
        UiAction::ReturnToMainMenu => next_state.set(GameState::MainMenu),
        UiAction::Quit => {
            app_exit.write(AppExit::Success);
        }
        UiAction::SensitivityStep(direction) => {
            settings.adjust_mouse_sensitivity(direction as f32 * 0.0002);
            if let Err(error) = save_mouse_sensitivity(settings.mouse_sensitivity) {
                eprintln!("Could not persist mouse sensitivity: {error}");
            }
        }
        UiAction::RenderDistanceStep(direction) => {
            settings.adjust_render_distance(direction as i32);
        }
        UiAction::AntiAliasingStep(direction) => settings.step_anti_aliasing(direction),
        UiAction::DisplayModeStep(direction) => {
            settings.step_display_mode(direction);
            apply_display_settings(&settings, &mut primary_window, &primary_monitor);
        }
        UiAction::ResolutionStep(direction) => {
            settings.step_resolution(direction);
            apply_display_settings(&settings, &mut primary_window, &primary_monitor);
        }
        UiAction::ToggleVsync => {
            settings.vsync_enabled = !settings.vsync_enabled;
            if !settings.vsync_enabled {
                settings.adaptive_sync_enabled = false;
            }
            apply_present_mode(&settings, &mut primary_window);
        }
        UiAction::ToggleAdaptiveSync => {
            settings.adaptive_sync_enabled = !settings.adaptive_sync_enabled;
            if settings.adaptive_sync_enabled {
                settings.vsync_enabled = true;
            }
            apply_present_mode(&settings, &mut primary_window);
        }
        UiAction::ToggleHdr => settings.hdr_enabled = !settings.hdr_enabled,
        UiAction::SaveGame => {
            let Ok(player) = player.single() else {
                save_status.0 = "Could not find the player to save.".to_owned();
                return;
            };
            let save = SaveFile::capture(&world, player, &hotbar);
            save_status.0 = match save_to_disk(&save) {
                Ok(()) => "Game saved.".to_owned(),
                Err(error) => format!("Save failed: {error}"),
            };
        }
        UiAction::LoadGame => match load_from_disk() {
            Ok(save) => {
                let Ok(hotbar_blocks) = save.decoded_hotbar() else {
                    save_status.0 = "Save contains invalid hotbar data.".to_owned();
                    return;
                };
                let Ok(edits) = save.decoded_edits() else {
                    save_status.0 = "Save contains invalid block data.".to_owned();
                    return;
                };
                settings.seed = save.terrain_seed;
                world.restore_saved_world(save.terrain_seed, edits);
                hotbar.slots = hotbar_blocks;
                hotbar.selected = save.hotbar_selected;
                pending_player_restore.0 = Some((save.player_position, save.player_rotation));
                save_status.0 = format!("Loaded world seed {}.", save.terrain_seed);
                next_state.set(GameState::Playing);
            }
            Err(error) => save_status.0 = error,
        },
        UiAction::SelectHotbar(slot) => {
            if *state.get() == GameState::Inventory
                && let Some(block) = dragged.0.take()
            {
                hotbar.replace_slot(slot, block);
            }
            hotbar.select_slot(slot);
        }
    }
}

fn pick_up_inventory_block(
    event: On<Activate>,
    pickers: Query<&InventoryBlockPicker>,
    state: Res<State<GameState>>,
    mut dragged: ResMut<DraggedBlock>,
) {
    if *state.get() != GameState::Inventory {
        return;
    }
    if let Ok(InventoryBlockPicker(block)) = pickers.get(event.entity) {
        dragged.0 = Some(*block);
    }
}

fn begin_inventory_block_drag(
    event: On<PointerDragStart>,
    pickers: Query<&InventoryBlockPicker>,
    mut dragged: ResMut<DraggedBlock>,
) {
    if event.button == PointerButton::Primary {
        dragged.0 = pickers.get(event.entity).ok().map(|picker| picker.0);
    }
}

fn end_inventory_block_drag(_event: On<PointerDragEnd>, mut dragged: ResMut<DraggedBlock>) {
    dragged.0 = None;
}

fn on_inventory_hotbar_drop(
    mut event: On<PointerDragDrop>,
    state: Res<State<GameState>>,
    pickers: Query<&InventoryBlockPicker>,
    hotbar_slots: Query<&HotbarSlot>,
    mut hotbar: ResMut<Hotbar>,
) {
    if *state.get() != GameState::Inventory || event.button != PointerButton::Primary {
        return;
    }
    let (Ok(InventoryBlockPicker(block)), Ok(HotbarSlot(slot))) =
        (pickers.get(event.dropped), hotbar_slots.get(event.entity))
    else {
        return;
    };
    hotbar.replace_slot(*slot, *block);
    event.propagate(false);
}

fn toggle_debug(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut enabled: ResMut<DebugEnabled>,
) {
    if *state.get() == GameState::Playing && keys.just_pressed(KeyCode::F3) {
        enabled.0 = !enabled.0;
    }
}

type UiVisibilityQuery<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static ScreenRoot>,
        Option<&'static CrosshairUi>,
        Option<&'static HotbarRoot>,
        Option<&'static DebugText>,
        &'static mut Visibility,
    ),
    Or<(
        With<ScreenRoot>,
        With<CrosshairUi>,
        With<HotbarRoot>,
        With<DebugText>,
    )>,
>;

fn update_screen_visibility(
    state: Res<State<GameState>>,
    debug_enabled: Res<DebugEnabled>,
    mut query: UiVisibilityQuery,
) {
    for (screen, crosshair, hotbar, debug, mut visibility) in &mut query {
        let visible = if let Some(screen) = screen {
            *state.get() == screen.0
        } else if crosshair.is_some() || hotbar.is_some() {
            *state.get() == GameState::Playing
        } else {
            *state.get() == GameState::Playing && debug_enabled.0 && debug.is_some()
        };
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

type ButtonAppearanceQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Hovered,
        Has<Pressed>,
        Option<&'static HotbarSlot>,
        Option<&'static ButtonTint>,
        &'static mut BackgroundColor,
    ),
    With<Button>,
>;

fn update_button_appearance(hotbar: Res<Hotbar>, mut buttons: ButtonAppearanceQuery) {
    for (hovered, pressed, hotbar_slot, tint, mut background) in &mut buttons {
        let color = if hotbar_slot.is_some_and(|slot| slot.0 == hotbar.selected) {
            HOTBAR_SELECTED_COLOR
        } else if pressed {
            BUTTON_PRESSED_COLOR
        } else if hovered.0 {
            BUTTON_HOVER_COLOR
        } else {
            tint.map_or(BUTTON_COLOR, |tint| tint.0)
        };
        background.0 = color;
    }
}

fn update_hotbar_icons(
    hotbar: Res<Hotbar>,
    icons: Res<BlockIconAssets>,
    mut slots: Query<(&HotbarIcon, &mut ImageNode)>,
) {
    if !hotbar.is_changed() {
        return;
    }
    for (slot, mut image) in &mut slots {
        image.image = icons.get(hotbar.slots[slot.0]);
    }
}

fn update_hotbar_selection_popup(
    time: Res<Time>,
    state: Res<State<GameState>>,
    hotbar: Res<Hotbar>,
    mut last_selection: Local<Option<(usize, Block)>>,
    mut remaining: Local<f32>,
    mut popup: Query<(&mut Text, &mut Visibility), With<HotbarSelectionPopup>>,
) {
    let Ok((mut text, mut visibility)) = popup.single_mut() else {
        return;
    };
    if *state.get() != GameState::Playing {
        *remaining = 0.0;
        *visibility = Visibility::Hidden;
        return;
    }

    let selection = (hotbar.selected, hotbar.selected_block());
    if let Some(previous) = *last_selection {
        if selection != previous {
            text.0 = selection.1.label().to_owned();
            *remaining = 1.25;
        }
    } else {
        *last_selection = Some(selection);
    }

    if *last_selection != Some(selection) {
        *last_selection = Some(selection);
    }
    *remaining = (*remaining - time.delta_secs()).max(0.0);
    *visibility = if *remaining > 0.0 {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
}

fn update_inventory_hover_tooltip(
    state: Res<State<GameState>>,
    dragged: Res<DraggedBlock>,
    windows: Query<&Window>,
    pickers: Query<(&Hovered, &InventoryBlockPicker)>,
    mut tooltip: Query<(&mut Text, &mut Node, &mut Visibility), With<InventoryHoverTooltip>>,
) {
    let Ok((mut text, mut node, mut visibility)) = tooltip.single_mut() else {
        return;
    };
    if *state.get() != GameState::Inventory || dragged.0.is_some() {
        *visibility = Visibility::Hidden;
        return;
    }
    let Some(block) = pickers
        .iter()
        .find_map(|(hovered, picker)| hovered.0.then_some(picker.0))
    else {
        *visibility = Visibility::Hidden;
        return;
    };
    let Ok(window) = windows.single() else {
        *visibility = Visibility::Hidden;
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        *visibility = Visibility::Hidden;
        return;
    };

    let label = block.label();
    if text.0 != label {
        text.0 = label.to_owned();
    }
    let tooltip_width = (label.len() as f32 * 9.0 + 24.0).max(64.0);
    let tooltip_height = 32.0;
    let left = if cursor.x + 16.0 + tooltip_width > window.width() {
        cursor.x - tooltip_width - 16.0
    } else {
        cursor.x + 16.0
    }
    .clamp(0.0, (window.width() - tooltip_width).max(0.0));
    let top = if cursor.y + 16.0 + tooltip_height > window.height() {
        cursor.y - tooltip_height - 16.0
    } else {
        cursor.y + 16.0
    }
    .clamp(0.0, (window.height() - tooltip_height).max(0.0));
    node.left = Val::Px(left);
    node.top = Val::Px(top);
    *visibility = Visibility::Visible;
}

fn update_inventory_drag_preview(
    state: Res<State<GameState>>,
    mut dragged: ResMut<DraggedBlock>,
    icons: Res<BlockIconAssets>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut preview: Query<(&mut ImageNode, &mut Node, &mut Visibility), With<InventoryDragPreview>>,
    mut last_block: Local<Option<Block>>,
) {
    let Ok((mut image, mut node, mut visibility)) = preview.single_mut() else {
        return;
    };
    if *state.get() != GameState::Inventory {
        dragged.0 = None;
    }
    let Some(block) = dragged.0 else {
        *visibility = Visibility::Hidden;
        *last_block = None;
        return;
    };
    let Ok(window) = windows.single() else {
        *visibility = Visibility::Hidden;
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        *visibility = Visibility::Hidden;
        return;
    };

    if *last_block != Some(block) {
        image.image = icons.get(block);
        *last_block = Some(block);
    }
    node.left = Val::Px(cursor.x + 14.0);
    node.top = Val::Px(cursor.y + 14.0);
    *visibility = Visibility::Visible;
}

fn update_sensitivity_readout(
    settings: Res<GameSettings>,
    mut readout: Query<&mut Text, With<SensitivityReadout>>,
) {
    if !settings.is_changed() {
        return;
    }
    for mut text in &mut readout {
        text.0 = format!("Mouse sensitivity: {:.4}", settings.mouse_sensitivity);
    }
}

fn update_render_distance_readout(
    settings: Res<GameSettings>,
    mut readout: Query<&mut Text, With<RenderDistanceReadout>>,
) {
    if !settings.is_changed() {
        return;
    }
    for mut text in &mut readout {
        text.0 = format!("{} chunks", settings.render_distance);
    }
}

fn update_anti_aliasing_readout(
    settings: Res<GameSettings>,
    mut readout: Query<&mut Text, With<AntiAliasingReadout>>,
) {
    if !settings.is_changed() {
        return;
    }
    let label = match settings.anti_aliasing_samples {
        1 => "Off".to_owned(),
        samples => format!("{samples}×"),
    };
    for mut text in &mut readout {
        text.0.clone_from(&label);
    }
}

fn update_vsync_toggle_label(
    settings: Res<GameSettings>,
    mut labels: Query<&mut Text, With<VsyncToggleLabel>>,
) {
    let label = if settings.vsync_enabled {
        "V-Sync: On"
    } else {
        "V-Sync: Off (Uncapped)"
    };
    for mut text in &mut labels {
        if text.0 != label {
            text.0 = label.to_owned();
        }
    }
}

fn update_adaptive_sync_toggle_label(
    settings: Res<GameSettings>,
    mut labels: Query<&mut Text, With<AdaptiveSyncToggleLabel>>,
) {
    let label = if settings.adaptive_sync_enabled {
        "Adaptive V-Sync: On"
    } else {
        "Adaptive V-Sync: Off"
    };
    for mut text in &mut labels {
        if text.0 != label {
            text.0 = label.to_owned();
        }
    }
}

fn update_hdr_toggle_label(
    settings: Res<GameSettings>,
    mut labels: Query<&mut Text, With<HdrToggleLabel>>,
) {
    let label = if settings.hdr_enabled {
        "HDR Rendering: On"
    } else {
        "HDR Rendering: Off"
    };
    for mut text in &mut labels {
        if text.0 != label {
            text.0 = label.to_owned();
        }
    }
}

fn update_display_mode_readout(
    settings: Res<GameSettings>,
    mut readouts: Query<&mut Text, With<DisplayModeReadout>>,
) {
    let label = match settings.display_mode {
        DisplayMode::Windowed => "Windowed",
        DisplayMode::Borderless => "Borderless Windowed",
        DisplayMode::Fullscreen => "Exclusive Fullscreen",
    };
    for mut text in &mut readouts {
        if text.0 != label {
            text.0 = label.to_owned();
        }
    }
}

fn update_resolution_readout(
    settings: Res<GameSettings>,
    windows: Query<&Window, With<PrimaryWindow>>,
    monitors: Query<&Monitor, With<PrimaryMonitor>>,
    mut readouts: Query<&mut Text, With<ResolutionReadout>>,
) {
    let monitor = monitors.single().ok();
    let fallback = windows
        .single()
        .map(|window| window.resolution.physical_size())
        .unwrap_or(UVec2::new(1280, 720));
    let (width, height) = selected_resolution(settings.resolution_index, monitor, fallback);
    let label = if settings.resolution_index == 0 {
        format!("Native ({width} × {height})")
    } else {
        format!("{width} × {height}")
    };
    for mut text in &mut readouts {
        if text.0 != label {
            text.0.clone_from(&label);
        }
    }
}

fn on_graphics_slider_change(
    event: On<ValueChange<f32>>,
    sliders: Query<&GraphicsSlider>,
    mut settings: ResMut<GameSettings>,
    mut commands: Commands,
) {
    let Ok(GraphicsSlider(kind)) = sliders.get(event.source) else {
        return;
    };
    let value = match kind {
        GraphicsSliderKind::FieldOfView => {
            settings.field_of_view_degrees = event.value.clamp(60.0, 120.0);
            settings.field_of_view_degrees
        }
        GraphicsSliderKind::Brightness => {
            settings.brightness = event.value.clamp(0.5, 1.5);
            settings.brightness
        }
        GraphicsSliderKind::Contrast => {
            settings.contrast = event.value.clamp(0.5, 1.5);
            settings.contrast
        }
        GraphicsSliderKind::Gamma => {
            settings.gamma = event.value.clamp(0.5, 2.0);
            settings.gamma
        }
    };
    commands.entity(event.source).insert(SliderValue(value));
}

fn update_graphics_slider_readouts(
    settings: Res<GameSettings>,
    mut sliders: Query<(Entity, &GraphicsSlider, &SliderValue)>,
    mut labels: Query<(&GraphicsSliderReadout, &mut Text)>,
    mut commands: Commands,
) {
    for (entity, GraphicsSlider(kind), value) in &mut sliders {
        let desired = graphics_slider_value(&settings, *kind);
        if (value.0 - desired).abs() > 0.001 {
            commands.entity(entity).insert(SliderValue(desired));
        }
    }
    for (GraphicsSliderReadout(kind), mut text) in &mut labels {
        let value = graphics_slider_value(&settings, *kind);
        let label = if *kind == GraphicsSliderKind::FieldOfView {
            format!("{value:.0}°")
        } else {
            format!("{value:.2}")
        };
        if text.0 != label {
            text.0 = label;
        }
    }
}

fn update_calibration_pattern(
    settings: Res<GameSettings>,
    mut swatches: Query<(&CalibrationSwatch, &mut BackgroundColor)>,
) {
    if !settings.is_changed() {
        return;
    }

    for (CalibrationSwatch(sample), mut background) in &mut swatches {
        let exposed = sample * settings.brightness;
        let contrasted = ((exposed - 0.5) * settings.contrast + 0.5).clamp(0.0, 1.0);
        let calibrated = contrasted.powf(settings.gamma.recip()).clamp(0.0, 1.0);
        background.0 = Color::srgb(calibrated, calibrated, calibrated);
    }
}

fn graphics_slider_value(settings: &GameSettings, kind: GraphicsSliderKind) -> f32 {
    match kind {
        GraphicsSliderKind::FieldOfView => settings.field_of_view_degrees,
        GraphicsSliderKind::Brightness => settings.brightness,
        GraphicsSliderKind::Contrast => settings.contrast,
        GraphicsSliderKind::Gamma => settings.gamma,
    }
}

fn update_save_status(
    status: Res<SaveStatus>,
    mut labels: Query<&mut Text, With<SaveStatusLabel>>,
) {
    if !status.is_changed() {
        return;
    }
    for mut text in &mut labels {
        text.0.clone_from(&status.0);
    }
}

fn update_inventory_hint(
    state: Res<State<GameState>>,
    hotbar: Res<Hotbar>,
    mut hint: Query<&mut Text, With<InventoryHint>>,
) {
    if *state.get() != GameState::Inventory && !hotbar.is_changed() {
        return;
    }
    for mut text in &mut hint {
        text.0 = format!(
            "Slot {} currently holds {}",
            hotbar.selected + 1,
            hotbar.selected_block().label()
        );
    }
}

fn update_debug_text(
    time: Res<Time>,
    state: Res<State<GameState>>,
    hotbar: Res<Hotbar>,
    selection: Res<Selection>,
    world: Res<WorldData>,
    player: Query<(&Transform, &PlayerVelocity), With<Player>>,
    mut debug_text: Query<&mut Text, With<DebugText>>,
    mut smoothed_fps: Local<f32>,
) {
    if *state.get() != GameState::Playing {
        return;
    }
    let Ok((transform, velocity)) = player.single() else {
        return;
    };
    let Ok(mut text) = debug_text.single_mut() else {
        return;
    };
    let delta = time.delta_secs();
    if delta > 0.0 {
        let instant_fps = 1.0 / delta;
        let smoothing = 1.0 - (-delta / 0.5).exp();
        *smoothed_fps = if *smoothed_fps == 0.0 {
            instant_fps
        } else {
            *smoothed_fps + (instant_fps - *smoothed_fps) * smoothing
        };
    }
    let position = transform.translation;
    let chunk_x = (position.x.floor() as i32).div_euclid(CHUNK_SIZE);
    let chunk_z = (position.z.floor() as i32).div_euclid(CHUNK_SIZE);
    let target = selection
        .0
        .map(|hit| world.block_at(hit.block).label())
        .unwrap_or("None");
    text.0 = format!(
        "FPS: {:.0}\nXYZ: {:.2} / {:.2} / {:.2}\nChunk: {}, {} | Loaded: {}\nHotbar: {} | Target: {} | Speed: {:.1} | F3 debug, E inventory",
        *smoothed_fps,
        position.x,
        position.y,
        position.z,
        chunk_x,
        chunk_z,
        world.loaded_count(),
        hotbar.selected_block().label(),
        target,
        velocity.0.length()
    );
}

fn block_tint(block: Block) -> Color {
    let color = block.face_color(2);
    Color::linear_rgba(color.x * 0.55, color.y * 0.55, color.z * 0.55, 1.0)
}

const BLOCK_ICON_SIZE: u32 = 48;

fn create_block_icons(images: &mut Assets<Image>) -> BlockIconAssets {
    let mut icons = HashMap::new();
    for block in Block::SOLID_PICKER {
        icons.insert(block, images.add(render_block_icon(block)));
    }
    BlockIconAssets(icons)
}

fn render_block_icon(block: Block) -> Image {
    let mut pixels = vec![0; (BLOCK_ICON_SIZE * BLOCK_ICON_SIZE * 4) as usize];
    let top = [[24, 3], [43, 13], [24, 23], [5, 13]];
    let left = [[5, 13], [24, 23], [24, 44], [5, 34]];
    let right = [[43, 13], [24, 23], [24, 44], [43, 34]];

    fill_textured_quad(&mut pixels, BLOCK_ICON_SIZE, top, block, 2, 1.0);
    fill_textured_quad(&mut pixels, BLOCK_ICON_SIZE, left, block, 1, 0.78);
    fill_textured_quad(&mut pixels, BLOCK_ICON_SIZE, right, block, 0, 0.60);

    let mut image = Image::new(
        Extent3d {
            width: BLOCK_ICON_SIZE,
            height: BLOCK_ICON_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

fn linear_to_srgb_byte(value: f32) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let srgb = if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (srgb * 255.0).round() as u8
}

fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn fill_textured_quad(
    pixels: &mut [u8],
    width: u32,
    points: [[i32; 2]; 4],
    block: Block,
    face: usize,
    shade: f32,
) {
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    fill_textured_triangle(
        pixels,
        width,
        [points[0], points[1], points[2]],
        [uvs[0], uvs[1], uvs[2]],
        block,
        face,
        shade,
    );
    fill_textured_triangle(
        pixels,
        width,
        [points[0], points[2], points[3]],
        [uvs[0], uvs[2], uvs[3]],
        block,
        face,
        shade,
    );
}

fn fill_textured_triangle(
    pixels: &mut [u8],
    width: u32,
    points: [[i32; 2]; 3],
    uvs: [[f32; 2]; 3],
    block: Block,
    face: usize,
    shade: f32,
) {
    let min_x = points
        .iter()
        .map(|point| point[0])
        .min()
        .unwrap_or(0)
        .max(0);
    let min_y = points
        .iter()
        .map(|point| point[1])
        .min()
        .unwrap_or(0)
        .max(0);
    let max_x = points
        .iter()
        .map(|point| point[0])
        .max()
        .unwrap_or(0)
        .min(width as i32 - 1);
    let max_y = points
        .iter()
        .map(|point| point[1])
        .max()
        .unwrap_or(0)
        .min(width as i32 - 1);
    let a = points[0].map(|value| value as f32);
    let b = points[1].map(|value| value as f32);
    let c = points[2].map(|value| value as f32);
    let area = triangle_edge(a, b, c);
    if area.abs() < f32::EPSILON {
        return;
    }
    let face_color = block.face_color(face);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let point = [x as f32 + 0.5, y as f32 + 0.5];
            let weight_a = triangle_edge(b, c, point) / area;
            let weight_b = triangle_edge(c, a, point) / area;
            let weight_c = triangle_edge(a, b, point) / area;
            if weight_a >= 0.0 && weight_b >= 0.0 && weight_c >= 0.0 {
                let u = uvs[0][0] * weight_a + uvs[1][0] * weight_b + uvs[2][0] * weight_c;
                let v = uvs[0][1] * weight_a + uvs[1][1] * weight_b + uvs[2][1] * weight_c;
                let texture_x = (u.clamp(0.0, 0.9999) * 16.0) as u32;
                let texture_y = (v.clamp(0.0, 0.9999) * 16.0) as u32;
                let texture = srgb_to_linear(
                    block_texture_pixel_for_face(block, face, texture_x, texture_y) as f32 / 255.0,
                );
                let offset = ((y as u32 * width + x as u32) * 4) as usize;
                pixels[offset] = linear_to_srgb_byte(face_color.x * shade * texture);
                pixels[offset + 1] = linear_to_srgb_byte(face_color.y * shade * texture);
                pixels[offset + 2] = linear_to_srgb_byte(face_color.z * shade * texture);
                pixels[offset + 3] = 255;
            }
        }
    }
}

fn triangle_edge(a: [f32; 2], b: [f32; 2], point: [f32; 2]) -> f32 {
    (point[0] - a[0]) * (b[1] - a[1]) - (point[1] - a[1]) * (b[0] - a[0])
}
