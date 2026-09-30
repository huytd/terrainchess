"""Procedural pixel art for the terrain, added to the atlas by process_sprites.py.

wall_rock  32×32  chunky rounded boulders for cliff sides; tiles in both directions.
grass_lip  32×16  a grass overhang with a bumpy, outlined hanging edge; tiles sideways,
                  transparent below the edge.
stone_lip  32×16  a cut-stone rim for board squares, in the board stone's greys.

Run on its own to write a 8× preview: python tools/gen_terrain_art.py /tmp/art.png
"""

import sys

import numpy as np

# Warm earthy rock, light to dark.
ROCK = [(214, 176, 128), (184, 142, 100), (152, 112, 80), (120, 86, 64), (88, 62, 50)]
ROCK_OUTLINE = (58, 42, 38)
# Grass, light to dark, plus the dark outline under the hanging edge.
GRASS = [(178, 222, 112), (136, 196, 90), (104, 164, 74), (78, 132, 64)]
GRASS_OUTLINE = (44, 86, 52)


def _rng(seed):
    return np.random.default_rng(seed)


def wall_rock(w=32, h=32, seed=7):
    rng = _rng(seed)
    # Boulder centres in staggered rows so the texture reads as stacked chunks.
    pts = []
    rows = 3
    for r in range(rows):
        y = (r + 0.5) * h / rows + rng.uniform(-2, 2)
        n = 3
        off = (r % 2) * (w / n / 2)
        for i in range(n):
            x = (i + 0.5) * w / n + off + rng.uniform(-2.5, 2.5)
            pts.append((x % w, y % h, rng.uniform(0.85, 1.15), rng.integers(0, 2)))
    img = np.zeros((h, w, 4), dtype=np.uint8)
    for yy in range(h):
        for xx in range(w):
            ds = []
            for (px, py, s, tone) in pts:
                # Wrap in both axes so the texture tiles.
                dx = min(abs(xx + 0.5 - px), w - abs(xx + 0.5 - px))
                dy = min(abs(yy + 0.5 - py), h - abs(yy + 0.5 - py))
                sx = xx + 0.5 - px
                sx = sx - w if sx > w / 2 else sx + w if sx < -w / 2 else sx
                sy = yy + 0.5 - py
                sy = sy - h if sy > h / 2 else sy + h if sy < -h / 2 else sy
                d = np.hypot(dx * 0.9, dy * 1.15) / s
                ds.append((d, sx, sy, tone))
            ds.sort(key=lambda t: t[0])
            d0, sx, sy, tone = ds[0]
            d1 = ds[1][0]
            if d1 - d0 < 1.1:
                c = ROCK_OUTLINE
            else:
                # Light from the upper left: lighter toward the top-left of each boulder,
                # darker toward its lower right and its rim.
                lit = (-sx * 0.35 - sy * 0.65) / 6.0 - (d0 / 9.0) * 0.8
                idx = 2 - int(np.clip(np.round(lit * 1.6), -2, 2)) + tone
                c = ROCK[int(np.clip(idx, 0, len(ROCK) - 1))]
            img[yy, xx, :3] = c
            img[yy, xx, 3] = 255
    return img


def grass_lip(w=32, h=16, seed=11):
    rng = _rng(seed)
    xs = np.arange(w)
    # Bumpy edge: a few whole sine periods across the width (so it tiles) plus tufts.
    edge = (
        8.5
        + 1.8 * np.sin(2 * np.pi * xs / w * 3 + 0.7)
        + 1.0 * np.sin(2 * np.pi * xs / w * 5 + 2.1)
    )
    for _ in range(4):
        t = rng.integers(0, w)
        for k in (-1, 0, 1):
            edge[(t + k) % w] += 1.3 if k == 0 else 0.6
    edge = np.clip(np.round(edge), 5, h - 2).astype(int)
    img = np.zeros((h, w, 4), dtype=np.uint8)
    for x in range(w):
        e = edge[x]
        for y in range(e + 1):
            if y == e:
                c = GRASS_OUTLINE
            elif y == 0:
                c = GRASS[0]
            elif y >= e - 1:
                c = GRASS[3]
            elif y >= e - 3:
                c = GRASS[2]
            else:
                c = GRASS[1]
            img[y, x, :3] = c
            img[y, x, 3] = 255
        # Sparse lighter blades in the body.
        if rng.random() < 0.35 and e > 4:
            y = int(rng.integers(1, e - 2))
            img[y, x, :3] = GRASS[0]
    return img


# Board stone greys, light to dark, and the outline under the rim.
STONE = [(150, 149, 153), (121, 120, 123), (104, 102, 107), (88, 86, 92)]
STONE_OUTLINE = (52, 50, 58)


def stone_lip(w=32, h=16):
    """A beveled stone band: light top edge, blocks split by joints, dark underside."""
    img = np.zeros((h, w, 4), dtype=np.uint8)
    band = 7
    for x in range(w):
        for y in range(band + 1):
            if y == band:
                c = STONE_OUTLINE
            elif y == 0:
                c = STONE[0]
            elif y >= band - 2:
                c = STONE[3]
            else:
                c = STONE[1] if (x // 4 + y) % 5 else STONE[2]
            if x % 16 == 0 and 0 < y < band:
                c = STONE_OUTLINE
            elif x % 16 == 1 and 0 < y < band - 1:
                c = STONE[0]
            img[y, x, :3] = c
            img[y, x, 3] = 255
    return img


def terrain_art():
    """Sprites for the atlas: name -> (RGBA uint8 array, anchor)."""
    return {
        "wall_rock": (wall_rock(), (0.5, 0.5)),
        "grass_lip": (grass_lip(), (0.5, 0.5)),
        "stone_lip": (stone_lip(), (0.5, 0.5)),
    }


if __name__ == "__main__":
    from PIL import Image

    out = sys.argv[1] if len(sys.argv) > 1 else "/tmp/terrain_art.png"
    rock = wall_rock()
    lip = grass_lip()
    sheet = np.zeros((32 * 2 + 16 * 2, 64, 4), dtype=np.uint8)
    sheet[..., 3] = 255
    sheet[:64, :64] = np.tile(rock, (2, 2, 1))
    sheet[64:80, :64] = np.tile(lip, (1, 2, 1))
    sheet[80:96, :64] = np.tile(lip, (1, 2, 1))
    sheet[80:96, :64][sheet[80:96, :64, 3] == 0] = (*ROCK[2], 255)
    Image.fromarray(sheet, "RGBA").resize((64 * 8, 96 * 8), Image.NEAREST).save(out)
    print(out)
