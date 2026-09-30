# Terrain Chess — notes for Claude

Pixel-art chess on 3D terrain (hills, cliffs, water, caves change how pieces move), with spell
cards and a 10-level ladder. Rust workspace, Bevy 0.19, shipped as WASM.
Live: https://terrainchess.vercel.app — every push to `main` deploys (GitHub Actions →
Vercel, `.github/workflows/deploy.yml`). Specs in `specs/` (game-design.md is the rules source).

## Workspace

| Crate | Path | Role |
|---|---|---|
| `tc_core` | `crates/core` | Rules, no Bevy. `Match` (rules.rs) = terrain + `Position` + hands + timed effects; `play`, `cast`, `cast_targets`, `cast_block`/`target_block` (why a card/square is refused), `outcome` (repetition hashes position **and** terrain). `spell.rs`: `SpellId` (16 spells, `ALL`, `is_quick`, `effect_text`, `no_target_hint`, `target_hint`), `SpellCast`. `position.rs`: `start(size)` (4,5,6,7,8,12,16), `from_army`. `worldgen/`: terrain generator. |
| `tc_ai` | `crates/ai` | Alpha-beta search + `choose_action` (move or spell; casts must beat the move by +60). `Limits::for_floor(n)`. |
| `tc_run` | `crates/run` | `level.rs`: `LEVELS` (10 levels: size, `army`, optional `enemy_army`, difficulty). `profile.rs`: `Profile` (owned items, cleared levels, `match_setup(level, seed)`, `record_win` → draft, `pick`, chosen `deck` + `swap_with_reserve`; always shuffled in `match_setup`). `deck.rs`: `card_pool`, `reconcile`, `reserve`, `shuffle`. `run.rs`: `RunState`/`MatchSetup` (terrain, armies, decks, AI level). `item.rs` + `assets/items/*.ron`. |
| `terrainchess` | `crates/game` | The Bevy app (below). |

### Game crate (`crates/game/src`)

- `main.rs` — plugins; font: m6x11plus is the only (default) font. It sits on an 18 px em, so
  use sizes in multiples of 9 (`theme::XS/S/M/L/XL` = 18/27/36/54/72). It has ASCII, Latin-1 and
  `×`, but no `·` `—` `–` `…`; `crates/run/tests/ui_strings.rs` checks rules/item strings.
- `theme.rs` — Balatro-style UI kit, all drawn in code: palette, `label`/`label_nowrap`/`ink`,
  `panel(PanelKind)`, `scrim`, `title_bar`, `number_chip`, `tag_chip`, `button(tone, w, h)` +
  `update_button_visuals` (`Tone`, `ButtonDisabled`), `card_root` + `spawn_card_face`,
  `spell_accent`/`rarity_color`. Bundles must not repeat components (Bevy panics at spawn):
  add extra components with `.insert(...)`.
- `ui_fx.rs` — `CardMotion` (sway, hover lift/scale, cursor tilt, `extra` offset) is the only
  writer of a card's `UiTransform`; interaction handlers set `motion.active`. `PopIn` scales
  panels/cards in.
- `game.rs` — `GameState` resource: the `Match`, selection, armed spell, `undo`, `events:
  Vec<GameEvent>`, dirty flags (`terrain_dirty`, `pieces_dirty`), `toast: Option<String>`
  (HUD shows and clears it), `announce` (held-back AI cast), `last_enemy_spell`,
  `force_ai_cast` (dev). `cast()`/`play()` push events.
- `run.rs` — `Run { profile, level, phase: Playing | Draft(items) | Result{won}, pending_draft }`.
  Win → Result screen with "Claim reward" → Draft → pick → back to level select (title menu).
  `TitleMenu { open, prepare, held }`: clicking a level opens its Prepare (deck) screen;
  `DeckEdit` messages (tap/reset; taps only swap deck ↔ reserve) go through `apply_deck_edit`. Retry skips Prepare.
- `ai.rs` — runs `tc_ai` in time slices; waits while a banner is up (`state.announce`).
- `input.rs` — camera orbit/zoom, click/tap → `tap_board`, hotkeys → `Action` messages →
  `apply_actions`.
- `hud.rs` — all UI: level badge, title menu/level select, Prepare screen (`spawn_prepare`:
  5×3 deck grid, reserve chips, Back/Reset/Start), hand bar (tooltip above the hovered/armed card; blocked cards dimmed with a
  red !), draft + result overlays, toasts. Big file; grep for the `fn sync_*` you need.
- `announce.rs` — enemy spell banner + "Enemy: <spell>" pill. AI casts are stashed in
  `GameState.announce` (events + dirty flags) and released by `finish_announce()` when the
  banner closes.
- `board_view.rs` — board meshes. `Quads` builds batched textured quads. Terrain look:
  `cushion()` (rounded tops curving `CUSHION` toward dropping sides, plan-rounded corners,
  baked `shine()` highlight), `rim()` (rock walls, curved corner walls, grass lips, corner
  shadow patch). Also used by `scenery.rs`.
- `scenery.rs` — the island around the board (grass terraces, sand shore + surf strips, sea,
  props). Rebuilt only when (seed, size) changes.
- `fx.rs` — one-sprite effects per event. `particles.rs` — CPU particle bursts per spell
  (WebGL2: no compute shaders; shared mesh/material per sprite+tint; cap 400 / 150 on phones).
- `sfx.rs` — sounds; **drains `state.events`** each frame. Anything that reads events must run
  before it (`fx::process_game_events`, `particles::cast_particles`; `announce::run_banner`
  runs before both). Breaking this order silently loses effects.
- `atlas.rs` — loads `assets/atlas.png` + `atlas.ron`; `atlas.rect(name)`, `atlas.uv(name)`.

## Assets

- `assets/atlas.png/.ron` are generated: `python tools/process_sprites.py` (needs numpy +
  pillow; the system python lacks numpy → `python3 -m venv /tmp/venv && /tmp/venv/bin/pip
  install numpy pillow`). Output is deterministic; re-running unchanged gives identical files.
- `tools/gen_terrain_art.py` draws procedural pixel art (`wall_rock`, `grass_lip`) that
  process_sprites.py packs in. Add new generated sprites there.
- Card art is `art_<spell_item_id>` (see `theme::spell_item_id`); icons `icon_*`, fx `fx_*`.
  UI frames are drawn in code; the `gui_*`, `btn_*`, `panel_*`, `banner`, `badge_*` and
  `deck_pile` atlas sprites are no longer used.

## Checks (run all before pushing)

```sh
cargo fmt --all -- --check
cargo clippy --workspace --exclude terrainchess --all-targets -- -D warnings
cargo clippy -p terrainchess --target wasm32-unknown-unknown -- -D warnings
cargo test --workspace --exclude terrainchess
cd web && env -u NO_COLOR trunk build --release     # output in web/dist
```

The native game crate can't build on this machine (no ALSA/udev); always lint it with the
wasm target. Clippy's `too_many_arguments` is common here — `#[allow]` it on UI/mesh helpers.

## Browser testing (Playwright)

Serve the build: `cd web/dist && setsid python3 -m http.server 8765 &` (don't `pkill -f`
with a pattern matching your own shell). Scripts use `playwright-core` with
`executablePath: '/usr/bin/google-chrome'`, args `--use-angle=swiftshader
--enable-unsafe-swiftshader`, URL `http://localhost:8765/?dev`, wait ~13 s for load (phone
viewport 390×844: ~30 s). Count console messages matching `/ERROR|panic/`.

- Level select at 1280×800: left column x=503, right x=775, rows y=331/405/479/553/627
  (levels 1–5 left, 6–10 right). Phone: left column x≈105, rows y≈358, step 66.
- A level button opens its Prepare (deck) screen; Enter starts. 1280×800 with no reserve:
  deck tiles x=400/520/640/760/880. With a full reserve: tile rows y=215/320/425, reserve chips
  from y=589, Start (882, 720).
- Gear/menu button (1242, 38). Mouse wheel zooms the camera.
- Swiftshader is slow (~0.5 s per screenshot); use F4 slow motion to catch short effects.

Dev keys (only with `?dev`): **F4** slow motion 0.2×, **F6** AI casts the next spell in
`SpellId::ALL` on its next turn (fills the AI's hand), **F7** random spells in your hand,
**F8** win, **F9** lose, **f** swap sides (AI plays the side to move), **U/Backspace** undo.

## Conventions

- Match surrounding style; comments explain *why*, sparingly.
- Commit messages: short summary line + a paragraph on what/why; end with the
  `Co-Authored-By` line from the session's attribution instructions.
- Work directly on `main` unless asked otherwise; pushing deploys, so ask before pushing
  unless the user said to deploy.
- Verify UI changes with a screenshot (desktop and phone) before claiming they work.
