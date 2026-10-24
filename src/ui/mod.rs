use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    picking::Pickable,
    picking::hover::Hovered,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    ui::Pressed,
    ui_widgets::{Activate, Button},
    window::{MonitorSelection, PresentMode, PrimaryWindow, WindowMode},
};

use crate::{
    game::{
        GameSettings, GameState, OptionsReturn, SaveFile, SaveStatus, load_from_disk, save_to_disk,
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
struct DebugText;

#[derive(Component)]
struct SensitivityReadout;

#[derive(Component)]
struct RenderDistanceReadout;

#[derive(Component)]
struct AntiAliasingReadout;

#[derive(Component)]
struct InventoryHint;

#[derive(Component)]
struct HotbarSlot(usize);

#[derive(Component)]
struct HotbarIcon(usize);

#[derive(Component)]
struct InventoryBlockPicker(Block);

#[derive(Component)]
struct FullscreenToggleLabel;

#[derive(Component)]
struct VsyncToggleLabel;

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
    ToggleVsync,
    ToggleFullscreen,
    SaveGame,
    LoadGame,
    SelectHotbar(usize),
    PickBlock(Block),
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
                    update_sensitivity_readout,
                    update_render_distance_readout,
                    update_anti_aliasing_readout,
                    update_vsync_toggle_label,
                    update_fullscreen_toggle_label,
                    update_save_status,
                    update_inventory_hint,
                    update_debug_text,
                )
                    .chain(),
            );
    }
}

fn setup_ui(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let icons = create_block_icons(&mut images);
    commands.insert_resource(icons.clone());
    setup_main_menu(&mut commands);
    setup_options_menu(&mut commands);
    setup_pause_menu(&mut commands);
    setup_inventory(&mut commands, &icons);
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
        panel = Some(
            screen
                .spawn((
                    Node {
                        width: Val::Px(panel_width),
                        max_width: Val::Percent(92.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(12.0),
                        padding: UiRect::all(Val::Px(28.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(PANEL_COLOR),
                    BorderColor::all(Color::srgba(0.69, 0.78, 0.73, 0.24)),
                ))
                .id(),
        );
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
        500.0,
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
        add_label(panel, "Render Distance", 14.0, MUTED_TEXT_COLOR);
        panel
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|row| {
                add_button(
                    row,
                    "−",
                    UiAction::RenderDistanceStep(-1),
                    72.0,
                    46.0,
                    None,
                    None,
                );
                row.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(17.0),
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    TextLayout::justify(Justify::Center),
                    Node {
                        width: Val::Px(230.0),
                        height: Val::Px(46.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    RenderDistanceReadout,
                ));
                add_button(
                    row,
                    "+",
                    UiAction::RenderDistanceStep(1),
                    72.0,
                    46.0,
                    None,
                    None,
                );
            });
        add_label(panel, "Anti-Aliasing", 14.0, MUTED_TEXT_COLOR);
        panel
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|row| {
                add_button(
                    row,
                    "−",
                    UiAction::AntiAliasingStep(-1),
                    72.0,
                    46.0,
                    None,
                    None,
                );
                row.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(17.0),
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    TextLayout::justify(Justify::Center),
                    Node {
                        width: Val::Px(230.0),
                        height: Val::Px(46.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    AntiAliasingReadout,
                ));
                add_button(
                    row,
                    "+",
                    UiAction::AntiAliasingStep(1),
                    72.0,
                    46.0,
                    None,
                    None,
                );
            });
        add_button(
            panel,
            "V-Sync: Off",
            UiAction::ToggleVsync,
            340.0,
            46.0,
            None,
            None,
        );
        add_button(
            panel,
            "Enter Fullscreen",
            UiAction::ToggleFullscreen,
            340.0,
            50.0,
            None,
            None,
        );
        add_button(
            panel,
            "Back",
            UiAction::BackFromOptions,
            340.0,
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

fn setup_inventory(commands: &mut Commands, icons: &BlockIconAssets) {
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
            "Choose a block for the selected hotbar slot. Press Escape to close.",
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
    if matches!(action, UiAction::ToggleFullscreen) {
        button.insert(FullscreenToggleLabel);
    }
    if matches!(action, UiAction::ToggleVsync) {
        button.insert(VsyncToggleLabel);
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
        ));
    });
    button.observe(on_menu_action);
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
        MenuButtonAction(UiAction::PickBlock(block)),
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
        ));
    });
    button.observe(on_menu_action);
}

fn on_menu_action(
    event: On<Activate>,
    actions: Query<&MenuButtonAction>,
    mut next_state: ResMut<NextState<GameState>>,
    mut options_return: ResMut<OptionsReturn>,
    mut settings: ResMut<GameSettings>,
    mut hotbar: ResMut<Hotbar>,
    mut world: ResMut<WorldData>,
    player: Query<&Transform, With<Player>>,
    mut pending_player_restore: ResMut<PendingPlayerRestore>,
    mut save_status: ResMut<SaveStatus>,
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,
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
        }
        UiAction::RenderDistanceStep(direction) => {
            settings.adjust_render_distance(direction as i32);
        }
        UiAction::AntiAliasingStep(direction) => settings.step_anti_aliasing(direction),
        UiAction::ToggleVsync => {
            settings.vsync_enabled = !settings.vsync_enabled;
            if let Ok(mut window) = primary_window.single_mut() {
                window.present_mode = if settings.vsync_enabled {
                    PresentMode::AutoVsync
                } else {
                    PresentMode::AutoNoVsync
                };
            }
        }
        UiAction::ToggleFullscreen => {
            if let Ok(mut window) = primary_window.single_mut() {
                if matches!(window.mode, WindowMode::Windowed) {
                    window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
                } else {
                    window.mode = WindowMode::Windowed;
                    window.set_maximized(true);
                }
            }
        }
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
        UiAction::SelectHotbar(slot) => hotbar.select_slot(slot),
        UiAction::PickBlock(block) => {
            hotbar.replace_selected(block);
            next_state.set(GameState::Playing);
        }
    }
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
    windows: Query<&Window>,
    pickers: Query<(&Hovered, &InventoryBlockPicker)>,
    mut tooltip: Query<(&mut Text, &mut Node, &mut Visibility), With<InventoryHoverTooltip>>,
) {
    let Ok((mut text, mut node, mut visibility)) = tooltip.single_mut() else {
        return;
    };
    if *state.get() != GameState::Inventory {
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
    if !settings.is_changed() {
        return;
    }
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

fn update_fullscreen_toggle_label(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut labels: Query<&mut Text, With<FullscreenToggleLabel>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let label = if matches!(window.mode, WindowMode::Windowed) {
        "Enter Fullscreen"
    } else {
        "Exit Fullscreen"
    };
    for mut text in &mut labels {
        if text.0 != label {
            text.0 = label.to_owned();
        }
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
