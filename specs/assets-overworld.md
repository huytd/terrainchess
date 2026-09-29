# Terrain Chess — Overworld Sheet Spec

Art for the top-down 2D overworld map, where heroes move between camps and explore. The board and battle pieces stay on `assets-sprites.md`; the environment backgrounds are on `assets-environment.md`. This is a fourth, separate sheet.

**Style:** the same as `assets-sprites.md`: modern indie pixel art, bold clean outlines, vivid readable colours, soft cel shading, chunky shapes. The scale is smaller than the battle board: a hero token represents a whole army, and a camp building represents a battlefield.

- §3 is the **prompt**. Paste it to the image model **with `assets/spritesheet.jpg` attached as the style reference**.
- §2 is the **cell map** the game slices by.

---

## 0. Technical contract

| Item | Value |
|---|---|
| File | `assets/overworld.png` (PNG preferred; JPG accepted) |
| Canvas | **2048 × 2048 px** = **16 × 16 grid** of **128 × 128 px cells** |
| Scale | **4×**: 1 logical pixel = one 4×4 block. 1 cell = 32 × 32 logical px = one map tile |
| Background | Flat **magenta `#FF00FF`** wherever there is no art; unused cells stay magenta |
| Outline | 1 logical px dark navy `#1A1C2C` on buildings, props, and tokens. No outline on seamless terrain tiles |
| Light | From the **top-left**, matching the main game |
| Rendering | Crisp pixels, no anti-aliasing, no blur, no dithering noise |
| Palette | The §1 palette of `assets-sprites.md` |
| View | Top-down **3/4 view** matching the main board |

**Cell numbering is 1-based: (row, col).**

---

## 1. Sheet layout

```
rows 1-3   A. Seamless terrain tiles (grass, forest, hills, water, roads, crossings)
rows 4-6   B. Camp buildings, map objects, and fog of war
rows 7-8   C. Hero map tokens and faction flags
rows 9-10  D. Overworld UI and move markers
rows 11-16 (empty, magenta)
```

---

## 2. Cell map

### A. Terrain Tiles (rows 1–3, seamless full-cell tiles, no outlines)

| Cells | Content |
|---|---|
| (1,1)–(1,4) | Grass plains (4 variants: flat, tufty, flower/clover, small stones) |
| (1,5)–(1,8) | Dense forest canopy (4 variants: dense canopy, clearing edge, light trees, dense pine/oak) |
| (1,9)–(1,12) | Rugged hills (4 variants: rocky outcrop, grassy knoll, ledge, scree/boulders) |
| (1,13)–(1,16) | Mountains (4 variants: single peak, ridge, double peak, snowcap; impassable) |
| (2,1)–(2,4) | Water/Sea (4 variants: calm, light ripples, crest ripples, deep swell; deep blue) |
| (2,5)–(2,8) | Coastline (4 cells: straight, inner corner, outer corner, diagonal/cove) |
| (2,9)–(2,16) | Dirt roads (8 cells: straight horizontal, straight vertical, corner NE, corner NW, corner SE, corner SW, crossroad, dead-end/end cap; T-junctions are composed in code from straight + perpendicular dead-end overlay) |
| (3,1)–(3,2) | Wooden bridge over water (2 cells: horizontal, vertical) |
| (3,3)–(3,4) | Ford / shallow water crossing (2 cells: stepping stones and gravel across shallow water: horizontal, vertical) |

### B. Camps & Objects (rows 4–6, outlined, standing on the tile)

Each camp building reflects the piece it grants. Instead of separate captured drawings, each camp has one drawing leaving an empty flag slot at the top-right; the controlling faction's flag (from row 8) is overlaid in code.

| Cells | Content |
|---|---|
| (4,1)–(5,2) | Fortress (2×2 cells: stout stone keep with rook crenellations, flag slot at top-right) |
| (4,3)–(5,4) | Citadel (2×2 cells: grand castle with queen crown spires, flag slot at top-right) |
| (4,5) | Village (1 cell: wood and thatch huts, flag slot at top-right) |
| (4,6) | Knight Camp (1 cell: stables with a horse motif, flag slot at top-right) |
| (4,7) | Bishop Camp (1 cell: stone chapel with a tall spire, flag slot at top-right) |
| (5,5) | Wooden treasure chest (closed) |
| (5,6) | Wooden treasure chest (open) |
| (5,7) | Glowing stone shrine |
| (5,8) | Wooden signpost |
| (6,1)–(6,12) | Fog of war edge tiles (12 cells: dark, translucent smoke edges: 4 straight edges North, East, South, West; 4 outer corners NE, NW, SE, SW; 4 inner corners NE, NW, SE, SW. Full-fog fill is rendered as a flat colour in code, no tile needed) |

### C. Heroes & Flags (rows 7–8)

Hero tokens represent armies moving on the map.

| Cells | Content |
|---|---|
| (7,1)–(7,4) | Ashen Sun Hero (Lord on a white horse: 4 animation frames: idle, walk 1, walk 2, walk 3, all facing right, mirrored in code for left) |
| (7,5)–(7,8) | Hollow Crown Hero (Lich on a skeletal horse: 4 animation frames: idle, walk 1, walk 2, walk 3, all facing right, mirrored in code for left) |
| (8,1) | Ashen Sun flag/banner (gold sun motif, planted in the ground) |
| (8,2) | Hollow Crown flag/banner (dark iron skull motif, planted in the ground) |
| (8,3) | Neutral flag/banner (weathered grey cloth, planted in the ground) |

### D. UI & Markers (rows 9–10)

Reuse `assets-gui.md` where possible; these are overworld-specific.

| Cells | Content |
|---|---|
| (9,1) | Path footsteps marker (small dashed line) |
| (9,2) | Destination crosshair/X |
| (9,3) | Blocked path icon (red X) |
| (9,4) | Day/Turn counter frame (parchment and wood, small) |
| (9,5) | Army panel empty slot (wood recess) |
| (10,1) | Hero portrait frame (ornate gold and iron) |
| (10,2) | Ashen Sun Lord portrait |
| (10,3) | Hollow Crown Lich portrait |
| (10,4)–(10,5) | Battle-deploy zone highlight (2 cells: glowing blue player tile, glowing red enemy tile) |
| (10,6) | End Turn button icon (hourglass with an arrow) |

---

## 3. Prompt

> A 2048x2048 pixel-art sprite sheet on a flat magenta (#FF00FF) background, in the exact style of the attached reference sheet: modern indie pixel art, bold clean dark-navy outlines, vivid readable colours, soft cel shading, crisp pixels with no anti-aliasing. Every logical pixel is a 4x4 block. The sheet is a 16x16 grid of 128-px cells (no grid lines). Top-down 3/4 view. Light from the top-left.
>
> Rows 1-3: Seamless full-cell terrain tiles without outlines. Row 1: 4 grass plains variants, 4 dense green forest canopy tiles, 4 rugged rocky hills tiles, 4 tall impassable mountain peaks. Row 2: 4 deep blue water tiles, 4 coastline edge tiles (straight, inner corner, outer corner, diagonal/cove), 8 dirt road segments (straight horizontal, straight vertical, 4 corners NE/NW/SE/SW, crossroad, dead-end cap; T-junctions composed in code). Row 3: wooden bridge over water (horizontal, vertical), ford/shallow water crossing (horizontal, vertical).
>
> Rows 4-6: Outlined buildings, props, and fog tiles. Rows 4-5, left to right: single drawings of camp buildings, each leaving an empty flag slot at the top-right for a code overlay: a 2x2 stout stone fortress with rook crenellations (rows 4-5, cols 1-2), a 2x2 grand citadel castle with a queen's crown roof (rows 4-5, cols 3-4), then in row 4 a 1-cell wooden village (col 5), a 1-cell knight's stable with a horse motif (col 6) and a 1-cell stone chapel with a tall spire (col 7). Row 5, cols 5-8: a closed wooden chest, an open chest, a glowing stone shrine, and a wooden signpost. Row 6: 12 dark translucent smoke fog-of-war edge tiles (4 straight edges N/E/S/W, 4 outer corners, 4 inner corners; full-fog fill handled in code).
>
> Rows 7-8: Outlined character tokens and flags. Row 7: An Ashen Sun lord riding a white horse (facing right, 4 animation frames: idle, walk 1, walk 2, walk 3) and a Hollow Crown lich riding a skeletal horse (facing right, 4 animation frames: idle, walk 1, walk 2, walk 3). Row 8: Three planted flags/banners (an Ashen Sun gold-sun banner, a Hollow Crown dark-iron banner, and a weathered grey neutral banner).
>
> Rows 9-10: UI and markers. Row 9: A dotted footstep path marker, a destination X, a red blocked icon, a parchment Day counter frame, and an empty wooden army slot. Row 10: An ornate portrait frame, a portrait of the Ashen Sun lord, a portrait of the Hollow Crown lich, 2 battle-deploy zone highlight tiles (glowing blue, glowing red), and an hourglass End Turn icon.
>
> Rows 11-16: Empty magenta.

---

## 4. Acceptance checklist

- [ ] Terrain tiles repeat seamlessly without outlines.
- [ ] Buildings, characters, and props have clean dark-navy outlines.
- [ ] Camp buildings distinctly resemble their corresponding chess pieces (knight, bishop, rook, queen).
- [ ] Camp buildings leave an empty flag slot at the top-right for faction banner overlay.
- [ ] Table row cell counts match the map and listed items exactly.
- [ ] Background is pure magenta with no gradients.
- [ ] Colours match the main sheet; pixels are crisp 4×4 blocks.
