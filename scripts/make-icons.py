#!/usr/bin/env python3
"""Draws the app icon (airfoil inside an instrument ring) as PNG, no PIL needed.

Usage: python3 scripts/make-icons.py   -> pwa/icon-180.png, icon-192.png, icon-512.png
"""
import math
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "pwa"

BG_TOP = (16, 28, 52)
BG_BOT = (5, 9, 18)
CYAN = (56, 208, 255)
GREEN = (61, 220, 132)
WHITE = (236, 243, 252)


def airfoil_half_thickness(x, t=0.17):
    return 5 * t * (0.2969 * math.sqrt(max(x, 0)) - 0.1260 * x - 0.3516 * x**2 + 0.2843 * x**3 - 0.1015 * x**4)


def camber(x, m=0.05, p=0.4):
    return m / p**2 * (2 * p * x - x * x) if x < p else m / (1 - p) ** 2 * ((1 - 2 * p) + 2 * p * x - x * x)


def color_at(u, v):
    """u, v in [0, 1] -> (r, g, b, inside_alpha)"""
    # background gradient
    k = v
    bg = tuple(BG_TOP[i] * (1 - k) + BG_BOT[i] * k for i in range(3))
    cx, cy = u - 0.5, v - 0.5
    r = math.hypot(cx, cy)
    ang = (math.degrees(math.atan2(cx, -cy)) + 360) % 360  # 0 = top, clockwise

    # ring: green arc (progress) over a dim track
    if 0.335 < r < 0.375:
        return GREEN if ang < 250 else tuple(c * 0.35 + b * 0.65 for c, b in zip(CYAN, bg))
    # tick marks
    if 0.395 < r < 0.425:
        if (ang % 30) < 2.2:
            return tuple(c * 0.55 + b * 0.45 for c, b in zip(CYAN, bg))

    # airfoil, chord 0.52, rotated by a small angle of attack
    a = math.radians(-8)
    x0, y0 = cx * math.cos(a) - cy * math.sin(a), cx * math.sin(a) + cy * math.cos(a)
    xc = (x0 + 0.26) / 0.52
    if 0 <= xc <= 1:
        yc = -camber(xc) * 0.52
        yt = airfoil_half_thickness(xc) * 0.52
        if yc - yt <= y0 - 0.025 <= yc + yt:
            return WHITE
    # airflow lines
    for off in (-0.13, 0.15):
        yl = off + 0.025 * math.sin((cx + 0.3) * 9) * (1 if off < 0 else -0.6)
        if abs(cy - yl) < 0.008 and -0.3 < cx < 0.3:
            return tuple(c * 0.6 + b * 0.4 for c, b in zip(CYAN, bg))
    return bg


def render(size, ss=3):
    rows = []
    for y in range(size):
        row = bytearray([0])
        for x in range(size):
            acc = [0.0, 0.0, 0.0]
            for sy in range(ss):
                for sx in range(ss):
                    c = color_at((x + (sx + 0.5) / ss) / size, (y + (sy + 0.5) / ss) / size)
                    for i in range(3):
                        acc[i] += c[i]
            row += bytes(int(a / (ss * ss)) for a in acc)
        rows.append(bytes(row))
    raw = b"".join(rows)

    def chunk(tag, data):
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 2, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


OUT.mkdir(exist_ok=True)
for s in (180, 192, 512):
    (OUT / f"icon-{s}.png").write_bytes(render(s, 3 if s < 300 else 2))
    print("icon", s)
