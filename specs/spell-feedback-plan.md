# Spell feedback plan

## Goals

1. Explain the "my Raise/Lower got reverted" reports.
2. Tell the player why a card can't be cast.
3. Announce opponent spells clearly.
4. Later: particle effects on the board for every spell.

## 0. Diagnosis: why a raised or lowered block "reverts"

In the rules, Raise and Lower are **permanent**. `Match::cast` edits `tile.height`, and no timed effect ever touches height. `tick_timed_effects` only restores tile *kind* after Freeze and cave links after Dig Tunnel. The terrain mesh rebuilds from `state.game.terrain` every time `terrain_dirty` is set, so the view isn't lagging behind the rules either.

What actually happens is that the **opponent casts a spell silently**:

- The **most likely cause is Rewind**:
  - The enemy deck is drawn from `ALL_CASTABLE` (`crates/run/src/run.rs:291`), so it can hold Rewind.
  - Rewind is a *quick* spell. The AI casts it and then still moves in the same turn.
  - It restores the snapshot from two plies back, which undoes your Raise/Lower **and** the AI's previous move.
  - It also hands your spent card back, because only the caster's hand is kept (`rules.rs`, the `SpellCast::Rewind` arm).
  - To you it looks like the block popped back, a piece jumped back, and a card reappeared, with no explanation.
- **Less likely: a Lower on the tile you raised, or a Raise on the tile you lowered.** In a quick experiment the AI cast a spell in only 1 of 56 positions right after a player Raise/Lower, and never on the same tile. So this is possible but rare.
- **Undo (Backspace/U)** also reverts, but it is keyboard-only, so it's unlikely to be hit by accident.

**Fix:** the rules are correct, so this is fixed by *visibility*: the announcement in section 2, plus a Rewind-specific effect (§2.4). Optional balance tweak: keep Rewind out of enemy decks on levels 1–4.

**Confirmation test (step 0 of implementation):**
- Add a `tc_run`/`tc_ai` test that plays a level-5 setup with the enemy hand forced to `[Rewind, …]` after a player Raise.
- Assert that the AI picks Rewind and the height reverts. This proves the path end-to-end.

## 1. "Why can't I cast this?" feedback

### 1.1 Core: a reason API (`crates/core/src/rules.rs`, `spell.rs`)

Add:

```rust
pub enum CastBlock {
    GameOver,
    NotYourTurn,                 // AI side to move / AI thinking
    CardUsed,                    // slot already spent this hand
    InCheck,                     // king is in check and this spell can't fix it
    NoTargets(SpellId),          // nothing on the board it can affect
    TooEarly,                    // Rewind: needs 2 plies of history
}
pub enum TargetBlock {
    NotATarget(SpellId),         // e.g. Freeze on a square with no water nearby
    KingSquare,                  // Raise/Lower/etc. can't target a king
    Obstacle,                    // tree/rock on the tile
    Void,
    ExposesKing,                 // the cast would leave your own king in check
}
impl Match {
    pub fn cast_block(&self, spell: SpellId) -> Option<CastBlock>;
    pub fn target_block(&self, spell: SpellId, sq: Sq) -> Option<TargetBlock>;
}
```

- `cast_block` returns `None` when `cast_targets(spell)` is non-empty. Otherwise it picks the most useful reason, in this order:
  1. `GameOver`
  2. `TooEarly` (Rewind)
  3. `InCheck`: in check, and the same spell has raw targets if the self-check filter is dropped.
  4. `NoTargets`
- To support 3 without copying code, refactor `cast_targets` into `raw_cast_targets(spell)` (no check filter) plus a filter. Behaviour stays the same.
- `target_block` classifies one clicked square for a single-square spell. For two-step spells (Swap, Dig Tunnel, Blink) it classifies the first pick only.
- Per-spell `NoTargets` text, as a `SpellId::no_target_hint() -> &'static str` in `spell.rs`:
  - Freeze: "No water next to any square"
  - Bridge/Evaporate: "No water to use it on"
  - Smite: "No trees or rocks to destroy"
  - Curse: "No enemy piece to curse"
  - …one line each, covering all 16.
- Unit tests:
  - Start position: Rewind → `TooEarly`.
  - King in check: Raise on an unrelated square → `ExposesKing` or `InCheck`.
  - Freeze on a dry board → `NoTargets`.
  - Raise on the king's square → `KingSquare`.

### 1.2 Game: toasts (`crates/game/src/hud.rs`, new `toast.rs` if it grows)

- `Toast` resource holding `{ text, ttl }`, plus a `ToastMsg(String)` message and a system that shows one line above the hand bar.
  - Handjet 22 px (18 on phone), wood 9-slice panel, `LineBreak::WordBoundary`, max-width 90 vw.
  - Fades in over 0.15 s, stays 2.2 s, fades out over 0.4 s. A new toast replaces the current one.
- Triggers:
  - `GameState::arm_slot`: if `cast_block` is `Some`, **don't arm**. Send the reason instead, e.g. "Your king is in check — this spell can't save it" or "Rewind needs two turns of history". Add `pub toast: Option<String>` to `GameState`, which the HUD drains, so `game.rs` stays Bevy-UI-free.
  - Clicking a non-target square while a spell is armed: currently this silently disarms or selects. Instead, send the `target_block` reason, e.g. "That would leave your king in check" or "Can't raise the square under a king", and **keep the spell armed**. A click on empty sky still disarms.
  - Clicking a card while the AI is thinking: "Wait for your turn".
  - Discarding with 0 discards left: "No discards left".
  - F7 dev spells: no change.

### 1.3 Card state in the hand bar

- In `sync_hand_bar`, compute `cast_block` for each card.
- A blocked card gets a desaturated tint (image color about 0.55 grey) and a small red corner badge with a padlock icon from `gui`/`atlas`, e.g. `icon_lock` if one exists, otherwise `icon_skull`.
- Hovering or pressing a blocked card shows its reason as a toast. Touch counts as a tap.
- Add `cast_block` to the hand bar's change key so the card state refreshes after every ply.

## 2. Opponent spell announcement

### 2.1 Event plumbing (`game.rs`)

- Add `side: Side` to `GameEvent::Cast` and fill it in `GameState::cast`. Update `fx.rs` and `sfx.rs` to match.
- Add `GameState::last_cast: Option<(Side, SpellId, Vec<Sq>)>` for the log pill in §2.3.

### 2.2 Banner (`hud.rs` or new `announce.rs`)

Triggered for `Cast` events where `side == ai_side`. In hotseat, trigger it for every cast.

Layout (1280×800; the phone size is in brackets):

- A full-width dark strip across the middle third:
  - Black at 80 % alpha, with a thin red rule (enemy) or blue rule (player / hotseat White) on the top and bottom edges.
  - Height 34 % of the viewport, minimum 180 px [phone: 26 %].
- **Left**: the spell card, rendered with the same card composition as the hand bar (frame, art and name).
  - Reuse `spawn_card_visual` if one exists; otherwise extract it from `sync_hand_bar` into a shared function.
  - Scale about 1.3 [phone 0.9].
  - Slides in from the left edge, 0.25 s ease-out-back.
- **Right**, stacked:
  - A small label, "Enemy casts" (Handjet 24).
  - The spell name in **Jacquard 24**, 72 px [phone 44], uppercase, with a 0.2 s scale-punch from 1.4 to 1.0.
  - Its one-line description (Handjet 22, word-wrapped, max width 40 vw).
- Timeline: about 1.6 s total.
  - 0–0.25 s: strip wipes in (height tween).
  - 0.25–1.3 s: hold.
  - 1.3–1.6 s: strip collapses and fades out.
  - Click or tap anywhere to skip.
- SFX: reuse the existing cast sound, plus a low "whoosh" if one exists in `assets/sfx`.
- **The game waits for the banner:**
  - Add `Announcing(bool)` resource; `ai::think` returns early while it is active. This stops a quick AI spell (Shield, Rewind, Featherfall, Insight) and its follow-up move from playing out under the banner.
  - Player input is blocked during the banner, except the click that skips it.
  - Board FX for the cast (dirt, snowflake, …) are deferred until the banner closes, so the player sees the effect *after* reading what it is.
    - Implementation: `fx::process_game_events` queues Cast events from the AI side in a `PendingCastFx` resource and releases them when `Announcing` turns false.
- Target reveal after the banner: for about 1.2 s, pulse a coloured ring or highlight on the cast's squares.
  - Reuse the target-marker quad from `board_view` in red.
  - If a target is off-screen, nudge the camera orbit toward it. Nice-to-have; skip on the first pass.

### 2.3 "Last enemy spell" pill

- A small pill at the top-left, under the level name: card icon plus "Enemy: Rewind". Tapping it shows the description as a toast.
- It stays until the enemy's next action, so a player who looked away can still see what happened.

### 2.4 Rewind-specific clarity

- When Rewind is cast by either side, add a banner subtitle: "The last two turns are undone".
- Every square whose height, tile or piece changed gets a brief "ghost" flash: a white quad that fades over 0.6 s.
- Pieces moved back by the rewind animate (hop) back instead of snapping.
  - Emit `GameEvent::Rewound { moved: Vec<(Sq, Sq)> }`, computed by diffing positions in `GameState::cast` (the square diff already exists there).
- This directly answers the "my block reverted" confusion.

## 3. Particle effects for spells (later phase)

Constraint: the web build is WebGL2, so **no compute-shader particles** (bevy_hanabi needs WebGPU). Instead, extend `fx.rs` with a tiny CPU particle system:

- `Particle { vel, life, max_life, size_curve, color_curve, gravity, spin }`. Each particle is a camera-facing quad that shares one mesh and one material per atlas sprite (the pattern `FxMeshes` already uses).
- `ParticleBurst` recipes, one per spell, with fixed seeds so replays look the same. Cap live particles at about 400, and at about 150 on phones (detected by the same compact breakpoint the HUD uses).
- Atlas sprites to use: the existing fx frames (`smoke`, `dust`, `dirt`, `snowflake`, `bubble`, `sparkle`, …) plus the `icon_*` art. Soft white dots can be tinted.

| Spell | Recipe |
|---|---|
| Raise Earth | Dirt chunks burst up and fall back, a dust ring at the base, a small camera shake (0.08) |
| Lower Earth | Dust sucked inward, then a puff of dust out |
| Freeze | Snowflakes spiral down; frost sparkles on each frozen tile |
| Bridge | Planks (brown sparks) sweep across; a splash |
| Dig Tunnel | Dirt spray at both ends; a dotted arc between them |
| Shield | Bubble dome pulse; blue sparkles orbit for 0.8 s |
| Swap | Two coloured trails along the arc between the squares |
| Rewind | Clock-hand swirl at board centre; ghost flash on changed squares (§2.4) |
| Smite | Lightning pillar (`spawn_pillar`), then sparks and debris |
| Evaporate | Steam rising, white fading to transparent |
| Flood | Water droplets burst out, then a splash ring |
| Featherfall | Feathers drifting down around the piece |
| Curse | Purple smoke with skull icons rising |
| Sprout | Green leaves pop up; small sparkles |
| Blink | Sparkle implosion at the source, explosion at the destination |
| Insight | Star sparkles from the caster's king; card-glow |

Particles fire when the Cast event is processed. For enemy casts that is after the banner closes (§2.2), so the timing works out.

## 4. Implementation order and files

| Step | What | Files | Size |
|---|---|---|---|
| 0 | Rewind revert repro test | `crates/ai/tests/` or `crates/run/src/` | S |
| 1 | `CastBlock`/`TargetBlock` API, hints, `raw_cast_targets` refactor, tests | `core/rules.rs`, `core/spell.rs` | M |
| 2 | Toast UI + triggers in arm/click/discard | `game.rs`, `input.rs`, `hud.rs` | M |
| 3 | Blocked-card dimming + badge + hover reason | `hud.rs` | S |
| 4 | `Cast{side}`, `Announcing`, banner, AI wait, deferred FX, target pulse | `game.rs`, `ai.rs`, `fx.rs`, `hud.rs`/`announce.rs`, `board_view.rs` | L |
| 5 | Last-enemy-spell pill | `hud.rs` | S |
| 6 | Rewind subtitle, ghost flash, hop-back | `game.rs`, `board_view.rs`, `fx.rs` | M |
| 7 (later) | CPU particle system + 16 recipes | `fx.rs` (or `particles.rs`) | L |

- Steps 1–3 are independent of 4–6, so they can run in parallel worktrees if we switch to agy.
- Step 7 is a separate PR.

## 5. Verification

- **Gates:** fmt, both clippy runs, workspace tests, and `trunk build --release`.
- **New dev key F6 (`?dev`):** force the AI to cast a random castable spell from a fixed list on its next turn. This makes the banner testable in Playwright. Add F5 to put the player's king in check, if that's cheap; otherwise use a scripted position.
- **Playwright on desktop (1280×800) and phone (390×844):**
  1. Blocked card: tap it → a toast with the reason appears → screenshot.
  2. Non-target square click → toast; the spell stays armed.
  3. F6 → banner: card, name and description fit the strip with no overflow → screenshot mid-hold.
  4. Banner skip on click.
  5. The AI's move happens only after the banner.
  6. Rewind: the ghost flash plays and pieces hop back.
  7. 0 console errors.
- **Live check after deploy:** bundle 200, 0 errors.
