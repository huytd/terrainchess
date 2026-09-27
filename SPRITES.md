# Terrain Chess — Sprite Sheet Spec

**Style:** modern indie pixel art, matching the grassland screenshot reference (bold clean outlines, vivid readable colours, soft cel shading, chunky shapes). The setting is fantasy with a dark edge: a holy human kingdom fights an undead court across meadows, cliffs, ruins and graveyards.

**One sheet:** `atlas.png`, 2048 × 2048 px, on a **32 × 32 grid**.

- §4 is the **prompt**. Paste it to the image model **and attach the reference screenshot as a style reference**.
- §3 is the **cell map** the game slices by.

---

## 0. Technical contract

| Item | Value |
|---|---|
| Canvas | **2048 × 2048 px** = **32 × 32 grid** of **64 × 64 px cells** |
| Scale | **2×**: 1 logical pixel = one 2×2 block. So 1 cell = **32 × 32 logical px** = one board tile |
| Board tile | 1 cell (32 × 32 logical) |
| **Chess piece frame** | **2 × 2 cells = 64 × 64 logical px** (4× the area of the previous spec) |
| Piece height | Varies by rank so pieces are easy to tell apart: Pawn ≈ 36 · Rook ≈ 46 · Bishop ≈ 50 · Knight ≈ 50 (widest, ≈ 56 wide) · Queen ≈ 54 · King ≈ 60 logical px |
| Piece anchor | Feet at the bottom-centre of the 2×2 frame. In game the feet sit on the tile centre, and the sprite rises about one tile above it (y-sorted) |
| Background | Flat **magenta `#FF00FF`**. Unused cells stay magenta |
| View | Top-down **3/4 view** like the reference: floors from above, cliffs and characters show their front |
| Light | From the **top-left**. Glows (magic, fire, runes) are the only other light sources |
| Outline | 1 logical px, dark navy `#1A1C2C` (never pure black) |
| Rendering | Crisp pixels, no anti-aliasing, no blur, no dithering noise, no retro CRT effects |
| Seamless tiles | Ground and water fill the whole cell with no border outline |
| Facing | Characters face **right** in a 3/4 front view. The game mirrors them |

**Cell numbering in this document is 1-based: (row, col).** The code subtracts 1.

**Post-processing** (`tools/process_sprites.py`):
1. Chroma-key the magenta.
2. Downscale 2× with nearest-neighbour and snap to the §1 palette.
3. Slice the **tile regions** strictly by grid.
4. For **sprite regions**, detect each sprite as a connected blob and snap it to its nearest cell or frame. This way small misalignments from the image model don't break the slicing.
5. Write a RON manifest listing every sprite.

---

## 1. Palette (vivid, based on the reference)

| Role | Hex (dark → light) |
|---|---|
| Outline | `#1A1C2C` |
| Grass | `#2E6B3A` `#3F8F45` `#6BBF4E` `#A6DB6A` |
| Moss / dark meadow | `#24503A` `#33704A` |
| Pine | `#1F4A45` `#2F6B57` `#4FA36B` `#86D07A` |
| Cliff stone (teal-grey) | `#2E4A55` `#44707A` `#6C9DA3` `#A5CCD0` |
| Water | `#2D6F7A` `#3F9BA0` `#7FD0CC` · foam `#E8FFF8` |
| Bog | `#3B5A3A` `#52774A` `#7A9A5A` |
| Lava | `#7A1E1E` `#D8431F` `#FF8A2A` `#FFD35A` |
| Soil / path | `#4A2F2A` `#6E4A3A` `#93684E` |
| Cobble / ruins | `#4E5563` `#6E7686` `#9AA3B3` |
| Wood | `#4A2E22` `#6E4633` `#96633F` |
| Bone | `#8F8570` `#C9BFA3` `#EFE6CE` |
| Skin | `#C98E6E` `#F2C29B` |
| **Ashen Sun** (holy kingdom) | silver `#7C8699` `#B7C0D0` `#EEF2F8` · white cloth `#E9E4D6` · gold `#8A5A1E` `#D19A2E` `#FFD35A` · sky-blue accent `#3F7FD1` `#79B4F2` · holy glow `#FFF1B8` |
| **Hollow Crown** (undead court) | black iron `#1F2130` `#343A52` `#565E7A` · crimson `#5E1225` `#9E2235` `#D8454E` · ghostfire violet `#5B2A86` `#9A55D6` `#D9A6FF` · bone (above) |
| Soul teal (neutral magic) | `#1E7A70` `#2BB5A6` `#7FF3E0` |
| Parchment (UI) | `#C8B384` `#E9DAB2` `#F7EDD0` · ink `#3B2F2A` |

---

## 2. Making pieces readable at a glance

Every piece is a fantasy character whose **silhouette copies its chess piece**. Both factions use the **same silhouette per type**.

| Piece | Chess-shape cue built into the character | Ashen Sun | Hollow Crown |
|---|---|---|---|
| **Pawn** | Small; round ball-shaped helmet like a pawn's head | Young squire: round helm, small sun buckler, short spear | Skeleton soldier: round rusted helm, cracked buckler, spear |
| **Rook** | **Is a walking castle tower**: cylindrical body with a crenellated top | White-stone tower golem with gold arrow-slit eyes and stone fists | Obsidian-and-bone tower golem with violet arrow-slit eyes |
| **Knight** | **Horse head in profile** dominates the silhouette | Paladin on a white barded horse, lance with a blue pennant | Death knight on a skeletal horse with a violet flame mane |
| **Bishop** | **Tall split mitre** (the bishop's notched top) | Sun-priest in a tall gold-trimmed mitre, sun-disc staff | Necromancer in a tall split horned hood, skull staff with violet fire |
| **Queen** | **Spiked coronet** (a ring of points) and a flowing gown/cape flaring at the base | Radiant queen-mage with a gold spiked coronet, glowing spear | Banshee witch-queen with a thorn coronet, floating tattered gown |
| **King** | Tallest; **cross-topped crown** | Bearded king, sun-cross crown, ermine robe, greatsword planted | Lich king, horned crown with a cross spike, crimson robe, glowing eyes |

---

## 3. Cell map (rows × cols, 1-based, 64 px cells)

```
          cols 1 ─────────────── 16 │ 17 ─────────────── 32
rows 1–6   A. TERRAIN               │ B. CLIFFS (1–4)
                                    │ C. TALL PROPS (5–6)
rows 7–8   D. SMALL PROPS & PICKUPS (1–16, and row 7 17–32) │ ICONS (row 8, 17–32)
rows 9–20  E. ASHEN SUN PIECES      │ F. HOLLOW CROWN PIECES
rows 21–26 G. BIG PROPS             │ H. PORTRAITS & CRESTS
rows 27–32 I. EFFECTS               │ J. UI
```

### A. Terrain: rows 1–6, cols 1–16 (seamless 1-cell tiles)

| Cells | Content |
|---|---|
| r1 c1–4 | Grass ×4 variants (textured blades and tufts, as in the reference) |
| r1 c5–6 | Grass with tiny flowers ×2 |
| r1 c7–8 | Dark meadow moss ×2 |
| r1 c9–10 | Soil path ×2 |
| r1 c11–12 | Cobblestone ×2 |
| r1 c13–14 | Cracked ruin flagstone ×2 |
| r1 c15–16 | Grave dirt with bones ×2 |
| r2–4 c1–3 | Autotile **grass → water** 3×3 (scalloped grass edge with a dark outline and foam, as in the reference) |
| r2–3 c4–5 | Its 4 inner corners |
| r4 c4–5 | Decals: cracks · rune circle |
| r2–4 c6–8 | Autotile **grass → soil path** 3×3 |
| r2–3 c9–10 | Its 4 inner corners |
| r4 c9–10 | Decals: pebbles · leaf litter |
| r2–4 c11–13 | Autotile **grass → bog** 3×3 |
| r2–3 c14–15 | Its 4 inner corners |
| r4 c14–15 | Decals: grave mound · scorch mark |
| r2–4 c16 | Ash ×1 · bone dirt ×2 |
| r5 c1–4 | Shallow water ×4 animation frames |
| r5 c5–8 | Deep water ×4 frames |
| r5 c9–12 | Bog ×4 frames |
| r5 c13–16 | Lava ×4 frames |
| r6 c1–4 | Shoreline foam strip (top half of cell) ×4 frames |
| r6 c5–8 | Cliff-base foam ×4 frames |
| r6 c9–12 | Frozen water (Freeze spell) ×2 · cracked ice ×2 |
| r6 c13–16 | Fog wisps ×4 |

### B. Cliffs & structures: rows 1–4, cols 17–32

The cliffs are teal-grey rounded stone blocks like the top wall of the reference, with grass hanging over the top. **1 height level = a half-cell face (16 logical px)**. 2 levels = a full cell. The game stacks them for level 3.

| Cells | Content |
|---|---|
| r1 c17–23 | Half-height cliff strip (top half of the cell): left end · middle ×4 · right end · pillar |
| r1 c24–30 | Full-height cliff wall: left end · middle ×4 · right end · pillar |
| r1 c31–32 | Stone stairs up 1 level ×2 |
| r2 c17–20 | Plateau grass-lip overlays: N · S · E · W |
| r2 c21–24 | Lip outer corners: NW · NE · SW · SE |
| r2 c25–28 | Lip inner corners: NW · NE · SW · SE |
| r2 c29–30 | Side rock slivers: east-facing drop · west-facing drop |
| r2 c31–32 | Cliff shadow overlays ×2 |
| r3 c17–18 | Cave mouth in a full cliff ×2 |
| r3 c19–22 | Rune-arch cave door in 4 glow colours (teal · violet · amber · crimson). Linked cave pairs share a colour |
| r3 c23–26 | Cave-exit rune floor in the same 4 colours |
| r3 c27–30 | Wooden bridge: horizontal · vertical · west end · east end |
| r3 c31–32 | Raised-earth pillar (spell) · sunken pit (spell) |
| r4 c17–32 | Border wall: seamless top-edge cliff wall like the reference ×4 · bottom-edge ×4 · left ×4 · right ×4 |

### C. Tall props (1 cell wide × 2 tall): rows 5–6, cols 17–32

Pine ×4 · dead twisted tree ×2 · birch ×1 · broken stone pillar · rune obelisk (teal glow) · angel statue · war banner (Ashen Sun) · war banner (Hollow Crown) · lamp post with a lantern · gallows-style hanging cage · crystal spire · rock spire

### D. Small props & pickups (1 cell each): row 7 all cols, row 8 cols 1–16

| Cells | Content |
|---|---|
| r7 c1–16 | Boulder L ×2 · boulder M ×2 · pebbles · stump ×2 · round bush ×2 · berry bush · grass tuft ×2 · flower patch · mushroom ring · glowing mushrooms · fallen log |
| r7 c17–32 | Tombstone ×3 · skull pile · bone pile · candles · broken cart wheel · crow · spiked barricade · crate · barrel · well · campfire ×2 frames · brazier ×2 frames |
| r8 c1–16 | Chest closed · chest open · key · soul orb (teal) · ember orb (orange) · holy orb (gold) · curse orb (violet) · rune stone · altar · potion ×3 (red/blue/green) · gold coin pile · scroll · gem ×2 |

### Icons (1 cell each): row 8, cols 17–32

| Cells | Content |
|---|---|
| r8 c17–24 | **Upgrades / relics**: climbing spikes · swim fins · hobnail boots · pickaxe · veteran's medal · treasure map · tectonic heartstone · shell amulet |
| r8 c25–32 | **Spells** (spellbook with a glowing sigil): raise earth · sink earth · freeze · bridge · tunnel · ward · swap · rewind |

### E & F. Pieces: rows 9–20 (2×2-cell frames)

**E = Ashen Sun, cols 1–16. F = Hollow Crown, cols 17–32.** Each piece type takes a 2-row band. Each band has **8 frames**, each 2 cols wide:

`idle 1 · idle 2 · move 1 (crouch) · move 2 (airborne) · attack 1 (wind-up) · attack 2 (strike) · hurt · ascended`

The **ascended** frame is the idle pose with a glowing aura outline: gold for the Ashen Sun, violet for the Hollow Crown. It is used for upgraded pieces.

| Rows | Piece |
|---|---|
| 9–10 | Pawn |
| 11–12 | Rook |
| 13–14 | Knight |
| 15–16 | Bishop |
| 17–18 | Queen |
| 19–20 | King |

### G. Big props: rows 21–26, cols 1–16

| Cells | Content |
|---|---|
| r21–23 c1–8 | Large layered pine trees, 2 wide × 3 tall, ×4 (like the reference's border pines) |
| r21–23 c9–12 | Giant dead trees 2×3 ×2 |
| r21–23 c13–14 | Ruined watchtower 2×3 |
| r21–23 c15–16 | Great statue 2×3 |
| r24–25 c1–16 | 2×2 props ×8: hedge silhouette ×2 (dark translucent green blob, as in the reference) · boulder cluster ×2 · ruined wall · crypt entrance · ruined gate · shrine |
| r26 | empty |

### H. Portraits & crests: rows 21–26, cols 17–32

| Cells | Content |
|---|---|
| r21–22 c17–28 | Ashen Sun portraits (head and shoulders), 2×2 each: pawn · rook · knight · bishop · queen · king |
| r23–24 c17–28 | Hollow Crown portraits, same order |
| r21–24 c29–32 | Faction crests 2×2: sun crest (top) · hollow crown crest (bottom) |
| r25–26 c17–32 | empty |

### I. Effects: rows 27–32, cols 1–16

| Cells | Content |
|---|---|
| r27 c1–8 | Tile overlays: move dot · capture brackets (red) · selected brackets (gold) · last move tint · check ring (pulsing red) · spell target rune · hover outline · blocked X |
| r27 c9–16 | Selection ring under feet ×2 frames · upgrade pip filled · empty · damage flash · 3 empty |
| r28 c1–6 | Capture smoke-and-soul burst ×6 frames |
| r28 c7–12 | Water splash ×6 frames |
| r28 c13–16 | Landing dust ×4 frames |
| r29 c1–6 | Cave warp vortex ×6 frames |
| r29 c7–12 | Pickup sparkle ×6 frames |
| r29 c13–16 | Ward bubble ×4 frames |
| r30 c1–4 | Earth burst ×4 frames |
| r30 c5–8 | Frost spread ×4 frames |
| r30 c9–12 | Holy smite ×4 frames |
| r30 c13–16 | Curse bolt ×4 frames |
| r31–32 c1–8 | 2×2 FX: king-fall / crown-shatter ×4 frames |
| r31–32 c9–16 | 2×2 FX: ascension beam ×4 frames |

### J. UI: rows 27–32, cols 17–32

| Cells | Content |
|---|---|
| r27–29 c17–19 | Parchment scroll panel 9-slice (like the reference speech box, with rolled ends) |
| r27–29 c20–22 | Dark wood-and-iron panel 9-slice |
| r27–29 c23–30 | Item card frames 2×3 ×4 rarities: common (stone) · rare (blue) · epic (violet) · legendary (gold, glowing) |
| r27–29 c31–32 | Tall pennant banners 1×3: Ashen Sun · Hollow Crown |
| r30 c17–22 | Buttons 2×1: normal · hover · pressed |
| r30 c23–32 | Name ribbon 2×1 · health-style bar frame 2×1 · bar fills (gold, violet) · turn hourglass · cursor ×2 · 1 empty |
| r31–32 c17–32 | UI icons ×32: sword · shield · crown · boot · mountain · wave · cave · eye · hourglass · heart · skull · star · scroll · potion · lightning · snowflake · rock · bridge · swap · rewind · chest · key · gem · gear · sound · music · pause · play · undo · flag · info · close |

---

## 4. The prompt (paste this, and attach the reference screenshot)

```
One large sprite sheet / texture atlas in MODERN INDIE PIXEL ART, matching the style of the
attached reference screenshot: bold clean dark-navy outlines (#1A1C2C), vivid readable
colours, chunky stylised shapes, soft 3-4 step cel shading with a light top-left highlight,
lush textured grass with small blade tufts, teal-grey rounded stone cliffs, teal water
with white foam, layered green pine trees. Fantasy setting with a dark edge: a radiant holy
kingdom (silver, white, gold, sky blue) versus an undead court (black iron, crimson, bone,
glowing violet ghostfire), fighting over meadows, cliffs, ruins and graveyards.
NOT retro, NOT SNES: crisp modern pixel art, no dithering noise, no blur, no anti-aliasing,
no gradients, no text, no grid lines, no watermark.

FORMAT: canvas exactly 2048x2048 px, an invisible 32x32 grid of 64x64 px cells.
Each logical pixel is a 2x2 block, so one cell = 32x32 logical px = one map tile.
Background is flat magenta #FF00FF; unused cells stay magenta. Sprites never cross into
neighbouring cells except multi-cell sprites, which fill exactly their block. Ground and
water tiles fill their cell edge-to-edge and tile seamlessly. Characters and props stand
at the bottom-centre of their block on a small soft oval shadow. Characters face right in a
3/4 front view.

LAYOUT (rows and columns counted from 1 at the top-left):

TOP-LEFT, rows 1-6, columns 1-16 - terrain tiles:
row 1: 4 grass, 2 grass with tiny flowers, 2 dark moss, 2 soil path, 2 cobblestone, 2 cracked
ruin flagstone, 2 grave dirt with small bones.
rows 2-4: three 3x3 autotiles with inner corners beside each: grass island edge over water
(scalloped outlined grass edge with white foam, like the reference) in cols 1-3 with inner
corners in cols 4-5; grass edge over soil path in cols 6-8 with inner corners in cols 9-10;
grass edge over green bog in cols 11-13 with inner corners in cols 14-15; small ground decals
(cracks, rune circle, pebbles, leaves, grave mound, scorch) under the inner corners; col 16
ash and bone dirt.
row 5: 4 animation frames each of shallow water, deep water, green bog, glowing lava.
row 6: 4 frames of shoreline foam strip, 4 frames of foam at a cliff base, 2 frozen water
tiles and 2 cracked ice tiles, 4 soft fog wisps.

TOP-RIGHT, rows 1-4, columns 17-32 - cliffs:
row 1: teal-grey rounded stone cliff faces with grass hanging over the top: 7 half-cell-tall
strips (left end, 4 middles, right end, pillar), 7 full-cell-tall walls (same set),
2 small stone staircases.
row 2: grass lip overlays for a raised plateau (N, S, E, W edges, 4 outer corners,
4 inner corners), 2 side rock slivers, 2 soft shadow overlays.
row 3: 2 dark cave mouths in a cliff, 4 stone cave arches glowing with runes in teal, violet,
amber and crimson, 4 matching glowing rune floor circles, 4 wooden plank bridge pieces,
a rock pillar bursting up, a sunken pit.
row 4: 16 seamless border-wall pieces like the top wall of the reference.

rows 5-6, columns 17-32 - 16 tall props, each 1 cell wide and 2 cells tall: 4 pines, 2 dead
twisted trees, a birch, a broken pillar, a glowing rune obelisk, an angel statue, a white-and-
gold sun banner, a crimson-and-black crown banner, a lantern post, a hanging iron cage, a
crystal spire, a rock spire.

rows 7-8 - small 1-cell props and items: boulders, pebbles, stumps, round bushes, berry bush,
grass tufts, flowers, mushroom ring, glowing mushrooms, log, tombstones, skull pile, bones,
candles, cart wheel, crow, spiked barricade, crate, barrel, well, campfire (2 frames),
brazier (2 frames), chests closed and open, key, four glowing orbs (teal, orange, gold,
violet), rune stone, altar, three potions, coin pile, scroll, two gems. Then in row 8 columns
17-24 eight upgrade icons (climbing spikes, swim fins, hobnail boots, pickaxe, medal,
treasure map, glowing heartstone, shell amulet) and columns 25-32 eight spellbook icons
with glowing sigils (rising rock, sinking pit, snowflake, bridge, tunnel, shield, swap
arrows, rewind hourglass).

rows 9-20 - THE CHESS PIECES, LARGE: each sprite fills a 2x2-cell block (64x64 logical px).
Columns 1-16 are the HOLY KINGDOM (polished silver armour, white cloth, gold trim, sky-blue
accents, sun emblems). Columns 17-32 are the UNDEAD COURT (black iron, crimson cloth, bone,
violet ghostfire, glowing eyes).
Each piece type is a 2-row band with 8 frames per faction: idle, idle (1 px lower), crouch,
leap, wind-up, strike, hurt recoil, and "ascended" (idle wrapped in a glowing aura outline -
gold for the kingdom, violet for the undead).
Every character's silhouette must copy its chess piece so it is recognisable instantly:
- rows 9-10 PAWN (small, about 36 px tall, round ball-shaped helmet): young squire with
  round helm, sun buckler and short spear / skeleton soldier with round rusted helm and spear.
- rows 11-12 ROOK (a walking castle tower: cylindrical body, crenellated battlement top,
  stone fists): white stone tower golem with glowing gold arrow-slit eyes / obsidian and bone
  tower golem with glowing violet arrow-slit eyes.
- rows 13-14 KNIGHT (a big horse head in profile dominates): paladin on a white barded horse
  with a lance and blue pennant / death knight on a skeletal horse with a violet flame mane.
- rows 15-16 BISHOP (tall split mitre with a notch): sun-priest in a tall gold-trimmed mitre
  with a sun-disc staff / necromancer in a tall split horned hood with a violet-flame skull
  staff.
- rows 17-18 QUEEN (spiked coronet, gown or cape flaring wide at the base): radiant queen-mage
  with gold spiked coronet and glowing spear / banshee witch-queen with thorn coronet and a
  floating tattered gown.
- rows 19-20 KING (tallest, about 60 px, cross-topped crown): bearded king with sun-cross
  crown, ermine robe and planted greatsword / lich king with horned cross crown, crimson robe
  and glowing eyes.

BOTTOM-LEFT, rows 21-26, columns 1-16 - big props: four large layered pine trees 2 wide x 3
tall like the reference, two giant dead trees 2x3, a ruined watchtower 2x3, a great statue
2x3; below them eight 2x2 props: two dark translucent hedge silhouettes like the reference,
two boulder clusters, a ruined wall, a crypt entrance, a ruined gate, a small shrine.

rows 21-26, columns 17-32 - 2x2 head-and-shoulders portraits of the six kingdom pieces
(row 21-22) and six undead pieces (rows 23-24) in the same order, plus two 2x2 faction
crests (a golden sun, a hollow black-and-crimson crown) at columns 29-32.

BOTTOM-LEFT, rows 27-32, columns 1-16 - effects: row 27 board overlays (white move dot, red
corner brackets, gold corner brackets, pale tint, red check ring, glowing rune circle, white
hover outline, grey X) then a gold selection ring (2 frames) and upgrade pips; rows 28-30
short animations: smoke-and-soul capture burst (6), water splash (6), landing dust (4),
teal warp vortex (6), gold pickup sparkle (6), cyan ward bubble (4), earth burst (4), frost
spread (4), holy golden smite (4), violet curse bolt (4); rows 31-32 two 2x2 animations: a
crown shattering (4 frames), a rising ascension beam of light (4 frames).

BOTTOM-RIGHT, rows 27-32, columns 17-32 - interface: a parchment scroll panel with rolled
ends like the reference speech box as a 3x3 nine-slice; a dark wood-and-iron panel as a 3x3
nine-slice; four 2x3 item card frames (stone, blue, violet, glowing gold); two tall pennant
banners (sun kingdom, undead crown); three 2x1 buttons (normal, hover, pressed); a name
ribbon; a bar frame with gold and violet fills; an hourglass; two cursors; and 32 small
interface icons (sword, shield, crown, boot, mountain, wave, cave, eye, hourglass, heart,
skull, star, scroll, potion, lightning, snowflake, rock, bridge, swap, rewind, chest, key,
gem, gear, speaker, music note, pause, play, undo, flag, info, close).

Everything shares one consistent scale, outline weight, lighting and palette.
```

---

## 5. Tips for generating

- **Attach the reference screenshot** every time. It carries the style better than words.
- A 32×32 grid is a lot for an image model to hold. If the layout drifts, the blob-detecting slicer usually recovers it. For areas that come out badly, **inpaint just that region** (e.g. "rows 9–20: redo the pieces") and keep the rest of the sheet.
- The **pieces block (rows 9–20)** matters most. If it's weak, regenerate that block on its own at the same size and paste it back into the sheet. It is still one sheet at the end.

## 6. Fonts (don't generate)

Use free pixel fonts, e.g. **"m6x11"** / **"m5x7"** by Daniel Linssen (like the reference's labels) and **"Alagard"** for titles. Check the licences.

## 7. Acceptance checklist

- [ ] Fill each piece solid black: all 6 types can still be told apart, on both factions
- [ ] At 1× zoom the two factions are easy to tell apart (bright silver/gold vs dark iron/violet)
- [ ] Ground, water and cliff-middle tiles repeat without seams
- [ ] After downscaling 2× and palette-snapping, the edges are crisp and nothing looks muddy
- [ ] Multi-cell sprites line up with their blocks (the slicer tolerates small drift)
