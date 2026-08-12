#!/usr/bin/env python3
"""Draw the WipTracker icon: a small cat reading a book.

The icon is generated rather than hand-drawn so it can be regenerated at any size and
kept in step with the app's palette. It writes two files:

  assets/icon.png   — 512x512, for the macOS bundle and the docs
  assets/icon.rgba  — 64x64 raw RGBA, which the app embeds and hands to the window
                      manager without needing an image decoder at runtime

Usage: packaging/make_icon.py
"""

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "assets"

# The app's palette, from src/theme.rs.
BACKGROUND = (0x1C, 0x20, 0x28, 255)
BORDER = (0x55, 0x5F, 0x70, 255)
CAT = (0xE6, 0xE6, 0xE6, 255)
CAT_SHADE = (0xB9, 0xC0, 0xCC, 255)
PAGE = (0xF3, 0xF1, 0xE8, 255)
PAGE_EDGE = (0xC9, 0xC4, 0xB4, 255)
AMBER = (0xE0, 0xB0, 0x5A, 255)
INK = (0x1C, 0x20, 0x28, 255)

SIZE = 1024  # drawn large, downsampled at the end
S = SIZE / 512.0  # everything below is expressed in 512-space


def px(*values: float) -> tuple:
    """Scale 512-space coordinates to the drawing canvas."""
    return tuple(value * S for value in values)


def draw_icon() -> Image.Image:
    image = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)

    # Rounded plate in the app's own dark slate, so the icon reads as the bar.
    draw.rounded_rectangle(
        px(16, 16, 496, 496), radius=96 * S, fill=BACKGROUND, outline=BORDER, width=int(6 * S)
    )

    # --- the cat ------------------------------------------------------------------
    # Every shape carries a background-coloured outline: at 32 pixels the pale cat and the
    # pale pages otherwise merge into one blob.
    gap = int(10 * S)

    # Body: a soft trapezoid sitting behind the book.
    draw.rounded_rectangle(
        px(150, 250, 362, 430), radius=70 * S, fill=CAT, outline=BACKGROUND, width=gap
    )

    # Ears first, so the head laps over their base.
    draw.polygon(px(176, 214, 208, 130, 246, 206), fill=CAT)
    draw.polygon(px(336, 214, 304, 130, 266, 206), fill=CAT)
    draw.polygon(px(192, 208, 210, 160, 234, 204), fill=AMBER)
    draw.polygon(px(320, 208, 302, 160, 278, 204), fill=AMBER)

    # Head.
    draw.ellipse(px(156, 164, 356, 334), fill=CAT, outline=BACKGROUND, width=gap)

    # Tail, curling out to the right.
    draw.arc(px(310, 296, 462, 442), start=270, end=80, fill=CAT_SHADE, width=int(24 * S))

    # Eyes: closed, tilted down at the book — arcs rather than dots, which is what carries
    # the "reading" read once the icon is small.
    draw.arc(px(190, 226, 250, 282), start=200, end=340, fill=INK, width=int(14 * S))
    draw.arc(px(262, 226, 322, 282), start=200, end=340, fill=INK, width=int(14 * S))

    # Nose and mouth. No whiskers: at 32 pixels they are noise across the face.
    draw.polygon(px(242, 282, 270, 282, 256, 300), fill=AMBER)
    draw.arc(px(228, 292, 256, 314), start=0, end=140, fill=INK, width=int(8 * S))
    draw.arc(px(256, 292, 284, 314), start=40, end=180, fill=INK, width=int(8 * S))

    # --- the book -----------------------------------------------------------------
    # Two pages meeting at a spine, tilted as if held up to be read.
    left_page = px(84, 394, 256, 350, 256, 474, 84, 466)
    right_page = px(256, 350, 428, 394, 428, 466, 256, 474)
    for page in (left_page, right_page):
        draw.polygon(page, fill=PAGE, outline=BACKGROUND, width=gap)
    draw.line(px(256, 352, 256, 472), fill=PAGE_EDGE, width=int(8 * S))

    # Two lines of text per page — three were mush once downsampled.
    for index in range(2):
        offset = index * 34
        draw.line(px(122, 412 + offset, 226, 396 + offset), fill=PAGE_EDGE, width=int(9 * S))
        draw.line(px(286, 396 + offset, 390, 412 + offset), fill=PAGE_EDGE, width=int(9 * S))

    # Paws holding the covers.
    draw.ellipse(px(92, 374, 152, 428), fill=CAT, outline=BACKGROUND, width=int(6 * S))
    draw.ellipse(px(360, 374, 420, 428), fill=CAT, outline=BACKGROUND, width=int(6 * S))

    return image


def main() -> None:
    ASSETS.mkdir(exist_ok=True)
    icon = draw_icon()

    png = icon.resize((512, 512), Image.LANCZOS)
    png.save(ASSETS / "icon.png")

    small = icon.resize((64, 64), Image.LANCZOS)
    (ASSETS / "icon.rgba").write_bytes(small.tobytes())

    print(f"wrote {ASSETS / 'icon.png'} and {ASSETS / 'icon.rgba'}")


if __name__ == "__main__":
    main()
