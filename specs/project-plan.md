# Terrain Chess — Project Plan

For game rules, terrain mechanics, AI, and rendering design, see [game-design.md](game-design.md).

A 2D pixel-art chess roguelike (top-down 3/4 view) built with Bevy. Sprites are specified in [assets-sprites.md](assets-sprites.md). It runs natively and on the web (WASM). The board is terrain with hills, cliffs, water and caves, and that terrain changes how pieces move. You play a **run**: a chain of matches against an AI. Each win adds power-ups, piece upgrades and spells that stay with you for the rest of the run. One loss ends the run and everything resets.

---

## 1. Tech stack

| Concern | Choice | Notes |
|---|---|---|
| Engine | `bevy` 0.19 (latest stable; 0.20 is RC) | Pin the version and upgrade on purpose |
| Rendering | Bevy 2D: `TilemapChunk` for ground/cliff layers, `Sprite` + `TextureAtlas` for pieces/props/FX | `ImagePlugin::default_nearest()` |
| Camera | `Camera2d` with pan/zoom at **integer zoom steps only** (1×, 2×, 3×…) | Keeps pixels crisp |
| Picking | Bevy built-in `bevy_picking` (sprite picking) + a tile lookup from the cursor position | Click a tile or piece |
| UI | `bevy_ui` for the game HUD, `bevy_egui` for debug/dev panels only | Styled bevy_ui fits the art better |
| Terrain noise | `noise` (or `fastnoise-lite`) | Seeded, deterministic |
| RNG | `rand` + `rand_chacha` (seeded) | Same seed gives the same board and run |
| Serialization | `serde` + `ron` / `serde_json` | Run saves and data files for items/spells |
| Save storage | Native: file in the config dir · Web: `localStorage` via `web-sys` | Behind one small trait |
| Web build | `trunk` (or `wasm-bindgen-cli` + `wasm-opt`) | `wasm32-unknown-unknown` is already installed |
| Web renderer | WebGL2 | 2D sprites need nothing that WebGL2 lacks |
| Asset pipeline | `tools/process_sprites.py` (Pillow) | Chroma-keys, downscales 4×, snaps to the palette and slices generated sheets (see assets-sprites.md §0) |


## 2. Code structure

Keep the rules separate from the engine so they can be tested headlessly and reused by the AI.

```
terrainchess/
├─ Cargo.toml              (workspace)
├─ crates/
│  ├─ core/                ← pure Rust, NO bevy dependency
│  │  ├─ board.rs          grid, coords, sizes (8/16/32)
│  │  ├─ terrain.rs        Tile { height, kind, feature }, cave links
│  │  ├─ piece.rs          PieceKind, Color, per-piece MoveProfile
│  │  ├─ movegen.rs        terrain-aware pseudo-legal + legal moves
│  │  ├─ rules.rs          check, mate, stalemate, promotion, draws
│  │  ├─ modifiers.rs      hooks that items/spells use to change rules
│  │  └─ gen/              terrain generator + validator
│  ├─ ai/                  alpha-beta search over `core`
│  ├─ run/                 roguelike: run state, items, rewards, saves
│  └─ game/                bevy app: rendering, input, UI, audio, VFX
│     └─ src/main.rs       (native + wasm entry)
└─ web/                    index.html, trunk config, loading screen
```

A key design choice is to make movement **data-driven**. Each piece type has a `MoveProfile` (for example `max_climb`, `max_drop`, `water: Forbidden | Stops | Free`, `slide_range`, `jump_height_limit`). Most upgrades just change these numbers. Only special effects need custom code, through a small `Modifier` trait with hooks such as `on_movegen`, `on_capture` and `on_turn_start`.

## 3. Milestones

| # | Milestone | Status | Done when |
|---|---|---|---|
| M0 | Workspace scaffold + native & web build pipeline | Done | An empty Bevy scene runs natively and in the browser via `trunk serve` |
| M1 | `core`: standard chess on a flat 8×8 board | Done | Perft tests pass |
| M2 | Terrain model, generator and terrain move rules | Done | Rule-fixture tests pass, and the validator rejects unfair or sealed boards |
| M3 | 2D board rendering (placeholder coloured tiles + letters for pieces) + height/cliff drawing + camera + picking + playable hotseat | Done | Two humans can play a full game on generated terrain |
| M4 | AI opponent | Done | It plays legal terrain moves within a time budget, also on the web |
| M5 | Roguelike loop: floors, reward draft, items, spells, pickups, save/reset | Done | A complete run can be won or lost, and the save persists and is wiped when you lose |
| M6 | 16×16 / 32×32 armies and scaling, balance pass | Done (60 fps at all sizes incl. AI turns; 8×8 self-play 7–3–10) | All sizes can be played at a steady 60 fps on the web |
| M7 | Art & juice: generate the atlas from assets-sprites.md, run the processing script, add autotiling, VFX, audio, menus | Done (3D pixel-art board and island, environment art, water foam, VFX, synthesized SFX, title menu) | It matches the look of the pixel-art reference image |
| M8 | Web deploy (static hosting) + size optimisation (`wasm-opt`, `opt-level="z"` for wasm) | Done (Vercel; 6.2 MB brotli, starts in 1.3 s, 3.4 s at 20 Mbps) | A public URL loads in under 10 s |
| M9 | Overworld map, camps, hero tokens, day loop, permanent deck, and deployment phase | Pending | A 2-hero playable overworld slice matching the design spec |

## 4. Risks

- **Balance.** Terrain plus items can break chess (unstoppable openings, kings you can't mate). Mitigate with symmetric generation, the validator, a draw-by-move-limit rule, and a quick self-play harness (AI vs AI over 1000 seeds) to spot broken items.
- **AI speed on 32×32 on the web.** Use time slicing, a lower depth, and the skirmish army style.
- **Readability.** Cliffs can hide pieces behind them. Mitigate with the limit of 3 height levels, silhouette outlines, and the Alt height badges.
- **Generated art consistency.** Image models drift between sheets and misalign grids. Keeping everything on one sheet keeps the style consistent. Mitigate grid drift with the palette-snapping script and a blob-detecting slicer for sprite regions, allow per-cell regeneration/inpainting for bad cells, and use placeholder art until M7.
- **Bevy version churn.** Pin the version and keep `core`/`ai`/`run` free of Bevy so upgrades only touch `game`.

## 5. Open decisions

1. **Opponent:** single-player vs AI only (fits the roguelike), or also local hotseat? → *Recommend AI for runs, with hotseat as a sandbox mode.*
2. **32×32 army style:** big army or skirmish? → *Recommend skirmish.*
3. **Board size in a run:** fixed at run start, or growing per floor? → *Recommend chosen at start, with a "Grand Run" option that grows.*
4. **Casting a spell uses the turn?** → *Recommend yes, except for Quick spells.*
5. **Any meta-progression across runs** (unlocking new items to the pool)? You said a loss resets everything, so the default is **none**.
