#!/usr/bin/env python3
"""Writes the test and demo images in static/img/.

Run from the repository root: python3 scripts/make_fixture_images.py
It uses only the standard library. The output is deterministic.
"""

import json
import math
import struct
import zlib
from pathlib import Path

OUT = Path("static/img")


def png(path, width, height, pixel):
    """Writes an RGBA PNG. `pixel(x, y)` returns (r, g, b, a)."""
    rows = bytearray()
    for y in range(height):
        rows.append(0)  # Filter type: none.
        for x in range(width):
            rows.extend(pixel(x, y))

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    data = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(bytes(rows), 9))
        + chunk(b"IEND", b"")
    )
    path.write_bytes(data)


def sheet(name, frame_size, colors_or_painters, animation):
    """Writes `name.png` and `name.json` (a PixiJS sprite sheet, one row)."""
    count = len(colors_or_painters)
    width = frame_size * count

    def pixel(x, y):
        frame = x // frame_size
        paint = colors_or_painters[frame]
        return paint(x % frame_size, y) if callable(paint) else paint

    png(OUT / f"{name}.png", width, frame_size, pixel)
    frames = {
        f"{animation}-{i}": {
            "frame": {"x": i * frame_size, "y": 0, "w": frame_size, "h": frame_size},
            "sourceSize": {"w": frame_size, "h": frame_size},
            "spriteSourceSize": {"x": 0, "y": 0, "w": frame_size, "h": frame_size},
        }
        for i in range(count)
    }
    doc = {
        "frames": frames,
        "animations": {animation: list(frames)},
        "meta": {"image": f"{name}.png", "format": "RGBA8888", "size": {"w": width, "h": frame_size}, "scale": "1"},
    }
    (OUT / f"{name}.json").write_text(json.dumps(doc, indent=2) + "\n")


def disc(color, size, radius):
    """A painter for a filled disc with a transparent outside."""
    c = (size - 1) / 2

    def paint(x, y):
        inside = (x - c) ** 2 + (y - c) ** 2 <= radius**2
        return (*color, 255) if inside else (0, 0, 0, 0)

    return paint


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    # E2E fixtures: solid colors that pixel tests can read.
    png(OUT / "red.png", 64, 64, lambda x, y: (255, 0, 0, 255))
    png(OUT / "checker.png", 32, 32, lambda x, y: (0, 255, 0, 255) if (x < 16) == (y < 16) else (0, 0, 255, 255))
    sheet("blink", 64, [(255, 0, 0, 255), (0, 255, 0, 255)], "blink")
    (OUT / "broken.png").write_bytes(b"\x89PNG\r\n\x1a\nnot really a png")
    (OUT / "dot.svg").write_text(
        '<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">'
        '<rect width="64" height="64" fill="#ff00ff"/></svg>\n'
    )

    # Demo art.
    gold = (255, 196, 0)
    png(OUT / "coin.png", 64, 64, disc(gold, 64, 30))

    def stars(x, y):
        h = (x * 73856093 ^ y * 19349663) & 0xFFFF
        return (255, 255, 255, 255) if h % 97 == 0 else (15, 23, 42, 255)

    png(OUT / "stars.png", 128, 128, stars)
    frames = []
    for i in range(6):
        radius = 18 + 10 * math.sin(i / 6 * 2 * math.pi)
        frames.append(disc((56, 189, 248), 64, radius))
    sheet("pulse", 64, frames, "pulse")


if __name__ == "__main__":
    main()
