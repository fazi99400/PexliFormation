#!/usr/bin/env python3
"""Generate PexliFormation app icons with no third-party deps.

Draws a diagonal rust->blue gradient rounded square with a white "P" mark and
writes PNGs plus a PNG-embedded .ico. Replace with real artwork any time by
running `npm run tauri icon <your-1024.png>`.
"""
import struct, zlib, os

OUT = os.path.join(os.path.dirname(__file__), "..", "src-tauri", "icons")
RUST = (0xE0, 0x6A, 0x3B)
BLUE = (0x4C, 0x8B, 0xF5)


def lerp(a, b, t):
    return int(a + (b - a) * t)


def in_p(x, y, n):
    # Normalised coords 0..1; return True if pixel is part of the "P" glyph.
    fx, fy = x / n, y / n
    def box(x0, x1, y0, y1):
        return x0 <= fx <= x1 and y0 <= fy <= y1
    stem = box(0.32, 0.42, 0.24, 0.76)
    top = box(0.32, 0.63, 0.24, 0.33)
    right = box(0.58, 0.68, 0.24, 0.52)
    bot = box(0.32, 0.63, 0.44, 0.53)
    return stem or top or right or bot


def make_png_bytes(n):
    rows = bytearray()
    r = int(n * 0.18)  # corner radius
    for y in range(n):
        rows.append(0)  # filter type 0
        for x in range(n):
            # rounded-corner mask
            inside = True
            for cx, cy in ((r, r), (n - r, r), (r, n - r), (n - r, n - r)):
                if (x < r or x > n - r) and (y < r or y > n - r):
                    if (x - cx) ** 2 + (y - cy) ** 2 > r * r:
                        inside = False
            if not inside:
                rows += bytes((0, 0, 0, 0))
                continue
            t = (x + y) / (2 * n)
            col = (lerp(RUST[0], BLUE[0], t), lerp(RUST[1], BLUE[1], t), lerp(RUST[2], BLUE[2], t))
            if in_p(x, y, n):
                col = (0xFF, 0xFF, 0xFF)
            rows += bytes((col[0], col[1], col[2], 0xFF))
    return _png(n, n, bytes(rows))


def _chunk(typ, data):
    c = typ + data
    return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)


def _png(w, h, raw):
    sig = b"\x89PNG\r\n\x1a\n"
    ihdr = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)
    return sig + _chunk(b"IHDR", ihdr) + _chunk(b"IDAT", zlib.compress(raw, 9)) + _chunk(b"IEND", b"")


def make_ico(png_256):
    # ICO header + single PNG-compressed 256x256 entry.
    header = struct.pack("<HHH", 0, 1, 1)
    entry = struct.pack("<BBBBHHII", 0, 0, 0, 0, 1, 32, len(png_256), 22)
    return header + entry + png_256


def main():
    os.makedirs(OUT, exist_ok=True)
    sizes = {"32x32.png": 32, "128x128.png": 128, "128x128@2x.png": 256, "icon.png": 512}
    pngs = {}
    for name, n in sizes.items():
        data = make_png_bytes(n)
        pngs[n] = data
        with open(os.path.join(OUT, name), "wb") as f:
            f.write(data)
    with open(os.path.join(OUT, "icon.ico"), "wb") as f:
        f.write(make_ico(pngs[256]))
    print("icons written to", os.path.normpath(OUT))


if __name__ == "__main__":
    main()
