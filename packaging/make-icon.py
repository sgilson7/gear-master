#!/usr/bin/env python3
"""Generate packaging/AppIcon.icns from scratch — no image libraries needed.

Draws the game's own motif: a slot grid with one assembled item outlined in
gold. Writes a 1024px PNG by hand (zlib + struct), then lets macOS's `sips`
and `iconutil` produce the .icns.
"""
import os
import shutil
import struct
import subprocess
import sys
import zlib

SIZE = 1024
BG = (22, 22, 34, 255)          # app background
CELL_EMPTY = (44, 44, 60, 255)  # unfilled grid cell
GOLD = (240, 200, 90, 255)      # assembled outline
FILLS = [                       # slot hues, matching the game
    (196, 84, 62, 255),   # weapon
    (196, 84, 62, 255),
    (86, 166, 104, 255),  # chest
    (120, 96, 190, 255),  # gloves
    (86, 166, 104, 255),
    (196, 84, 62, 255),
]


def write_png(path, w, h, rows):
    raw = b"".join(b"\x00" + bytes(v for px in row for v in px) for row in rows)

    def chunk(tag, data):
        body = tag + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    header = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)  # 8-bit RGBA
    blob = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )
    with open(path, "wb") as f:
        f.write(blob)


def rounded(x, y, size, radius):
    """Is (x, y) inside a rounded square of `size` with corner `radius`?"""
    cx = min(max(x, radius), size - radius)
    cy = min(max(y, radius), size - radius)
    return (x - cx) ** 2 + (y - cy) ** 2 <= radius * radius


def draw():
    # A 3-wide by 2-tall block of cells, centred, with a gold outline around it.
    cols, rows_n = 3, 2
    margin = 190
    grid_w = SIZE - margin * 2
    cell = grid_w // cols
    grid_h = cell * rows_n
    ox = (SIZE - grid_w) // 2
    oy = (SIZE - grid_h) // 2
    border = 16
    inset = 22

    pixels = []
    for y in range(SIZE):
        row = []
        for x in range(SIZE):
            if not rounded(x, y, SIZE, 210):
                row.append((0, 0, 0, 0))  # transparent outside the squircle
                continue

            px = BG
            gx, gy = x - ox, y - oy
            if 0 <= gx < grid_w and 0 <= gy < grid_h:
                col, rw = gx // cell, gy // cell
                inx, iny = gx % cell, gy % cell
                on_edge = (
                    inx < border and col == 0
                    or inx >= cell - border and col == cols - 1
                    or iny < border and rw == 0
                    or iny >= cell - border and rw == rows_n - 1
                )
                if on_edge:
                    px = GOLD
                elif inset <= inx < cell - inset and inset <= iny < cell - inset:
                    px = FILLS[(rw * cols + col) % len(FILLS)]
                else:
                    px = CELL_EMPTY
            row.append(px)
        pixels.append(row)
    return pixels


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    master = os.path.join(here, "icon-1024.png")
    iconset = os.path.join(here, "AppIcon.iconset")
    icns = os.path.join(here, "AppIcon.icns")

    print("drawing 1024px master...")
    write_png(master, SIZE, SIZE, draw())

    if not shutil.which("sips") or not shutil.which("iconutil"):
        print("sips/iconutil not found — leaving the PNG only", file=sys.stderr)
        return 0

    shutil.rmtree(iconset, ignore_errors=True)
    os.makedirs(iconset)
    for size in (16, 32, 128, 256, 512):
        for scale, suffix in ((1, ""), (2, "@2x")):
            px = size * scale
            out = os.path.join(iconset, f"icon_{size}x{size}{suffix}.png")
            subprocess.run(
                ["sips", "-z", str(px), str(px), master, "--out", out],
                check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            )
    subprocess.run(["iconutil", "-c", "icns", iconset, "-o", icns], check=True)
    shutil.rmtree(iconset, ignore_errors=True)
    os.remove(master)
    print(f"wrote {icns}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
