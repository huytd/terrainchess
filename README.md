# Terrain Chess

A pixel-art chess roguelike where hills, cliffs, water and caves change how pieces move.

<img width="1000" height="516" alt="image" src="https://github.com/user-attachments/assets/7c179462-47d6-4707-88d5-5e11a56cf6d6" />

Specs live in [`specs/`](specs/README.md): [game design](specs/game-design.md), [project plan](specs/project-plan.md) and asset specs.

## Layout

| Path | What |
|---|---|
| `specs/` | Specifications: game design, project plan, sprite specs |
| `crates/core` (`tc_core`) | Rules, terrain, move generation, board generator. No Bevy. |
| `crates/ai` (`tc_ai`) | Alpha-beta AI over `tc_core`, resumable in time slices. No Bevy. |
| `crates/run` (`tc_run`) | Levels, profile, items, draft rewards, saves. No Bevy. |
| `crates/game` | Bevy 0.19 app: rendering, input, HUD (native + web) |
| `tools/process_sprites.py` | Turns `assets/spritesheet.jpg` into `assets/atlas.png` + `atlas.ron` |
| `web/` | Trunk page for the browser build |

## Run

```sh
cargo run -p terrainchess            # native (Linux needs libwayland-dev, libxkbcommon-dev, libudev-dev)
cargo test -p tc_core                # perft, terrain rule fixtures, generator checks
cargo test -p tc_ai                  # AI tactics and symmetry checks
cargo run -p tc_core --example dump -- 42 8   # print a generated board
cargo run --release -p tc_ai --example selfplay -- 20 8 100   # AI vs AI on 20 boards

cd web && trunk serve                # browser build at http://localhost:8080
cd web && trunk build --release      # optimised browser build in web/dist (about half the size)
```

You play the Ashen Sun against the AI by default. Controls: click to select and move · right-drag
or Q/E to turn the board · right-drag up/down or Z/X to tilt · middle-drag / WASD to pan · wheel
to zoom · U undo · F swaps sides with the AI ·
- /= changes AI level (0–7) · A toggles threat markers · M mute · Menu button or Esc opens the level select menu ·
spells: 15-card deck with 3-card hand; click a card or 1–3 to arm, then a highlighted square (Esc cancels); spends the card and auto-draws up to 3 when empty; D discards the selected card (5 per match).
Picking a level opens its deck screen: tap two cards to swap them (the reserve holds owned cards beyond 15), toggle Shuffle to fix the draw order, Enter starts.
Pawns promote to a queen automatically. Red frame + sword = your piece can be captured, amber frame = the piece that threatens it.

On phones and tablets (web build): tap to select and move · drag to pan · pinch to zoom · twist two
fingers to turn the board · two-finger vertical drag to tilt.

## Deploys

Web builds and deployments are automated with GitHub Actions:
- Pull requests receive a sticky comment with a Vercel preview deployment URL.
- Merges to `main` deploy to production.

Deployments require the `VERCEL_TOKEN` repository secret.

## Art

After regenerating or editing `assets/spritesheet.jpg`, rebuild the atlas (needs Pillow, numpy, scipy):

```sh
python3 tools/process_sprites.py --preview /tmp/atlas_preview.png
```

The region table at the top of the script maps sheet cells to sprite names.

## Credits

- UI font: **Jacquard 24** by The Soft Type Project (Sarah Cadigan-Fried), licensed under the SIL Open Font License, Version 1.1 (`assets/fonts/OFL.txt`).

