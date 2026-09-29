#!/usr/bin/env python3
"""Turn the generated sprite sheet and environment sheets into the game's atlas (see specs/assets-sprites.md §0).

    python3 tools/process_sprites.py [--palette] [--preview out.png]

Reads  assets/spritesheet.jpg         (2048², magenta background)
       assets/environment_sky.jpg     (2000², magenta background)
       assets/environment_ground.jpg  (2000², 8×8 grid of 250 px cells)
       assets/overworld.png           (1024², magenta background)
Writes assets/atlas.png               (packed, transparent, 1 logical px = 1 px)
       assets/atlas.ron               (sprite name -> rect + anchor, read by the game)

The generated sheet came out as a 16×16 grid of 128 px cells instead of the
32×32 grid of 64 px cells in specs/assets-sprites.md, and some rows ignore the grid, so
regions are listed by hand below. Tile regions are sliced strictly by cell.
Sprite regions give a search box; the sprite is the non-background pixels found
inside it, so small misalignments don't matter.

assets/overworld.png (see specs/assets-overworld.md) also came out off-grid: a
~10×10 grid of ~102.4 px cells instead of the spec's 16×16 grid of 128 px cells,
and some buildings/heroes bleed a little past their cell into the next. Regions
below were measured by hand from the actual sheet (see OWCELL / owcell()).

Pipeline per sprite:
  1. Chroma-key the magenta (tolerant of JPEG noise) and erode the key by 1 px
     to drop the magenta fringe.
  2. Downscale 4× (128 px cell -> 32 px tile) with alpha-weighted box filtering,
     then threshold alpha so edges stay crisp.
  3. Optionally snap colours to the specs/assets-sprites.md §1 palette (--palette).

Requires Pillow, numpy and scipy.
"""

import argparse
import os
import sys

import numpy as np
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "assets", "spritesheet.jpg")
SRC_SKY = os.path.join(ROOT, "assets", "environment_sky.jpg")
SRC_GROUND = os.path.join(ROOT, "assets", "environment_ground.jpg")
SRC_GUI = os.path.join(ROOT, "assets", "gui.jpg")
SRC_OW = os.path.join(ROOT, "assets", "overworld.png")
OUT_PNG = os.path.join(ROOT, "assets", "atlas.png")
OUT_RON = os.path.join(ROOT, "assets", "atlas.ron")

CELL = 128  # source px per grid cell
TILE = 32  # logical px per board tile
SCALE = TILE / CELL

OWCELL = 102.4  # overworld.png: ~10x10 grid of 102.4 px cells (1024 / 10)
SCALE_OW = TILE / OWCELL


def cell(r, c, w=1, h=1):
    """Source box of grid cells (0-based row/col)."""
    return (c * CELL, r * CELL, (c + w) * CELL, (r + h) * CELL)


def owcell(r, c, w=1, h=1):
    """Source box of overworld.png grid cells (0-based row/col, 102.4 px pitch)."""
    x0, y0 = round(c * OWCELL), round(r * OWCELL)
    x1, y1 = round((c + w) * OWCELL), round((r + h) * OWCELL)
    return (x0, y0, x1, y1)


def gcell(r, c):
    """Source box of 250 px grid cells (1-based row/col) in environment_ground.jpg."""
    x0 = 250 * (c - 1)
    y0 = 250 * (r - 1)
    return (x0, y0, x0 + 250, y0 + 250)


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

# Pieces: idle frames, pawn smallest to king tallest. Kept shorter than specs/assets-sprites.md §0 so a
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
    ("cave_rune_0", (1215, 410, 1340, 492), None, CENTER),
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
    # Cards, UI panels, and pickups
    ("card_common", (1443, 1682, 1528, 1793), None, CENTER),
    ("card_uncommon", (1541, 1682, 1625, 1793), None, CENTER),
    ("card_rare", (1639, 1682, 1723, 1793), None, CENTER),
    ("panel_parchment", (1029, 1687, 1225, 1900), None, CENTER),
    ("panel_wood", (1237, 1690, 1429, 1900), None, CENTER),
    ("chest", (789, 941, 875, 1013), 20, BOTTOM),
    ("orb", (1548, 866, 1611, 928), 14, CENTER),
    # Visual effects and icons (M7a)
    ("fx_smoke", (9, 1850, 93, 1938), None, CENTER),
    ("fx_splash", (108, 1855, 199, 1939), None, CENTER),
    ("fx_dust", (211, 1885, 301, 1938), None, CENTER),
    ("fx_tornado", (314, 1851, 403, 1938), None, CENTER),
    ("fx_sparkle", (433, 1864, 490, 1924), None, CENTER),
    ("fx_bubble", (519, 1850, 607, 1939), None, CENTER),
    ("fx_dirt", (624, 1851, 709, 1938), None, CENTER),
    ("fx_snowflake", (725, 1851, 812, 1937), None, CENTER),
    ("fx_slash_gold", (824, 1847, 919, 1941), None, CENTER),
    ("fx_slash_violet", (928, 1849, 1018, 1938), None, CENTER),
    ("icon_crown_gold", (7, 1963, 93, 2029), None, CENTER),
    ("icon_crown_violet", (110, 1955, 196, 2031), None, CENTER),
    ("fx_pillar", (829, 1949, 912, 2041), None, CENTER),
]

HORIZON_STRIPS = [
    ("sky_far", (3, 35, 997, 165)),
    ("sky_mid", (3, 520, 997, 750)),
    ("sky_near", (3, 761, 997, 998)),
]

CLOUDS = [
    ("cloud_0", (28, 1006, 472, 1150)),
    ("cloud_1", (564, 1025, 932, 1136)),
    ("cloud_2", (1077, 1009, 1421, 1144)),
    ("cloud_3", (1546, 1008, 1704, 1150)),
    ("cloud_4", (134, 1165, 366, 1244)),
    ("cloud_5", (625, 1177, 876, 1230)),
    ("cloud_6", (1132, 1165, 1370, 1244)),
    ("cloud_7", (1575, 1167, 1673, 1240)),
]

ENV_TILES = [
    ("sea_deep_0", 1, 1),
    ("sea_deep_1", 1, 2),
    ("sea_shallow_0", 1, 3),
    ("sea_shallow_1", 1, 4),
    ("shore_0", 1, 5),
    ("shore_1", 1, 6),
    ("sand_wet", 2, 1),
    ("sand_dry", 2, 4),
    ("sand_shells", 2, 5),
]

ENV_SPRITES = [
    *[(f"decal_{i}", gcell(3, i + 1), None, CENTER) for i in range(8)],
    ("gull_0", gcell(4, 1), 14, CENTER),
    ("gull_1", gcell(4, 2), 14, CENTER),
    ("crow_0", gcell(4, 3), 14, CENTER),
    ("crow_1", gcell(4, 4), 14, CENTER),
    ("sparrow_0", gcell(5, 5), 14, CENTER),
    ("sparrow_1", gcell(5, 6), 14, CENTER),
    ("butterfly_0", gcell(4, 7), 14, CENTER),
    ("butterfly_1", gcell(4, 8), 14, CENTER),
]

GUI_BOXES = [
    # Card frames (row 1)
    ("gui_card_common", (7, 7, 242, 328)),
    ("gui_card_plain", (256, 7, 492, 328)),
    ("gui_card_uncommon", (506, 4, 742, 328)),
    ("gui_card_rare", (755, 4, 993, 328)),
    ("gui_card_spell", (1005, 3, 1242, 328)),
    ("gui_card_back", (1253, 6, 1492, 331)),
    ("gui_card_used", (1504, 5, 1742, 330)),
    ("gui_card_highlight", (1755, 6, 1991, 329)),
    # Card art (each box 242 x 216)
    # row y 337-553:
    ("art_mountaineer_rooks", (4, 337, 246, 553)),
    ("art_amphibious_knights", (254, 337, 496, 553)),
    ("art_surefooted_pawns", (504, 337, 746, 553)),
    ("art_momentum_bishops", (754, 337, 996, 553)),
    ("art_long_jump_knights", (1004, 337, 1246, 553)),
    ("art_daring_queens", (1254, 337, 1496, 553)),
    ("art_tunneler_bishops", (1504, 337, 1746, 553)),
    ("art_veteran", (1754, 337, 1996, 553)),
    # row y 558-774:
    ("art_tectonic_pact", (4, 558, 246, 774)),
    ("art_calm_terrain", (254, 558, 496, 774)),
    ("art_tide_charm", (504, 558, 746, 774)),
    ("art_cartographer", (754, 558, 996, 774)),
    ("art_chest", (1004, 558, 1246, 774)),
    ("art_scroll", (1254, 558, 1496, 774)),
    ("art_swap", (1504, 558, 1746, 774)),
    ("art_rewind", (1754, 558, 1996, 774)),
    # row y 779-994:
    ("art_raise_earth", (4, 779, 246, 994)),
    ("art_lower_earth", (254, 779, 496, 994)),
    ("art_freeze", (504, 779, 746, 994)),
    ("art_bridge", (754, 779, 996, 994)),
    ("art_dig_tunnel", (1004, 779, 1246, 994)),
    ("art_shield", (1254, 779, 1496, 994)),
    # Small icons:
    ("icon_fire", (1504, 779, 1622, 884)),
    ("icon_gem", (1629, 779, 1746, 884)),
    ("icon_bolt", (1754, 779, 1871, 884)),
    ("icon_feather", (1878, 779, 1996, 884)),
    ("icon_skull", (1504, 889, 1622, 996)),
    ("icon_sun", (1629, 889, 1746, 996)),
    ("icon_moon", (1754, 889, 1871, 996)),
    ("icon_star_small", (1878, 889, 1996, 996)),
    ("icon_star", (1014, 1766, 1111, 1863)),
    ("icon_heart", (1141, 1771, 1234, 1858)),
    ("icon_coin", (1267, 1764, 1357, 1862)),
    ("icon_potion", (1400, 1762, 1475, 1865)),
    ("icon_sword", (1512, 1763, 1612, 1865)),
    ("icon_gear", (1638, 1762, 1738, 1864)),
    ("icon_alert", (1796, 1763, 1830, 1863)),
    ("icon_close", (1894, 1768, 1981, 1859)),
    ("icon_leaf", (1028, 1895, 1096, 1982)),
    ("icon_next", (1145, 1895, 1228, 1981)),
    # Buttons (each 242 x 124):
    # row y 1003-1127:
    ("btn_wood", (4, 1003, 246, 1127)),
    ("btn_wood_hover", (254, 1003, 496, 1127)),
    ("btn_wood_pressed", (504, 1003, 746, 1127)),
    ("btn_wood_disabled", (754, 1003, 996, 1127)),
    ("btn_gold", (1004, 1003, 1246, 1127)),
    ("btn_gold_hover", (1254, 1003, 1496, 1127)),
    ("btn_gold_pressed", (1504, 1003, 1746, 1127)),
    ("btn_gold_disabled", (1754, 1003, 1996, 1127)),
    # row y 1137-1261:
    ("btn_stone", (4, 1137, 246, 1261)),
    ("btn_stone_hover", (254, 1137, 496, 1261)),
    ("btn_stone_pressed", (504, 1137, 746, 1261)),
    ("btn_stone_disabled", (754, 1137, 996, 1261)),
    # Round:
    ("btn_round", (1514, 1131, 1652, 1270)),
    ("btn_round_hover", (1682, 1132, 1818, 1270)),
    ("btn_round_disabled", (1849, 1132, 1986, 1270)),
    # Panels:
    ("panel_gui_wood", (3, 1269, 246, 1536)),
    ("panel_gui_parchment", (257, 1275, 493, 1531)),
    ("panel_gui_parchment_curl", (757, 1275, 993, 1532)),
    ("panel_gui_stone", (1004, 1269, 1246, 1536)),
    ("panel_gui_glass", (1254, 1269, 1496, 1535)),
    ("banner", (1505, 1340, 1995, 1473)),
    # Hand bar:
    ("hand_tray", (5, 1557, 494, 1739)),
    ("deck_pile", (1019, 1555, 1152, 1738)),
    ("card_slot_empty", (1381, 1566, 1494, 1728)),
    ("icon_discard", (1480, 1575, 1855, 1752)),
    ("icon_draw", (1867, 1545, 1990, 1752)),
    # Badges:
    ("badge_floor", (25, 1761, 225, 1986)),
    ("badge_boss", (255, 1762, 495, 1984)),
    ("badge_victory", (505, 1768, 745, 1982)),
    ("badge_defeat", (766, 1760, 989, 1977)),
]

# --- Overworld sheet (assets/overworld.png, see specs/assets-overworld.md) ------
# Rows 1-3: seamless terrain tiles, sliced strictly by (row, col) like TILES.
OW_TILES = [
    ("ow_grass_0", 0, 0),
    ("ow_grass_1", 0, 1),
    ("ow_grass_2", 1, 0),
    ("ow_forest_0", 0, 2),
    ("ow_forest_1", 1, 1),
    ("ow_forest_2", 1, 2),
    ("ow_forest_3", 2, 1),
    ("ow_forest_4", 2, 2),
    ("ow_hills_0", 0, 3),
    ("ow_hills_1", 1, 3),
    ("ow_hills_2", 2, 3),
    ("ow_mountain_0", 0, 4),
    ("ow_mountain_1", 0, 5),
    ("ow_mountain_2", 1, 4),
    ("ow_mountain_3", 2, 4),
    ("ow_water_0", 0, 6),
    ("ow_water_1", 1, 5),
    ("ow_water_2", 2, 5),
    ("ow_coast_0", 1, 6),
    ("ow_coast_1", 2, 6),
    # One canonical piece per road/bridge shape; the other rotations (N/E/S/W
    # connections) are made in code with board_view::rotate_uv.
    ("ow_road_corner", 0, 7),  # as drawn: connects south + east
    ("ow_road_cross", 1, 7),  # connects all 4 sides
    ("ow_road_straight", 2, 7),  # as drawn: connects east + west
    ("ow_bridge_ns", 0, 8),
    ("ow_bridge_ew", 2, 8),
    ("ow_river_0", 1, 9),
    ("ow_river_1", 2, 9),
]

# Rows 4-5: camp buildings (2 rows tall for fortress/citadel), each with a small
# brown "flag slot" square near the top-right that's erased and replaced in code
# by the controlling faction's banner. Search boxes are generous; the actual
# sprite is the connected non-magenta blob(s) found inside.
OW_CAMPS = [
    ("ow_fortress", (0, 296, 148, 520), (0.5, 1.0)),
    ("ow_citadel", (98, 296, 312, 520), (0.5, 1.0)),
    ("ow_village", (300, 296, 415, 414), (0.5, 1.0)),
    ("ow_knight_camp", (405, 296, 515, 414), (0.5, 1.0)),
    ("ow_bishop_camp", (505, 296, 615, 414), (0.5, 1.0)),
]

# Same row, no flag overlay needed but the sheet still drew (and this still
# erases) the flag-slot marker for visual consistency with the camps.
OW_PROPS = [
    ("ow_chest_open", (605, 296, 715, 414), (0.5, 1.0)),
    ("ow_chest", (705, 296, 820, 414), (0.5, 1.0)),
    ("ow_shrine", (810, 296, 920, 414), (0.5, 1.0)),
    ("ow_signpost", (910, 296, 1024, 414), (0.5, 1.0)),
]

# Row 5 (cols 2-9): dark fog-of-war tiles. Not used for map rendering (the game
# keeps the existing tint-based fog darkening) but sliced for completeness.
OW_FOG = [(f"ow_fog_{i}", owcell(4, 2 + i)) for i in range(8)]

# Rows 6-7: mounted heroes, 2 rows each faction (idle + walk frames, facing
# right; mirrored in code via card_mesh(..., flip) for left movement) plus 3
# planted banners.
OW_HEROES_SUN = [owcell(5, c) for c in range(3)] + [owcell(6, c) for c in range(4)]
OW_HEROES_CROWN = [owcell(5, c) for c in range(4, 7)] + [owcell(6, c) for c in range(4, 7)]
OW_BANNERS = [
    ("ow_banner_sun", owcell(5, 7)),
    ("ow_banner_crown", owcell(5, 8)),
    ("ow_banner_neutral", owcell(5, 9)),
]

# Row 8: overworld UI and move markers.
OW_UI = [
    ("ow_path", owcell(7, 0)),
    ("ow_target", owcell(7, 1)),
    ("ow_blocked", owcell(7, 2)),
    ("ow_day_frame", owcell(7, 3)),
    ("ow_army_slot", owcell(7, 4)),
    ("ow_portrait_frame", owcell(7, 5)),
    ("ow_portrait_sun", owcell(7, 6)),
    ("ow_deploy_blue", owcell(7, 7)),
    ("ow_deploy_red", owcell(7, 8)),
    ("ow_end_turn", owcell(7, 9)),
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


# specs/assets-sprites.md §1
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


def key_mask(rgb, env=False, gui=False):
    """True where the pixel is foreground (not magenta background)."""
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    if gui:
        bg = (r > 150) & (b > 150) & (g < 120) & (np.abs(r - b) < 80)
    elif env:
        bg = (r > 120) & (b > 120) & (g < 110) & (np.abs(r - b) < 90)
    else:
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


# --- Column walls -----------------------------------------------------------------
# The 3D board textures each column's sides with slices of the cliff art: the grass
# lip under the top edge, then stone repeated down to the base.
WALL_FILL = (78, 85, 99)  # stone behind transparent bits of the cliff art


def wall_sprites(out):
    art = out["cliff_0"][0]  # rows: 0 magenta fringe, 1-6 grass lip, 8-27 stone, 28-29 dark base
    walls = {}
    for name, rows in [("wall_lip", art[1:14]), ("wall_stone", art[8:28])]:
        img = rows.copy()
        img[img[..., 3] == 0, :3] = WALL_FILL
        img[..., 3] = 255
        walls[name] = (img, (0.5, 0.5))
    return walls


def process(src, src_sky, src_ground, src_gui, src_ow, palette):
    rgb_all = np.asarray(src.convert("RGB")).astype(np.float32)
    rgb_sky = np.asarray(src_sky.convert("RGB")).astype(np.float32)
    rgb_ground = np.asarray(src_ground.convert("RGB")).astype(np.float32)
    rgb_gui = np.asarray(src_gui.convert("RGB")).astype(np.float32)
    rgb_ow = np.asarray(src_ow.convert("RGB")).astype(np.float32)
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

    def trimmed(rgb, target_h, env=False):
        mask = key_mask(rgb, env=env)
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

    rim_color = np.array([0xA4, 0x92, 0xC9], dtype=np.float32)
    for name, box, target_h, anchor in SPRITES:
        img = trimmed(crop(box), target_h)
        if name.startswith("black_"):
            alpha = img[..., 3]
            is_opaque = alpha > 0
            sh, sw = alpha.shape
            upper_trans = np.zeros((sh, sw), dtype=bool)
            upper_trans[0, :] = True
            upper_trans[1:, :] = alpha[:-1, :] == 0
            left_trans = np.zeros((sh, sw), dtype=bool)
            left_trans[:, 0] = True
            left_trans[:, 1:] = alpha[:, :-1] == 0
            rim_mask = is_opaque & (upper_trans | left_trans)
            img[rim_mask, :3] = img[rim_mask, :3] * 0.3 + rim_color * 0.7
        out[name] = (img, anchor)
    out["white_king"] = (trimmed(white_king(crop), PIECE_H["king"]), BOTTOM)

    outline_color = np.array([0x12, 0x0E, 0x14], dtype=np.float32)
    for name in list(out.keys()):
        if name.startswith(("white_", "black_")):
            img, _ = out[name]
            h, w = img.shape[:2]
            padded = np.zeros((h + 2, w + 2, 4), dtype=np.float32)
            padded[1:-1, 1:-1] = img
            is_opaque = padded[..., 3] > 0
            is_transparent = ~is_opaque
            touches_opaque = np.zeros((h + 2, w + 2), dtype=bool)
            touches_opaque[:-1, :] |= is_opaque[1:, :]
            touches_opaque[1:, :] |= is_opaque[:-1, :]
            touches_opaque[:, :-1] |= is_opaque[:, 1:]
            touches_opaque[:, 1:] |= is_opaque[:, :-1]
            outline_mask = is_transparent & touches_opaque
            padded[outline_mask, :3] = outline_color
            padded[outline_mask, 3] = 255.0
            out[name] = (padded, BOTTOM)

    # Generated sprite: board_stone (compressed contrast, lifted mortar, desaturated)
    flag = out["flagstone"][0].copy()
    flag_rgb = flag[..., :3]
    mean = flag_rgb.mean(axis=(0, 1), keepdims=True)
    comp = mean + (flag_rgb - mean) * (1.0 - 0.65)
    floor_val = mean * (1.0 - 0.15)
    comp = np.maximum(comp, floor_val)
    lum = 0.299 * comp[..., 0:1] + 0.587 * comp[..., 1:2] + 0.114 * comp[..., 2:3]
    desat = comp * 0.70 + lum * 0.30
    flag[..., :3] = desat
    out["board_stone"] = (flag, CENTER)

    # Generated sprite: shadow_ellipse (32x16, black, 0.45 alpha inside, 1px dithered edge)
    sw, sh = 32, 16
    cx, cy = (sw - 1) / 2.0, (sh - 1) / 2.0
    a, b = 15.9, 7.9
    y, x = np.mgrid[:sh, :sw]
    outer = ((x - cx) / a) ** 2 + ((y - cy) / b) ** 2 <= 1.0
    inner = ((x - cx) / (a - 1.0)) ** 2 + ((y - cy) / (b - 1.0)) ** 2 <= 1.0
    ring = outer & ~inner
    checker = (x + y) % 2 == 0
    shadow = np.zeros((sh, sw, 4), dtype=np.float32)
    shadow[inner, 3] = 0.45 * 255.0
    shadow[ring & checker, 3] = 0.45 * 255.0
    out["shadow_ellipse"] = (shadow, CENTER)

    # Environment sky: horizon strips
    for name, (x0, y0, x1, y1) in HORIZON_STRIPS:
        rgb = rgb_sky[y0:y1, x0:x1]
        mask = key_mask(rgb, env=True)
        h, w = mask.shape
        out[name] = (
            downscale(rgb, mask.astype(np.float32), max(1, round(w * SCALE)), max(1, round(h * SCALE))),
            CENTER,
        )

    # Environment sky: clouds
    for name, (x0, y0, x1, y1) in CLOUDS:
        out[name] = (trimmed(rgb_sky[y0:y1, x0:x1], None, env=True), CENTER)

    # Environment ground: tiles
    for name, r, c in ENV_TILES:
        x0, y0, x1, y1 = gcell(r, c)
        rgb = rgb_ground[y0 + 10 : y1 - 10, x0 + 10 : x1 - 10]
        out[name] = (downscale(rgb, np.ones(rgb.shape[:2], np.float32), TILE, TILE), CENTER)

    # Environment ground: sprites
    for name, box, target_h, anchor in ENV_SPRITES:
        x0, y0, x1, y1 = box
        out[name] = (trimmed(rgb_ground[y0:y1, x0:x1], target_h, env=True), anchor)

    # GUI sprites
    for name, (x0, y0, x1, y1) in GUI_BOXES:
        rgb = rgb_gui[y0:y1, x0:x1]
        mask = key_mask(rgb, gui=True)
        h, w = mask.shape
        out[name] = (
            downscale(rgb, mask.astype(np.float32), max(1, round(w * SCALE)), max(1, round(h * SCALE))),
            CENTER,
        )

    # --- Overworld sheet -----------------------------------------------------

    def ow_trimmed(box, target_h=None, min_area=40):
        """Like `trimmed`, but for overworld.png (scaled by SCALE_OW, not SCALE)."""
        x0, y0, x1, y1 = box
        rgb = rgb_ow[y0:y1, x0:x1]
        mask = key_mask(rgb)
        boxes = components(mask, min_area)
        if not boxes:
            raise SystemExit(f"empty overworld sprite region {box}")
        area = lambda b: (b[2] - b[0]) * (b[3] - b[1])
        biggest = max(area(b) for b in boxes)
        boxes = [b for b in boxes if area(b) >= 0.08 * biggest]
        bx0 = min(b[0] for b in boxes)
        by0 = min(b[1] for b in boxes)
        bx1 = max(b[2] for b in boxes)
        by1 = max(b[3] for b in boxes)
        rgb, mask = rgb[by0:by1, bx0:bx1], mask[by0:by1, bx0:bx1]
        return rgb, mask

    def ow_erase_flag(rgb, mask):
        """Camp/prop cells have a small brown flag-slot square near the top-right,
        left empty for the code to overlay the controlling faction's banner.
        Erases it (drops it from the alpha mask) in place."""
        h, w = mask.shape
        r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
        from scipy import ndimage

        brownish = (r > 120) & (r < 210) & (g > 80) & (g < 170) & (b > 40) & (b < 140) & (r > g) & (g > b + 5)
        region = np.zeros((h, w), dtype=bool)
        region[: round(h * 0.45), round(w * 0.35) :] = True
        candidate = brownish & mask & region
        labels, n = ndimage.label(candidate, structure=np.ones((3, 3)))
        best = None
        for i, sl in enumerate(ndimage.find_objects(labels), start=1):
            if sl is None:
                continue
            blob_area = int((labels[sl] == i).sum())
            bw, bh = sl[1].stop - sl[1].start, sl[0].stop - sl[0].start
            if 120 <= blob_area <= 2500 and bh > 0 and 0.4 <= bw / bh <= 2.5:
                if best is None or blob_area > best[0]:
                    best = (blob_area, sl)
        if best:
            mask[best[1]] = False

    def ow_downscale(rgb, mask, target_h=None):
        h, w = mask.shape
        s = target_h / h if target_h else SCALE_OW
        return downscale(rgb, mask.astype(np.float32), max(1, round(w * s)), max(1, round(h * s)))

    # Rows 1-3: seamless terrain tiles, sliced strictly by cell (no trimming).
    for name, r, c in OW_TILES:
        x0, y0, x1, y1 = owcell(r, c)
        rgb = rgb_ow[y0 + 3 : y1 - 3, x0 + 3 : x1 - 3]  # skip the thin grid line
        out[name] = (downscale(rgb, np.ones(rgb.shape[:2], np.float32), TILE, TILE), CENTER)

    # Rows 4-5: camp buildings, flag slot erased and left transparent; the
    # controlling faction's banner is overlaid in code near the top-right.
    for name, box, anchor in OW_CAMPS:
        rgb, mask = ow_trimmed(box)
        ow_erase_flag(rgb, mask)
        out[name] = (ow_downscale(rgb, mask), anchor)

    # Same row: props with the same flag-slot artifact erased, but unused.
    for name, box, anchor in OW_PROPS:
        rgb, mask = ow_trimmed(box)
        ow_erase_flag(rgb, mask)
        out[name] = (ow_downscale(rgb, mask), anchor)

    # Row 5 (cols 2-9): fog-of-war tiles (unused by map rendering today, which
    # keeps the existing tint-based fog darkening; sliced for completeness).
    for name, box in OW_FOG:
        x0, y0, x1, y1 = box
        rgb = rgb_ow[y0 + 2 : y1 - 2, x0 + 2 : x1 - 2]
        out[name] = (downscale(rgb, np.ones(rgb.shape[:2], np.float32), TILE, TILE), CENTER)

    # Rows 6-7: mounted heroes, facing right (mirrored in code for left).
    hero_h = PIECE_H["king"]
    for i, box in enumerate(OW_HEROES_SUN):
        rgb, mask = ow_trimmed(box)
        out[f"ow_hero_sun_{i}"] = (ow_downscale(rgb, mask, hero_h), BOTTOM)
    for i, box in enumerate(OW_HEROES_CROWN):
        rgb, mask = ow_trimmed(box)
        out[f"ow_hero_crown_{i}"] = (ow_downscale(rgb, mask, hero_h), BOTTOM)
    for name, box in OW_BANNERS:
        rgb, mask = ow_trimmed(box)
        out[name] = (ow_downscale(rgb, mask), BOTTOM)

    # Row 8: overworld UI and move markers.
    for name, box in OW_UI:
        rgb, mask = ow_trimmed(box)
        out[name] = (ow_downscale(rgb, mask), CENTER)

    out.update(wall_sprites(out))

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


def write_ron(rects, size, path):
    lines = ["// Generated by tools/process_sprites.py. Do not edit.", "(", f"    size: ({size[0]}, {size[1]}),", "    sprites: {"]
    for name, (x, y, w, h, (ax, ay)) in rects.items():
        lines.append(
            f'        "{name}": (x: {x}, y: {y}, w: {w}, h: {h}, anchor: ({ax:.2f}, {ay:.2f})),'
        )
    lines += ["    },", ")", ""]
    with open(path, "w") as f:
        f.write("\n".join(lines))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--palette", action="store_true", help="snap colours to the specs/assets-sprites.md palette")
    ap.add_argument("--preview", help="also write a 4× preview of the atlas here")
    args = ap.parse_args()

    sprites = process(
        Image.open(SRC),
        Image.open(SRC_SKY),
        Image.open(SRC_GROUND),
        Image.open(SRC_GUI),
        Image.open(SRC_OW),
        args.palette,
    )
    atlas, rects = pack(sprites, width=512)
    if atlas.height > 2048:
        atlas, rects = pack(sprites, width=1024)
    atlas.save(OUT_PNG)
    write_ron(rects, atlas.size, OUT_RON)
    if args.preview:
        big = atlas.resize((atlas.width * 4, atlas.height * 4), Image.NEAREST)
        bg = Image.new("RGBA", big.size, (40, 44, 60, 255))
        Image.alpha_composite(bg, big).save(args.preview)
    print(f"{len(rects)} sprites -> {OUT_PNG} ({atlas.width}×{atlas.height}), {OUT_RON}")


if __name__ == "__main__":
    main()
