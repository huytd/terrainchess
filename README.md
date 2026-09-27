# Terrain Chess

A pixel-art chess roguelike where hills, cliffs, water and caves change how pieces move.
Design: [PLAN.md](PLAN.md) · Art spec: [SPRITES.md](SPRITES.md).

## Layout

| Path | What |
|---|---|
| `crates/core` (`tc_core`) | Rules, terrain, move generation, board generator. No Bevy. |
| `crates/game` | Bevy 0.19 app: rendering, input, HUD (native + web) |
| `tools/process_sprites.py` | Turns `assets/spritesheet.jpg` into `assets/atlas.png` + `atlas.ron` |
| `web/` | Trunk page for the browser build |

## Run

```sh
cargo run -p terrainchess            # native (Linux needs libwayland-dev, libxkbcommon-dev, libudev-dev)
cargo test -p tc_core                # perft, terrain rule fixtures, generator checks
cargo run -p tc_core --example dump -- 42 8   # print a generated board

cd web && trunk serve                # browser build at http://localhost:8080
```

Controls: click to select and move · right-drag / WASD to pan · wheel to zoom · hold Alt for tile
heights · U undo · N new board · 1/2/3 for 8×8, 16×16, 32×32.

## Art

After regenerating or editing `assets/spritesheet.jpg`, rebuild the atlas (needs Pillow, numpy, scipy):

```sh
python3 tools/process_sprites.py --preview /tmp/atlas_preview.png
```

The region table at the top of the script maps sheet cells to sprite names.
