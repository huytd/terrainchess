# Terrain Chess — Plan

A 2D pixel-art chess roguelike (top-down 3/4 view) built with Bevy. Sprites are specified in `SPRITES.md`. It runs natively and on the web (WASM). The board is terrain with hills, cliffs, water and caves, and that terrain changes how pieces move. You play a **run**: a chain of matches against an AI. Each win adds power-ups, piece upgrades and spells that stay with you for the rest of the run. One loss ends the run and everything resets.

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
| Asset pipeline | `tools/process_sprites.py` (Pillow) | Chroma-keys, downscales 4×, snaps to the palette and slices generated sheets (see SPRITES.md §0) |


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

## 3. Terrain model

The world is a **heightmap** (levels 0–3) with an optional cave network. Chess needs exactly one standable surface per square, so each square has a single height:

```
Tile {
  height: u8,          // 0..=3 (kept small so 2D cliffs stay readable)
  kind: Grass | Stone | Sand | ShallowWater | DeepWater | Lava? | Void,
  feature: None | CaveEntrance(link_id) | Pickup(id) | Obstacle(Rock/Tree)
}
```

- **Hills and cliffs** are height differences between neighbouring columns.
- **Deep caves** are pairs (or small networks) of cave entrances cut into cliff sides and joined by tunnels. They are drawn as arched openings in cliff faces.
- **Water** fills columns below the water level. Water is shallow when the column is one block below the water level and deep when it is lower.

### Generation, per match

1. Seeded fBm noise builds the heightmap. The generator adds ridges and valleys, and the parameters scale with difficulty and board size.
2. **Mirror symmetry** across the middle rank so neither side gets an unfair board. Mild asymmetry can come later as an "unfair" difficulty modifier.
3. Flatten the **home rows** (2 ranks on 8×8, more on bigger boards) so the starting setup is legal.
4. Carve water basins, place cave pairs, and scatter obstacles and pickups (pickups favour hilltops and cave ends).
5. **Validator:** a flood-fill checks that every piece type can reach the opponent's half, the kings aren't sealed in, and the position is not already check. If validation fails, it regenerates with the next seed.

## 4. Terrain movement rules (first draft, tune by playtesting)

| Rule | Effect |
|---|---|
| **Climb** | A step onto a neighbouring tile is allowed if `Δh ≤ max_climb` (default 1). Anything taller is a cliff that blocks the move. |
| **Drop** | Dropping down any height is allowed, but a drop greater than 1 **ends a sliding move** at that tile. |
| **Sliders** (R/B/Q) | Rays check each step against climb/drop. An upward step ≥1 also ends the ray ("hard going uphill"). This makes hills defensive. |
| **Knight** | Jumps ignore terrain in between but can only land if `|Δh| ≤ 2`. It cannot land in deep water. |
| **Pawn** | Needs `Δh ≤ 1` to advance. It can't make the double step if either tile is higher. It captures diagonally with the same climb rule. Promotion happens on the far rank. |
| **King** | `Δh ≤ 1` and never enters deep water. The king is attacked only through terrain-legal attack rays, so terrain can block a check. |
| **Shallow water** | Can be entered, but it ends any slide. A piece standing in it can't move 2+ squares next turn. |
| **Deep water** | Impassable unless an item or spell allows it (bridge, freeze, boat). |
| **Cave entrance** | A piece standing on an entrance may use its move to go to the linked entrance, if that tile is empty or holds an enemy it can capture. It is one tunnel hop regardless of piece type. |
| **High ground** | Captures only go level or downhill: a piece can't capture an enemy standing on a higher tile (this includes en passant, cave hops and check). It may still climb onto a higher tile that is empty. |

Castling needs flat, clear squares between king and rook. En passant stays as normal.

Check, checkmate and stalemate reuse the same terrain-aware move generator, so there is no separate attack logic to keep in sync.

**Tests:** perft on a flat 8×8 board must match standard chess numbers (a strong correctness check), plus hand-made terrain fixtures for every rule above.

## 5. Board sizes and armies

| Size | Army per side |
|---|---|
| 8×8 | Standard 16 pieces |
| 16×16 | 2 ranks: 16 pawns plus a doubled back rank (e.g. 4R 4N 4B 2Q 1K + extras). Pawn double step is allowed until the pawn passes its 4th rank |
| 32×32 | Choose between: (a) bigger armies, or (b) "skirmish": a standard 16-piece army spawned in a home zone, with the extra space used for terrain, pickups and caves. **I recommend (b).** A huge army is slow to play and hard for the AI |

The board size can be set in the menu for a new run. Inside a run, the size can also grow in later matches (8 → 16 → 32) as progression.

## 6. Roguelike layer

### Run structure

- A run is a series of **matches** (e.g. 7 "floors" + 1 boss match).
- Each floor raises the difficulty through AI depth/time, rougher terrain, and **enemy modifiers** (the enemy also gets items).
- Between matches you get a **reward draft**: pick 1 of 3, rolled by rarity.
- **Win** → keep everything and move to the next floor. **Lose** (checkmate, or resign) → the run ends and everything resets. Draws count as a loss, or as a retry at a cost, still to be decided.
- The run is saved after every match, so closing the browser doesn't lose it. The save is deleted when you lose. No "save-scumming": the save updates when a match starts.

### Kinds of rewards (all last until the run ends)

1. **Piece enhancements.** Permanent, per piece type or per individual piece:
   - *Mountaineer Rooks*: `max_climb` +1
   - *Amphibious Knights*: can land in deep water
   - *Sure-footed Pawns*: may step sideways onto higher ground
   - *Tunneler Bishops*: may use caves as part of a slide
   - *Veteran*: one chosen piece survives its first capture (it is knocked back one tile instead)
2. **Relics.** Passive, global:
   - *Cartographer*: see pickup locations before the match starts
   - *Tectonic Pact*: your home rows start 1 block higher
   - *Tide Charm*: the water level is −1 on your half
3. **Spells.** Active, with charges per match or a cooldown. Casting a spell **uses your turn** unless it is marked *Quick*:
   - *Raise / Lower Earth*: change a tile's height by ±1–2
   - *Freeze*: make water walkable for 3 turns
   - *Bridge*: place a walkable plank over a gap
   - *Dig Tunnel*: create a temporary cave link
   - *Shield* (Quick): a piece can't be captured until your next turn
   - *Swap*: switch two of your own pieces
   - *Rewind* (rare): undo the last full move

### In-match pickups

Some tiles hold pickups (chests, glowing orbs). A piece that moves onto one collects it:
- **Match-only** buffs, such as a one-time extra move or a temporary spell charge
- Sometimes a **run-permanent** item, which is rarer and placed in dangerous spots like hilltops, cave ends or the enemy half

This gives players a reason to play into the terrain instead of turtling.

### Implementation

- Items are defined in data files (`assets/items/*.ron`) with an id, rarity, kind, and either `MoveProfile` deltas or a `Modifier` id.
- `RunState { seed, floor, owned_items, spells, piece_upgrades, rng_state }` is serializable.
- Spells become part of the move list (`Action::Move | Action::Cast(spell, target)`), so the AI can use them and undo works the same way.

## 7. AI

- Negamax with alpha-beta, iterative deepening, move ordering (MVV-LVA, killer moves), and a transposition table (Zobrist keys that include height changes from spells).
- **Evaluation:** material, mobility (which matters a lot with terrain), high-ground control, king safety that accounts for cliffs, and nearness to pickups.
- **Difficulty** per floor sets the time budget and depth, plus a "blunder chance" on early floors.
- **Web:** no threads by default, so run the search **time-sliced across frames** (N nodes per frame) or in a Web Worker later. Native uses Bevy's `AsyncComputeTaskPool`.
- The AI also uses spells and gets its own enemy relics as floors go up.

## 8. Rendering: 2D pixel art, top-down 3/4 view (see SPRITES.md)

Target: **modern indie pixel art** like the grassland screenshot reference (bold outlines, vivid colours, textured grass, teal cliffs, foamy water), with a fantasy setting that has a dark edge (ruins, graveyards, magic glows). All art comes from a **single 2048² atlas on a 32×32 grid of 64 px cells** (`atlas.png`, see SPRITES.md). It is generated at 2× and downscaled.

- **Grid:** 32×32 px logical tiles, rendered at integer scale with nearest-neighbour sampling. A faint grid is drawn over the board, as in the reference.
- **3D board, pixel-art look:** every square is a 3D column: its top is the square's ground tile and its sides are slices of the cliff art (grass lip, then stone), raised **0.4 squares** per height level. Materials are unlit and faces are shaded by direction, as if lit from the south-east, so the board keeps its sprite look. A perspective camera (30° field of view, tilted 35°) orbits the board: it can turn, pan and zoom freely.
- **Pieces and props** are upright sprite cards that turn to face the camera. The depth buffer handles occlusion, and move markers are drawn by a second camera on top of everything so they are never hidden. Clicks test piece sprites first, then the terrain columns.
- **Layers:** (1) water, (2) ground autotiles, (3) cliff faces + lips, (4) shadows, (5) board overlays (highlights), (6) y-sorted props and pieces, (7) FX, (8) UI.
- **Autotiling:** 13-tile RPG-style sets (3×3 + inner corners) are composed from quarter-tiles at load time into all 47 neighbour cases. This covers grass/water and grass/sand edges and plateau lips.
- **Water:** 2–4 frame tile animation plus a scrolling foam strip where it meets land or cliffs.
- **Caves:** arched openings in the cliff faces. Linked entrances share a coloured rune glow, and hovering one highlights its partner.
- **Board frame:** a decorative ring of dead trees, dark pines, tombstones, ruins and fog outside the playable grid, so the board sits in a small diorama.
- **Mood:** a light vignette, drifting fog-wisp overlays, and flickering point-light tint around braziers, lava and runes (a simple additive glow sprite; no real lighting needed).
- **Pieces:** large **64×64** fantasy characters, about 1.1–1.9 tiles tall from pawn to king. They are y-sorted and overlap the tile behind them. Each silhouette copies its chess piece: the rook is a walking tower golem, the knight shows a big horse head, the bishop wears a tall split mitre, the queen a spiked coronet, the king a cross crown, and the pawn a round helm. The two factions are a **holy kingdom** (silver/white/gold) and an **undead court** (black iron/crimson/violet). **Upgrade pips** under a piece show its enhancements. Upgraded pieces use the "ascended" aura frame.
- **Animation:** mostly procedural motion on top of a few sprite frames:
  - Move: squash, then a hop along a parabolic arc. Climbs step up level by level.
  - Knight: a big leap.
  - Water: a splash.
  - Cave: warp swirl out, then in.
  - Capture: the attacker plays its attack frames and the victim plays hurt + a poof.
  - Screen shake for Earth spells.
- **UI:** a 9-slice parchment scroll (like the reference speech box) and a dark wood-and-iron panel, for opponent taunts, event text and the reward draft (item cards). There are faction banners and pixel fonts (m6x11 / m5x7, Alagard for titles).
- **UX:** tile overlays for move, capture, selected, last move, blocked-by-cliff, check and spell target. Holding Alt shows height badges (0–3) on every tile. Pieces hidden behind a cliff show a faint silhouette outline.

## 9. Milestones

| # | Milestone | Status | Done when |
|---|---|---|---|
| M0 | Workspace scaffold + native & web build pipeline | Done | An empty Bevy scene runs natively and in the browser via `trunk serve` |
| M1 | `core`: standard chess on a flat 8×8 board | Done | Perft tests pass |
| M2 | Terrain model, generator and terrain move rules | Done | Rule-fixture tests pass, and the validator rejects unfair or sealed boards |
| M3 | 2D board rendering (placeholder coloured tiles + letters for pieces) + height/cliff drawing + camera + picking + playable hotseat | Done | Two humans can play a full game on generated terrain |
| M4 | AI opponent | Done | It plays legal terrain moves within a time budget, also on the web |
| M5 | Roguelike loop: floors, reward draft, items, spells, pickups, save/reset | Done | A complete run can be won or lost, and the save persists and is wiped when you lose |
| M6 | 16×16 / 32×32 armies and scaling, balance pass | Done (60 fps at all sizes incl. AI turns; 8×8 self-play 7–3–10) | All sizes can be played at a steady 60 fps on the web |
| M7 | Art & juice: generate the atlas from SPRITES.md, run the processing script, add autotiling, VFX, audio, menus | Done (3D pixel-art board and island, environment art, water foam, VFX, synthesized SFX, title menu) | It matches the look of the pixel-art reference image |
| M8 | Web deploy (static hosting) + size optimisation (`wasm-opt`, `opt-level="z"` for wasm) | Done (Vercel; 6.2 MB brotli, starts in 1.3 s, 3.4 s at 20 Mbps) | A public URL loads in under 10 s |

## 10. Risks

- **Balance.** Terrain plus items can break chess (unstoppable openings, kings you can't mate). Mitigate with symmetric generation, the validator, a draw-by-move-limit rule, and a quick self-play harness (AI vs AI over 1000 seeds) to spot broken items.
- **AI speed on 32×32 on the web.** Use time slicing, a lower depth, and the skirmish army style.
- **Readability.** Cliffs can hide pieces behind them. Mitigate with the limit of 3 height levels, silhouette outlines, and the Alt height badges.
- **Generated art consistency.** Image models drift between sheets and misalign grids. Keeping everything on one sheet keeps the style consistent. Mitigate grid drift with the palette-snapping script and a blob-detecting slicer for sprite regions, allow per-cell regeneration/inpainting for bad cells, and use placeholder art until M7.
- **Bevy version churn.** Pin the version and keep `core`/`ai`/`run` free of Bevy so upgrades only touch `game`.

## 11. Open decisions

1. **Opponent:** single-player vs AI only (fits the roguelike), or also local hotseat? → *Recommend AI for runs, with hotseat as a sandbox mode.*
2. **32×32 army style:** big army or skirmish? → *Recommend skirmish.*
3. **Board size in a run:** fixed at run start, or growing per floor? → *Recommend chosen at start, with a "Grand Run" option that grows.*
4. **Casting a spell uses the turn?** → *Recommend yes, except for Quick spells.*
5. **Any meta-progression across runs** (unlocking new items to the pool)? You said a loss resets everything, so the default is **none**.
