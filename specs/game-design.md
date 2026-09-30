# Terrain Chess — Game Design

For tech stack, code structure, and milestones, see [project-plan.md](project-plan.md).

---

## 1. Terrain model

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

## 2. Terrain movement rules (first draft, tune by playtesting)

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
| **Clear blocker** | A pawn may sacrifice itself to destroy an obstacle on one of its two forward diagonal squares (its attack squares). The pawn is removed and the obstacle is permanently destroyed. |

Castling needs flat, clear squares between king and rook. En passant stays as normal.

Check, checkmate and stalemate reuse the same terrain-aware move generator, so there is no separate attack logic to keep in sync.

**Tests:** perft on a flat 8×8 board must match standard chess numbers (a strong correctness check), plus hand-made terrain fixtures for every rule above.

## 3. Board sizes and armies

| Size | Army per side |
|---|---|
| 8×8 | Standard 16 pieces |
| 16×16 | 2 ranks: 16 pawns plus a doubled back rank (e.g. 4R 4N 4B 2Q 1K + extras). Pawn double step is allowed until the pawn passes its 4th rank |
| 32×32 | Choose between: (a) bigger armies, or (b) "skirmish": a standard 16-piece army spawned in a home zone, with the extra space used for terrain, pickups and caves. **I recommend (b).** A huge army is slow to play and hard for the AI |

The board size can be set in the menu for a new run. Inside a run, the size can also grow in later matches (8 → 16 → 32) as progression.

## 4. Roguelike layer

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

   Each side plays with a 15-card spell deck and a hand of up to 3 cards. Casting a spell removes that card from the hand; once the hand is empty, it draws up to 3 cards from the deck. A player may also discard a chosen card without spending a turn (up to 5 times per match) as long as the deck is not empty, immediately drawing a replacement card into that same slot. Before each level the player picks which 15 of their owned spell cards make the deck (basic filler cards pad it when they own fewer than 15) by swapping cards between the deck and a reserve. The deck is always shuffled at the start of a match; the player can't set the draw order.

### In-match pickups

Some tiles hold pickups (chests, glowing orbs). A piece that moves onto one collects it:
- **Match-only** buffs, such as a one-time extra move or a temporary spell charge
- Sometimes a **run-permanent** item, which is rarer and placed in dangerous spots like hilltops, cave ends or the enemy half

This gives players a reason to play into the terrain instead of turtling.

### Implementation

- Items are defined in data files (`assets/items/*.ron`) with an id, rarity, kind, and either `MoveProfile` deltas or a `Modifier` id.
- `RunState { seed, floor, owned_items, spells, piece_upgrades, rng_state }` is serializable.
- Spells become part of the move list (`Action::Move | Action::Cast(spell, target)`), so the AI can use them and undo works the same way.

## 5. AI

- Negamax with alpha-beta, iterative deepening, move ordering (MVV-LVA, killer moves), and a transposition table (Zobrist keys that include height changes from spells).
- **Evaluation:** material, mobility (which matters a lot with terrain), high-ground control, king safety that accounts for cliffs, and nearness to pickups.
- **Difficulty** per floor sets the time budget and depth, plus a "blunder chance" on early floors.
- **Web:** no threads by default, so run the search **time-sliced across frames** (N nodes per frame) or in a Web Worker later. Native uses Bevy's `AsyncComputeTaskPool`.
- The AI also uses spells and gets its own enemy relics as floors go up.

## 6. Rendering: 2D pixel art, top-down 3/4 view (see assets-sprites.md)

Target: **modern indie pixel art** like the grassland screenshot reference (bold outlines, vivid colours, textured grass, teal cliffs, foamy water), with a fantasy setting that has a dark edge (ruins, graveyards, magic glows). All art comes from a **single 2048² atlas on a 32×32 grid of 64 px cells** (`atlas.png`, see assets-sprites.md). It is generated at 2× and downscaled.

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
