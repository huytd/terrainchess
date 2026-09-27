#!/usr/bin/env python3
"""Turn the generated sprite sheet into the game's atlas (see SPRITES.md §0).

    python3 tools/process_sprites.py [--palette] [--preview out.png]

Reads  assets/spritesheet.jpg  (2048², magenta background)
Writes assets/atlas.png        (packed, transparent, 1 logical px = 1 px)
       assets/atlas.ron        (sprite name -> rect + anchor, read by the game)

The generated sheet came out as a 16×16 grid of 128 px cells instead of the
32×32 grid of 64 px cells in SPRITES.md, and some rows ignore the grid, so
regions are listed by hand below. Tile regions are sliced strictly by cell.
Sprite regions give a search box; the sprite is the non-background pixels found
inside it, so small misalignments don't matter.

Pipeline per sprite:
  1. Chroma-key the magenta (tolerant of JPEG noise) and erode the key by 1 px
     to drop the magenta fringe.
  2. Downscale 4× (128 px cell -> 32 px tile) with alpha-weighted box filtering,
     then threshold alpha so edges stay crisp.
  3. Optionally snap colours to the SPRITES.md §1 palette (--palette).

Requires Pillow, numpy and scipy.
"""

import argparse
import os
import sys

import numpy as np
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "assets", "spritesheet.jpg")
OUT_PNG = os.path.join(ROOT, "assets", "atlas.png")
OUT_RON = os.path.join(ROOT, "assets", "atlas.ron")

CELL = 128  # source px per grid cell
TILE = 32  # logical px per board tile
SCALE = TILE / CELL


def cell(r, c, w=1, h=1):
    """Source box of grid cells (0-based row/col)."""
    return (c * CELL, r * CELL, (c + w) * CELL, (r + h) * CELL)


# --- Region table ---------------------------------------------------------------
# tile(name, row, col): seamless ground, full cell, opaque, 32×32.
# face(name, row, col): full cell with transparency kept, 32×32 (cliff faces).
# sprite(name, box, height=None, anchor): trimmed sprite; `height` rescales it to a
#   target height in logical px (default: plain 4× downscale).

TILES = [
    ("grass_0", 0, 0),
    ("grass_1", 0, 1),
    ("grass_dark", 0, 2),
    ("moss", 0, 3),
    ("soil", 0, 4),
    ("cobble", 0, 5),
    ("flagstone", 0, 6),
    ("grave_dirt", 0, 7),
    ("sand", 3, 0),
    ("shallow_0", 4, 0),
    ("shallow_1", 4, 1),
    ("deep", 4, 2),
    ("bog_0", 4, 3),
    ("bog_1", 4, 4),
    ("lava_0", 4, 5),
    ("lava_1", 4, 6),
    ("void", 4, 7),
    ("ice", 5, 4),
    ("ice_cracked", 5, 5),
]

FACES = [
    ("cliff_0", 0, 8),
    ("cliff_1", 0, 9),
    ("cliff_2", 0, 10),
    ("cliff_3", 0, 11),
    ("cliff_4", 0, 12),
]

BOTTOM = (0.5, 1.0)  # anchor: feet / base at the bottom centre
CENTER = (0.5, 0.5)

# Pieces: idle frames, pawn smallest to king tallest. Kept shorter than SPRITES.md §0 so a
# piece covers at most about half of the square behind it.
PIECE_H = {"pawn": 26, "knight": 30, "rook": 32, "bishop": 36, "queen": 38, "king": 40}

SPRITES = [
    # Ashen Sun (white)
    ("white_pawn", cell(8, 0), PIECE_H["pawn"], BOTTOM),
    ("white_pawn_ascended", cell(8, 7), PIECE_H["pawn"] + 2, BOTTOM),
    ("white_knight", cell(9, 2), PIECE_H["knight"], BOTTOM),
    ("white_rook", cell(9, 8), PIECE_H["rook"], BOTTOM),
    ("white_bishop", cell(10, 8, 1, 2), PIECE_H["bishop"], BOTTOM),
    ("white_queen", cell(10, 12, 1, 2), PIECE_H["queen"], BOTTOM),
    # white_king has no full-body cell on the sheet; see WHITE_KING below.
    # Hollow Crown (black)
    ("black_pawn", cell(8, 8), PIECE_H["pawn"], BOTTOM),
    ("black_knight", cell(9, 14), PIECE_H["knight"], BOTTOM),
    ("black_rook", cell(9, 11), PIECE_H["rook"], BOTTOM),
    ("black_bishop", cell(10, 11, 1, 2), PIECE_H["bishop"], BOTTOM),
    ("black_queen", cell(10, 13, 1, 2), PIECE_H["queen"], BOTTOM),
    ("black_king", cell(10, 15, 1, 2), PIECE_H["king"], BOTTOM),
    # Portraits and crests for the HUD
    ("portrait_white_king", cell(12, 8), None, CENTER),
    ("portrait_black_king", cell(12, 11), None, CENTER),
    ("crest_white", cell(12, 14), None, CENTER),
    ("crest_black", cell(12, 15), None, CENTER),
    # Terrain features
    ("cave_mouth", (1024, 256, 1152, 400), None, BOTTOM),
    ("cave_door_0", (1152, 256, 1280, 400), None, BOTTOM),
    ("cave_door_1", (1280, 256, 1408, 400), None, BOTTOM),
    ("cave_door_2", (1408, 256, 1536, 400), None, BOTTOM),
    ("cave_rune_0", (1168, 408, 1280, 482), None, CENTER),
    ("rock", cell(6, 0), None, BOTTOM),
    ("rock_mossy", cell(7, 0), None, BOTTOM),
    ("pine", (1024, 578, 1152, 768), 56, BOTTOM),
    ("dead_tree", (1152, 578, 1262, 768), 56, BOTTOM),
    ("bush", cell(7, 2), None, BOTTOM),
    # Board overlays (row 13 ignores the grid; ~100 px apart). The "pale tint" came
    # out magenta-blended, so tints are drawn as plain colour quads in game instead.
    ("ov_dot", (30, 1690, 75, 1738), None, CENTER),
    ("ov_capture", (108, 1664, 204, 1762), None, CENTER),
    ("ov_select", (208, 1664, 304, 1762), None, CENTER),
    ("ov_check", (418, 1670, 506, 1758), None, CENTER),
    ("ov_rune", (518, 1670, 608, 1758), None, CENTER),
    ("ov_hover", (618, 1668, 712, 1760), None, CENTER),
    ("ov_blocked", (730, 1676, 806, 1752), None, CENTER),
    ("ov_ring", (822, 1668, 918, 1760), None, CENTER),
]

# The sheet has no full-body Ashen King, so one is assembled in source pixels: the
# portrait bust (crown, head, ermine cape) over the bishop's robe and sun staff.
WHITE_KING = {
    "body": cell(10, 8, 1, 2),  # bishop, 128×256
    "head_cut": (105, 84),  # blank the bishop's mitre and face: rows < 105, cols < 84
    "bust": cell(12, 8),
    "bust_scale": 0.9,
    "bust_bottom": 130,  # bust's lower edge, in body-cell rows
}


# SPRITES.md §1
PALETTE = """
1A1C2C 2E6B3A 3F8F45 6BBF4E A6DB6A 24503A 33704A 1F4A45 2F6B57 4FA36B 86D07A
2E4A55 44707A 6C9DA3 A5CCD0 2D6F7A 3F9BA0 7FD0CC E8FFF8 3B5A3A 52774A 7A9A5A
7A1E1E D8431F FF8A2A FFD35A 4A2F2A 6E4A3A 93684E 4E5563 6E7686 9AA3B3 4A2E22
6E4633 96633F 8F8570 C9BFA3 EFE6CE C98E6E F2C29B 7C8699 B7C0D0 EEF2F8 E9E4D6
8A5A1E D19A2E 3F7FD1 79B4F2 FFF1B8 1F2130 343A52 565E7A 5E1225 9E2235 D8454E
5B2A86 9A55D6 D9A6FF 1E7A70 2BB5A6 7FF3E0 C8B384 E9DAB2 F7EDD0 3B2F2A
"""


def palette_array():
    cols = [tuple(int(h[i : i + 2], 16) for i in (0, 2, 4)) for h in PALETTE.split()]
    return np.array(cols, dtype=np.float32)


def key_mask(rgb):
    """True where the pixel is foreground (not magenta background)."""
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    bg = (r > 170) & (b > 170) & (g < 110) & (np.abs(r - b) < 70)
    fg = ~bg
    # Erode by one pixel: JPEG leaves a magenta-tinted fringe around sprites.
    e = fg.copy()
    e[1:, :] &= fg[:-1, :]
    e[:-1, :] &= fg[1:, :]
    e[:, 1:] &= fg[:, :-1]
    e[:, :-1] &= fg[:, 1:]
    return e


def components(mask, min_area):
    """Bounding boxes (x0, y0, x1, y1) of 8-connected blobs of at least min_area px."""
    try:
        from scipy import ndimage

        labels, _ = ndimage.label(mask, structure=np.ones((3, 3)))
        boxes = []
        for i, sl in enumerate(ndimage.find_objects(labels), start=1):
            if sl is None:
                continue
            area = int((labels[sl] == i).sum())
            if area >= min_area:
                boxes.append((sl[1].start, sl[0].start, sl[1].stop, sl[0].stop))
        return boxes
    except ImportError:
        sys.exit("scipy is needed: pip install scipy")


def downscale(rgb, alpha, out_w, out_h):
    """Alpha-weighted box downscale, then a hard alpha threshold."""
    h, w = alpha.shape
    ys = np.linspace(0, h, out_h + 1).astype(int)
    xs = np.linspace(0, w, out_w + 1).astype(int)
    out = np.zeros((out_h, out_w, 4), dtype=np.float32)
    for j in range(out_h):
        for i in range(out_w):
            a = alpha[ys[j] : max(ys[j + 1], ys[j] + 1), xs[i] : max(xs[i + 1], xs[i] + 1)]
            c = rgb[ys[j] : max(ys[j + 1], ys[j] + 1), xs[i] : max(xs[i + 1], xs[i] + 1)]
            cover = a.mean()
            if cover >= 0.5:
                wsum = a.sum()
                out[j, i, :3] = (c * a[..., None]).sum((0, 1)) / wsum
                out[j, i, 3] = 255
    return out


def snap(img, pal):
    rgb = img[..., :3].reshape(-1, 3)
    d = ((rgb[:, None, :] - pal[None, :, :]) ** 2).sum(-1)
    img[..., :3] = pal[d.argmin(1)].reshape(img.shape[:2] + (3,))
    return img


def white_king(crop):
    """Composite the full-body white king (see WHITE_KING) on a magenta canvas."""
    k = WHITE_KING
    body = crop(k["body"]).copy()
    rows, cols = k["head_cut"]
    body[:rows, :cols] = (255, 0, 255)
    bust = Image.fromarray(crop(k["bust"]).astype(np.uint8))
    side = round(CELL * k["bust_scale"])
    bust = np.asarray(bust.resize((side, side), Image.LANCZOS)).astype(np.float32)
    mask = key_mask(bust)
    x0, y0 = (body.shape[1] - side) // 2, k["bust_bottom"] - side
    body[y0 : y0 + side, x0 : x0 + side][mask] = bust[mask]
    return body


def process(src, palette):
    rgb_all = np.asarray(src.convert("RGB")).astype(np.float32)
    out = {}  # name -> (RGBA float array, anchor)

    def crop(box):
        x0, y0, x1, y1 = box
        return rgb_all[y0:y1, x0:x1]

    for name, r, c in TILES:
        x0, y0, x1, y1 = cell(r, c)
        # Skip the thin grid line the model drew around terrain cells.
        rgb = crop((x0 + 4, y0 + 4, x1 - 4, y1 - 4))
        out[name] = (downscale(rgb, np.ones(rgb.shape[:2], np.float32), TILE, TILE), CENTER)

    for name, r, c in FACES:
        x0, y0, x1, y1 = cell(r, c)
        rgb = crop((x0 + 3, y0, x1 - 3, y1))  # drop the grid line at the sides
        out[name] = (downscale(rgb, key_mask(rgb).astype(np.float32), TILE, TILE), (0.5, 0.0))  # anchored at its top edge

    def trimmed(rgb, target_h):
        mask = key_mask(rgb)
        boxes = components(mask, 60)
        if not boxes:
            raise SystemExit("empty sprite region")
        # Drop stray bits of neighbouring sprites: keep blobs near the largest one's size.
        area = lambda b: (b[2] - b[0]) * (b[3] - b[1])
        biggest = max(area(b) for b in boxes)
        boxes = [b for b in boxes if area(b) >= 0.08 * biggest]
        x0 = min(b[0] for b in boxes)
        y0 = min(b[1] for b in boxes)
        x1 = max(b[2] for b in boxes)
        y1 = max(b[3] for b in boxes)
        rgb, mask = rgb[y0:y1, x0:x1], mask[y0:y1, x0:x1]
        h, w = mask.shape
        s = target_h / h if target_h else SCALE
        return downscale(rgb, mask.astype(np.float32), max(1, round(w * s)), max(1, round(h * s)))

    for name, box, target_h, anchor in SPRITES:
        out[name] = (trimmed(crop(box), target_h), anchor)
    out["white_king"] = (trimmed(white_king(crop), PIECE_H["king"]), BOTTOM)

    if palette:
        pal = palette_array()
        for name, (img, anchor) in out.items():
            out[name] = (snap(img, pal), anchor)
    return out


def pack(sprites, width=512, pad=1):
    """Shelf packer, tallest first. Returns (atlas image, {name: (x, y, w, h, anchor)})."""
    order = sorted(sprites, key=lambda n: (-sprites[n][0].shape[0], n))
    x = y = shelf_h = 0
    rects = {}
    for name in order:
        img = sprites[name][0]
        h, w = img.shape[:2]
        if x + w > width:
            x, y, shelf_h = 0, y + shelf_h + pad, 0
        rects[name] = (x, y, w, h)
        x += w + pad
        shelf_h = max(shelf_h, h)
    height = y + shelf_h
    atlas = np.zeros((height, width, 4), dtype=np.uint8)
    for name, (x, y, w, h) in rects.items():
        atlas[y : y + h, x : x + w] = sprites[name][0].astype(np.uint8)
    return Image.fromarray(atlas, "RGBA"), {
        n: (*rects[n], sprites[n][1]) for n in sorted(rects)
    }


def write_ron(rects, path):
    lines = ["// Generated by tools/process_sprites.py. Do not edit.", "(", "    sprites: {"]
    for name, (x, y, w, h, (ax, ay)) in rects.items():
        lines.append(
            f'        "{name}": (x: {x}, y: {y}, w: {w}, h: {h}, anchor: ({ax:.2f}, {ay:.2f})),'
        )
    lines += ["    },", ")", ""]
    with open(path, "w") as f:
        f.write("\n".join(lines))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--palette", action="store_true", help="snap colours to the SPRITES.md palette")
    ap.add_argument("--preview", help="also write a 4× preview of the atlas here")
    args = ap.parse_args()

    sprites = process(Image.open(SRC), args.palette)
    atlas, rects = pack(sprites)
    atlas.save(OUT_PNG)
    write_ron(rects, OUT_RON)
    if args.preview:
        big = atlas.resize((atlas.width * 4, atlas.height * 4), Image.NEAREST)
        bg = Image.new("RGBA", big.size, (40, 44, 60, 255))
        Image.alpha_composite(bg, big).save(args.preview)
    print(f"{len(rects)} sprites -> {OUT_PNG} ({atlas.width}×{atlas.height}), {OUT_RON}")


if __name__ == "__main__":
    main()
