# VoxCraft

VoxCraft is a first-person voxel sandbox built from original Rust code with Bevy. Terrain is generated from a fresh seed for each new game and streamed in 16 × 16 × 16 chunks. Pass `--seed` to use a fixed terrain seed. Each chunk is drawn as a combined mesh with hidden block faces removed.

## Requirements

- Rust 1.97.1 or newer (the project uses Bevy 0.20)
- A desktop GPU with a Vulkan, DirectX 12, or Metal capable driver

## Build and run

```sh
cargo run --release
```

Optional arguments:

```sh
cargo run --release -- --seed=12345 --render-distance=8 --mouse-sensitivity=0.0022
```

Render distance is measured in horizontal chunks and is clamped to 2–16. Mouse sensitivity is clamped to 0.0001–0.02. Defaults are a random terrain seed, render distance 8, and mouse sensitivity `0.0022`.

## Controls

| Input | Action |
| --- | --- |
| Main menu | Start Game, load a saved game, open Options, or Quit |
| W / A / S / D | Move |
| Mouse | Look |
| Mouse wheel | Cycle through hotbar slots |
| Space | Jump |
| Double-tap Space | Toggle flight |
| Space while flying | Rise |
| Left Ctrl while flying | Descend; touching down exits flight |
| Left Shift | Sprint and move faster while flying |
| Left mouse | Break targeted block (bedrock is unbreakable) |
| Right mouse | Place selected block; oak logs align to the face you place against |
| 1–5 | Select a hotbar slot |
| E | Open the block picker for the selected hotbar slot |
| Escape | Open the pause menu; save, resume, change options, or return to the main menu |
| Options | Use − / + to change mouse sensitivity; changes apply immediately |
| F3 | Toggle coordinates and chunk debug information |

## Architecture

- `src/world/` defines block types, negative-safe coordinate conversion, seeded terrain, chunk storage, streaming, and combined exposed-face meshes.
- `src/player/` contains the first-person camera, mouse look, movement, gravity, jumping, sprinting, flight, and an axis-aligned collision controller.
- `src/interaction/` contains a voxel grid raycast, block selection, breaking particles, placement, and outline drawing.
- `src/rendering/` sets up the directional sunlight, ambient light, world material, and blue sky clear color.
- `src/ui/` creates the main menu, mouse sensitivity options, pause menu, image-only square hotbar and block picker, crosshair, and F3 debug text. Block cube icons are generated from the block face colors at startup.

Chunk generation and mesh assembly run on Bevy's async compute pool; completed meshes are uploaded in a paced main-thread queue. Returning to the main menu unloads terrain, cancels pending chunk work, clears edits, and resets the player spawn. Blocks use a generated 16 × 16 pixel texture atlas with nearest-neighbor sampling and vertex colors for face shading, so the project has no external asset downloads or copyrighted game assets.

The pause menu saves one local world slot, including its terrain seed, block edits, player position and view direction, and hotbar. Loading regenerates the terrain from that saved seed, so the landscape is the same each time; saved block edits are reapplied afterward. The main menu loads that save. On Windows, the save file is stored at `%APPDATA%\VoxCraft\save.json`.

## Checks

Run the automated checks with:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

The tests cover negative coordinate mapping, chunk indexing and boundaries, deterministic generation, terrain layers, exposed-face meshing across chunk boundaries, block edits, block raycasting, and hotbar selection. Linux and macOS runtime behavior has not been tested in this workspace.
