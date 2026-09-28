# Terrain Chess — GUI Sheet Spec

Art for the on-screen interface: card frames and card art, buttons, panels, the hand bar,
badges and small icons. The board and pieces stay on `SPRITES.md`; the island and sky are on
`ENVIRONMENT_SPRITES.md`. This is a third, separate sheet.

**Style:** the same as `SPRITES.md`: modern indie pixel art, bold clean outlines, vivid readable
colours, soft cel shading. Fantasy with a dark edge: carved wood, parchment, iron fittings, gold
trim for rare things. The GUI is drawn on top of a colourful 3D scene, so every panel is
**fully opaque** and has a clear dark outline.

- §3 is the **prompt**. Paste it to the image model **with `assets/spritesheet.jpg` attached as
  the style reference**.
- §2 is the **cell map** the game slices by.

---

## 0. Technical contract

| Item | Value |
|---|---|
| File | `assets/gui.png` (PNG preferred; JPG accepted) |
| Canvas | **2048 × 2048 px** = **16 × 16 grid** of **128 × 128 px cells** |
| Scale | **4×**: 1 logical pixel = one 4×4 block |
| Background | Flat **magenta `#FF00FF`** wherever there is no art; unused cells stay magenta |
| Outline | 1 logical px dark navy `#1A1C2C` around every frame, button, panel and icon |
| Light | From the **top-left** |
| Rendering | Crisp pixels, no anti-aliasing, no blur, **no text or letters anywhere** (the game draws all text) |
| Palette | The §1 palette of `SPRITES.md` |

**Cell numbering is 1-based: (row, col).** A frame spanning several cells is written
`(r,c)–(r2,c2)`.

**Nine-slice frames.** Panels and buttons are stretched to any size in game with nine-slice
scaling: the four corners stay fixed, the edges repeat, the centre stretches. So for every item
marked *9-slice*:
- corners and ornaments sit only in the outer **16 logical px** (64 image px) of each side;
- the edges between corners are plain and uniform, so they can repeat;
- the centre is a flat, calm fill (subtle texture only), with room for text.

**Card frames are not stretched:** they are shown at their drawn size (scaled by whole
numbers), so they can have detail anywhere.

---

## 1. Sheet layout

```
rows 1-3    A. Card frames: 4 rarities + card back + 2 states        (each 2 × 3 cells)
rows 4-5    B. Card art, 16 items                                    (each 1 × 1 cell, row 4-5)
rows 6-7    B. Card art, 8 spells                                    (each 1 × 1 cell)
rows 8-9    C. Buttons (9-slice), 3 styles × 4 states                (each 2 × 1 cells)
rows 10-12  D. Panels (9-slice): wood, parchment, stone, dark glass  (each 3 × 3 cells)
row  13     E. Hand bar pieces
row  14     F. Badges and banners
row  15     G. Small icons
row  16     (empty, magenta)
```

---

## 2. Cell map

### A. Card frames (rows 1–3, each frame 2 cols × 3 rows = 256 × 384 px, portrait 2:3)

A card frame is a vertical card with: a **title plate** across the top (plain, for the item
name), an **art window** in the upper middle (a recessed square about 160 × 160 image px, left
empty/dark so the game places card art there), and a **text box** in the lower third (plain
parchment or a flat inner panel, for the description). Rarity shows in the border material and
the gem at the top centre.

| Cells | Content |
|---|---|
| (1,1)–(3,2) | **Common:** weathered grey stone border, iron corner studs, pale parchment text box, no gem |
| (1,3)–(3,4) | **Uncommon:** polished blue steel border with silver rivets, a small sapphire at the top |
| (1,5)–(3,6) | **Rare:** carved violet-black wood with gold filigree corners, an amethyst at the top, faint glow along the edge |
| (1,7)–(3,8) | **Spell card:** teal arcane border with small rune marks along the sides, an aquamarine at the top (used for cards in the hand) |
| (1,9)–(3,10) | **Card back:** dark navy with a gold compass-and-mountain emblem in the centre (deck pile, face-down draws) |
| (1,11)–(3,12) | **Used overlay:** the same card silhouette as the others, filled with a translucent-looking dark grey (draw it opaque mid-grey; the game makes it translucent) with a diagonal crack, shown over a spent card |
| (1,13)–(3,14) | **Highlight overlay:** only a 2-logical-px glowing gold outline in the card silhouette with small sparkles at the corners, transparent (magenta) inside, for the armed / hovered card |

### B. Card art (rows 4–7, one illustration per cell, 128 × 128, fills the cell edge to edge with a dark vignette, no outline around the square)

Each is a small scene or emblem that reads at 64 px.

| Cell | Item | Picture |
|---|---|---|
| (4,1) | Mountaineer Rooks | a stone rook tower with a climbing rope and pitons on a cliff |
| (4,2) | Amphibious Knights | a horse head rising from dark waves |
| (4,3) | Surefooted Pawns | a small iron boot on a rocky step |
| (4,4) | Momentum Bishops | a bishop's mitre sliding down a hill with motion lines |
| (4,5) | Long-Jump Knights | a knight silhouette leaping in a high arc over a cliff |
| (4,6) | Daring Queens | a crown on a figure diving off a ledge, cape flowing |
| (4,7) | Tunneler Bishops | a cave mouth with a bishop's staff glowing inside |
| (4,8) | Veteran | a dented shield with a sword scar |
| (5,1) | Tectonic Pact | two hands pushing a slab of earth upward |
| (5,2) | Calm Terrain | gentle green hills under a soft sun |
| (5,3) | Tide Charm | a shell amulet with receding water |
| (5,4) | Cartographer | an unrolled map with a red X and a compass |
| (5,5)–(5,8) | spare | four generic art pieces: a treasure chest, a scroll, a key, a lantern |
| (6,1) | Raise Earth | a square block of earth thrusting up out of grass |
| (6,2) | Lower Earth | a square pit sinking with falling pebbles |
| (6,3) | Freeze | a pond freezing into ice with a snowflake |
| (6,4) | Bridge | a wooden plank bridge over a gap |
| (6,5) | Dig Tunnel | two cave mouths joined by a glowing dotted arc |
| (6,6) | Shield | a translucent blue bubble over a pawn |
| (6,7) | Swap | two chess pieces with curved arrows swapping |
| (6,8) | Rewind | an hourglass with a counter-clockwise arrow |
| (7,1)–(7,8) | spare | eight generic art pieces: fire, lightning, a feather, a gem, a skull, a sun, a moon, a star |

### C. Buttons (rows 8–9, each button 2 cols × 1 row = 256 × 128 px, *9-slice*, no text)

| Cells | Style: Wood (normal · hover · pressed · disabled) |
|---|---|
| (8,1)–(8,2), (8,3)–(8,4), (8,5)–(8,6), (8,7)–(8,8) | Carved wood plank with iron corner brackets. Hover: lighter wood and a thin gold rim. Pressed: shifted down 1 logical px, darker, no drop shadow. Disabled: desaturated grey wood |
| (8,9)–(8,10), (8,11)–(8,12), (8,13)–(8,14), (8,15)–(8,16) | **Style: Gold (primary action)** — gold-trimmed crimson lacquer, same four states |
| (9,1)–(9,2), (9,3)–(9,4), (9,5)–(9,6), (9,7)–(9,8) | **Style: Stone (secondary)** — grey stone slab, same four states |
| (9,9) | Round icon button, normal (1 cell, circular wood with iron rim) |
| (9,10) | Round icon button, hover |
| (9,11) | Round icon button, pressed |
| (9,12) | Round icon button, disabled |

### D. Panels (rows 10–12, each panel 3 × 3 cells = 384 × 384 px, *9-slice*)

| Cells | Content |
|---|---|
| (10,1)–(12,3) | **Wood panel:** dark oak planks, iron corner brackets (floor badge, small boxes) |
| (10,4)–(12,6) | **Parchment panel:** cream parchment with a thin brown ink border and curled corners (reward screen, run over) |
| (10,7)–(12,9) | **Stone panel:** grey carved stone with moss in the corners (title menu) |
| (10,10)–(12,12) | **Dark glass panel:** near-black navy with a thin gold inner line, very calm centre (tooltips, hint text) |
| (10,13)–(12,15) | **Banner:** a horizontal cloth ribbon with folded ends, crimson with gold trim (titles). 9-slice horizontally only: the folded ends are the left and right 24 logical px |

### E. Hand bar (row 13)

| Cells | Content |
|---|---|
| (13,1)–(13,4) | **Hand tray:** a long wooden tray with a lip that the three cards sit in (512 × 128 px, 9-slice horizontally) |
| (13,5) | **Deck pile:** a small stack of face-down cards (card back from A, seen slightly from above, 4 cards thick) |
| (13,6) | **Empty card slot:** a faint dashed outline of a card, carved into wood |
| (13,7) | **Discard icon:** a card being tossed with a small swoosh |
| (13,8) | **Draw icon:** a card rising from the deck with a sparkle |

### F. Badges and banners (row 14)

| Cells | Content |
|---|---|
| (14,1)–(14,2) | **Floor badge:** a shield-shaped wooden plaque with an empty centre for the floor number (256 × 128) |
| (14,3)–(14,4) | **Boss floor badge:** the same plaque in black iron with red gems and horns |
| (14,5)–(14,6) | **Victory banner end piece:** gold laurel wreath |
| (14,7)–(14,8) | **Defeat banner end piece:** broken sword over a cracked shield |
| (14,9) | **Floor pip, cleared:** small gold diamond |
| (14,10) | **Floor pip, current:** glowing white diamond |
| (14,11) | **Floor pip, locked:** dark grey diamond |
| (14,12) | **Floor pip, boss:** small red skull diamond |

### G. Small icons (row 15, each centred in its cell, about 64 × 64 px of art)

| Cell | Icon |
|---|---|
| (15,1) | Menu (three horizontal bars in a wooden frame) |
| (15,2) | Sound on (horn) |
| (15,3) | Sound off (horn with a red slash) |
| (15,4) | Undo (curved arrow) |
| (15,5) | Rotate left |
| (15,6) | Rotate right |
| (15,7) | Heights / terrain (three stepped blocks) |
| (15,8) | Enemy modifier (red skull) |
| (15,9) | Relic (gold amulet) |
| (15,10) | Enhancement (upward chevron on a shield) |
| (15,11) | Spell (open book with a glow) |
| (15,12) | Chest (small closed chest) |
| (15,13) | Close (red X) |
| (15,14) | Check / confirm (green tick) |
| (15,15) | Info (i in a circle) |
| (15,16) | Lock |

---

## 3. Prompt

> A 2048×2048 pixel-art user-interface sprite sheet on a flat magenta (#FF00FF) background, in the
> exact style of the attached reference sheet: modern indie pixel art, bold clean dark-navy
> outlines, vivid readable colours, soft cel shading, crisp pixels with no anti-aliasing and no
> blur. Every logical pixel is a 4×4 block. The sheet is a 16×16 grid of 128-px cells (do not
> draw grid lines). **Absolutely no text, letters or numbers anywhere.** Fantasy with a dark
> edge: carved wood, parchment, iron fittings, gold trim. Light from the top-left.
>
> Rows 1–3: seven portrait playing-card frames, each 2 cells wide and 3 cells tall, side by
> side. Each has a plain title plate at the top, an empty dark square art window in the upper
> middle, and a plain text box in the lower third. Left to right: a weathered grey stone frame
> with iron studs; a polished blue steel frame with silver rivets and a small sapphire; a carved
> violet-black wood frame with gold filigree corners and an amethyst; a teal arcane frame with
> rune marks and an aquamarine; a card back in dark navy with a gold compass-and-mountain emblem;
> a plain mid-grey card silhouette with a diagonal crack; and a card-shaped glowing gold outline
> with corner sparkles and an empty magenta inside.
>
> Rows 4–7: thirty-two small square illustrations filling their cells edge to edge with a dark
> vignette, no border: a rook tower with climbing rope on a cliff; a horse head rising from
> waves; an iron boot on a rocky step; a bishop's mitre sliding downhill; a knight leaping over a
> cliff; a crowned figure diving off a ledge; a cave mouth with a glowing staff inside; a dented
> shield with a sword scar; hands pushing up a slab of earth; gentle green hills under a soft
> sun; a shell amulet with receding water; a map with a red X and a compass; a treasure chest; a
> scroll; a key; a lantern; an earth block thrusting up from grass; a square pit with falling
> pebbles; a pond freezing with a snowflake; a plank bridge over a gap; two cave mouths joined by
> a glowing dotted arc; a blue bubble over a pawn; two chess pieces with swapping arrows; an
> hourglass with a counter-clockwise arrow; fire; lightning; a feather; a gem; a skull; a sun; a
> moon; a star.
>
> Rows 8–9: wide buttons, each 2 cells wide and 1 cell tall, with plain uniform edges so they can
> be stretched: four states (normal, hover with a thin gold rim, pressed and darker, disabled and
> grey) of a carved wood plank button with iron corner brackets, the same four states of a gold-
> trimmed crimson lacquer button, and the same four states of a grey stone slab button; then four
> states of a round wooden icon button with an iron rim.
>
> Rows 10–12: five stretchable panels, each 3×3 cells, with ornaments only in the corners, plain
> uniform edges and a calm flat centre: dark oak planks with iron brackets; cream parchment with
> a thin brown ink border and curled corners; grey carved stone with moss in the corners; a
> near-black navy glass panel with a thin gold inner line; and a crimson cloth ribbon banner with
> folded ends and gold trim.
>
> Row 13: a long wooden card tray 4 cells wide; a small stack of face-down navy-and-gold cards; a
> faint dashed card outline carved into wood; a card being tossed with a swoosh; a card rising
> from a deck with a sparkle.
>
> Row 14: a shield-shaped wooden plaque 2 cells wide with an empty centre; the same plaque in
> black iron with red gems and horns; a gold laurel wreath; a broken sword over a cracked shield;
> four small diamonds: gold, glowing white, dark grey, and red with a tiny skull.
>
> Row 15: sixteen small icons centred in their cells: a menu icon of three bars, a horn, a horn
> with a red slash, a curved undo arrow, rotate-left and rotate-right arrows, three stepped
> blocks, a red skull, a gold amulet, an upward chevron on a shield, an open glowing book, a
> small closed chest, a red X, a green tick, an "i" in a circle drawn as a symbol, and a lock.
>
> Row 16 stays empty magenta.

---

## 4. Acceptance checklist

- [ ] No text, letters or numbers anywhere on the sheet
- [ ] Nine-slice items keep all ornaments within the outer 16 logical px and have plain,
      uniform edges between the corners
- [ ] All seven card frames share the same silhouette and art-window position
- [ ] Card art fills its cell and reads clearly when shown at 64 × 64 px
- [ ] Every item and spell in `assets/items/*.ron` has its own card art
- [ ] Background is pure magenta; nothing crosses its cell or frame border
- [ ] Colours match the main sheet; pixels are crisp 4×4 blocks
