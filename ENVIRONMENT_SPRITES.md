# Terrain Chess — Environment Sheet Spec

Art for everything **around** the board: the sky and horizon, the sea, the island shore,
ground details and small ambient life. The chess pieces and board tiles stay on the main
sheet (`SPRITES.md`); this is a second, separate sheet.

**Style:** the same as `SPRITES.md`: modern indie pixel art, bold clean outlines, vivid
readable colours, soft cel shading, chunky shapes. Fantasy with a dark edge. The board sits on
the high plateau of a small island in a calm sea, under a wide late-afternoon sky.

- §3 is the **prompt**. Paste it to the image model **with the current `assets/spritesheet.jpg`
  attached as the style reference**.
- §2 is the **cell map** the game slices by.

---

## 0. Technical contract

| Item | Value |
|---|---|
| File | `assets/environment.png` (PNG preferred; JPG is accepted) |
| Canvas | **2048 × 2048 px** = **16 × 16 grid** of **128 × 128 px cells** |
| Scale | **4×**: 1 logical pixel = one 4×4 block, so 1 cell = 32 × 32 logical px = one board square |
| Background | Flat **magenta `#FF00FF`** wherever there is no art. Unused cells stay magenta |
| Light | From the **top-left**, warm late-afternoon sun |
| Outline | 1 logical px dark navy `#1A1C2C` on props, birds and decals. **No outlines** on the horizon bands, clouds or seamless tiles |
| Rendering | Crisp pixels, no anti-aliasing, no blur, no dithering noise, no text, no grid lines |
| Palette | The §1 palette of `SPRITES.md`, plus soft sky tints (pale peach, lilac, haze blue) for §2 A–B |
| Seamless | Horizon bands tile **left-to-right** (the left and right edges must join). Sea and sand tiles tile in **both** directions |

**Cell numbering is 1-based: (row, col).** Pixel boxes are given as `x0,y0 – x1,y1`
(end exclusive).

**How the game uses it** (`tools/process_sprites.py` will slice it into the atlas):
- Horizon bands are drawn as three layers on a huge cylinder around the island, far to near,
  and fade into the fog. They must read as silhouettes against a pale sky.
- Clouds are big flat cards drifting slowly across the sky above the horizon.
- Sea tiles cover the water around the island; foam tiles line the shore and are rotated in
  game for each side and corner.
- Ground decals lie **flat** on top of grass squares seen from above, so draw them top-down.
- Ambient sprites are small billboards (birds circling, sparkles, drifting leaves).

---

## 1. Sheet layout

```
rows 1-3   A. Far horizon band: mountain range            (2048 × 384)
rows 4-5   A. Mid horizon band: hills, forest, far castle (2048 × 256)
rows 6-7   A. Near horizon band: treeline and sea cliffs  (2048 × 256)
rows 8-9   B. Clouds, 8 × (2 × 2 cells)
row  10    C. Sea and shore tiles
row  11    D. Ground decals (top-down)
row  12    E. Ambient animation frames
rows 13-16 (empty, magenta)
```

---

## 2. Cell map

### A. Horizon bands (full-width strips, seamless left-right)

Each band has sky-free magenta above its silhouette so the game can layer them. Silhouettes
get lighter and bluer with distance (atmospheric perspective). No outlines.

| Rows | Box | Content |
|---|---|---|
| 1–3 | `0,0 – 2048,384` | **Far:** a jagged snow-capped mountain range in pale haze blue / lilac, peaks reaching the top third, flat base along the bottom edge |
| 4–5 | `0,384 – 2048,640` | **Mid:** rolling blue-green hills with pine forest edges, one small ruined castle silhouette and one lone tower, a few faint warm window lights |
| 6–7 | `0,640 – 2048,896` | **Near:** dark green treeline and rocky sea cliffs meeting the water line along the bottom edge, slightly darker and more saturated |

### B. Clouds (rows 8–9)

Eight clouds, each filling one **2 × 2 cell** frame (256 × 256 px), soft pixel-art cumulus with
a warm lit top-left and a lilac shaded underside, flat bottoms. No outlines.

| Cells | Content |
|---|---|
| (8,1)–(9,2) | Large cumulus |
| (8,3)–(9,4) | Large cumulus, different shape |
| (8,5)–(9,6) | Medium cloud |
| (8,7)–(9,8) | Medium cloud |
| (8,9)–(9,10) | Long thin streak cloud |
| (8,11)–(9,12) | Long thin streak cloud |
| (8,13)–(9,14) | Small puff |
| (8,15)–(9,16) | Small puff pair |

### C. Sea and shore (row 10, one tile per cell, full cell, seamless)

| Cell | Content |
|---|---|
| (10,1) | Deep sea, frame A: dark teal with small wave glints |
| (10,2) | Deep sea, frame B: same, glints shifted (animation pair with 10,1) |
| (10,3) | Deep sea variant, frame A |
| (10,4) | Deep sea variant, frame B |
| (10,5) | Shallow sea, frame A: lighter turquoise, sandy floor faintly visible |
| (10,6) | Shallow sea, frame B |
| (10,7) | Wet sand |
| (10,8) | Dry sand with a few shells |
| (10,9) | **Shore, straight:** sand on the bottom half, foam line across the middle, shallow sea on the top half |
| (10,10) | **Shore, outer corner:** sand in the bottom-left quarter, foam curving around it, sea elsewhere |
| (10,11) | **Shore, inner corner:** sea in the top-right quarter only, foam curving around it, sand elsewhere |
| (10,12) | Shore straight, foam frame B (animation pair with 10,9) |
| (10,13) | Rocky shore: grey boulders in shallow water |
| (10,14) | Pebble beach |
| (10,15) | Grass fading into sand (transition, grass on top half) |
| (10,16) | Seaweed patch in shallow water |

### D. Ground decals (row 11, flat, top-down, centred in the cell, about 64 × 64 px of art)

| Cells | Content |
|---|---|
| (11,1)–(11,4) | Grass tufts, four shapes |
| (11,5)–(11,8) | Flower patches: white, yellow, lilac, red |
| (11,9)–(11,10) | Pebbles, two arrangements |
| (11,11)–(11,12) | Fallen leaves (autumn orange), two arrangements |
| (11,13)–(11,14) | Mushroom rings / clusters, top-down |
| (11,15)–(11,16) | Moss patch, cracked stone slab |

### E. Ambient animation (row 12, side view, each sprite centred in its cell)

| Cells | Content |
|---|---|
| (12,1)–(12,2) | Seagull, wings up / wings down (about 40 × 20 px of art) |
| (12,3)–(12,4) | Crow, wings up / wings down |
| (12,5)–(12,6) | Small songbird, wings up / wings down |
| (12,7)–(12,10) | Sparkle / firefly, 4 frames (grow, bright, shrink, fade) |
| (12,11)–(12,14) | Falling leaf, 4 rotation frames |
| (12,15)–(12,16) | Butterfly, wings open / closed |

---

## 3. Prompt

> A 2048×2048 pixel-art sprite sheet on a flat magenta (#FF00FF) background, in the exact style
> of the attached reference sheet: modern indie pixel art, bold clean dark-navy outlines, vivid
> readable colours, soft cel shading, chunky shapes, crisp pixels with no anti-aliasing, no blur
> and no text. Every logical pixel is a 4×4 block. The sheet is a 16×16 grid of 128-px cells
> (do not draw grid lines). Light comes from the top-left, warm late-afternoon sun.
>
> Rows 1–3: one full-width seamless horizontal strip of a far snow-capped mountain range in pale
> haze blue and lilac, no outlines, magenta sky above it. Rows 4–5: a full-width seamless strip
> of rolling blue-green hills with pine forest edges, a small ruined castle and a lone tower
> silhouette with faint warm window lights, no outlines, magenta above. Rows 6–7: a full-width
> seamless strip of a dark green treeline and rocky sea cliffs meeting a water line at the bottom,
> no outlines, magenta above. Each strip's left and right edges must join seamlessly.
>
> Rows 8–9: eight soft pixel-art clouds, each filling a 2×2-cell frame, warm lit tops and lilac
> undersides, no outlines: two large cumulus, two medium, two long thin streaks, a small puff and
> a small puff pair.
>
> Row 10: sixteen seamless square tiles filling their cells: deep sea (two animation frames,
> then a variant with two frames), shallow turquoise sea (two frames), wet sand, dry sand with
> shells, a straight shoreline (sand bottom half, white foam line across the middle, shallow sea
> top half), an outer-corner shoreline (sand in the bottom-left quarter), an inner-corner
> shoreline (sea only in the top-right quarter), the straight shoreline again with the foam
> shifted, a rocky shore, a pebble beach, grass fading into sand, and seaweed in shallow water.
>
> Row 11: sixteen flat top-down ground decals centred in their cells, about half a cell wide,
> outlined: four grass tufts, four flower patches (white, yellow, lilac, red), two pebble
> clusters, two piles of orange fallen leaves, two mushroom clusters, a moss patch and a cracked
> stone slab.
>
> Row 12: small outlined side-view animation frames centred in their cells: a seagull with wings
> up and down, a crow with wings up and down, a small songbird with wings up and down, a
> four-frame sparkle (grow, bright, shrink, fade), a four-frame falling leaf rotating, and a
> butterfly with wings open and closed.
>
> Rows 13–16 stay empty magenta.

---

## 4. Acceptance checklist

- [ ] Horizon strips join seamlessly at their left and right edges
- [ ] Sea, sand and shore tiles repeat without visible seams
- [ ] Nothing crosses a cell border except the horizon strips and the 2 × 2 clouds
- [ ] Background is pure magenta with no gradient or noise
- [ ] Colours match the main sheet; pixels are crisp 4×4 blocks
