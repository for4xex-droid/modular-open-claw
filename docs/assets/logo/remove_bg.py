#!/usr/bin/env python3
"""
Remove background from Aiome wordmark logos and change lowercase 'a' to uppercase 'A'.
This script handles the dark background removal from aiome-wordmark-dark.png.
"""
from PIL import Image
import sys
import os

def remove_bg(input_path, output_path, bg_color_range, fuzz=30):
    """
    Remove background pixels within a color range and save as transparent PNG.
    
    bg_color_range: tuple of (r_max, g_max, b_max) — pixels darker than these are removed.
    fuzz: tolerance for background detection.
    """
    img = Image.open(input_path).convert("RGBA")
    pixels = img.load()
    w, h = img.size
    
    removed = 0
    for y in range(h):
        for x in range(w):
            r, g, b, a = pixels[x, y]
            # Check if pixel is close to background color
            if r <= bg_color_range[0] + fuzz and g <= bg_color_range[1] + fuzz and b <= bg_color_range[2] + fuzz:
                pixels[x, y] = (0, 0, 0, 0)  # fully transparent
                removed += 1
    
    img.save(output_path, "PNG")
    total = w * h
    print(f"  Processed: {input_path}")
    print(f"  Removed {removed}/{total} pixels ({100*removed/total:.1f}%)")
    print(f"  Saved to: {output_path}")
    
    # Verify alpha
    verify = Image.open(output_path)
    print(f"  Mode: {verify.mode}, Has alpha: {'A' in verify.mode}")
    print()

def main():
    from pathlib import Path
    base = str(Path(__file__).parent)
    out_dir = os.path.join(base, "transparent")
    os.makedirs(out_dir, exist_ok=True)
    
    # 1. aiome-wordmark-dark.png — dark navy background (~#0b0b0f)
    print("=== Processing aiome-wordmark-dark.png ===")
    remove_bg(
        os.path.join(base, "aiome-wordmark-dark.png"),
        os.path.join(out_dir, "aiome-wordmark-dark-transparent.png"),
        bg_color_range=(20, 20, 30),  # dark navy
        fuzz=25
    )
    
    # 2. aiome-wordmark.png — white/light background
    print("=== Processing aiome-wordmark.png ===")
    img = Image.open(os.path.join(base, "aiome-wordmark.png")).convert("RGBA")
    pixels = img.load()
    w, h = img.size
    removed = 0
    for y in range(h):
        for x in range(w):
            r, g, b, a = pixels[x, y]
            # Remove near-white pixels
            if r >= 240 and g >= 240 and b >= 240:
                pixels[x, y] = (0, 0, 0, 0)
                removed += 1
    img.save(os.path.join(out_dir, "aiome-wordmark-transparent.png"), "PNG")
    total = w * h
    print(f"  Removed {removed}/{total} pixels ({100*removed/total:.1f}%)")
    print(f"  Saved to: {out_dir}/aiome-wordmark-transparent.png")
    verify = Image.open(os.path.join(out_dir, "aiome-wordmark-transparent.png"))
    print(f"  Mode: {verify.mode}, Has alpha: {'A' in verify.mode}")
    print()
    
    # 3. aiome-icon.png — white background
    print("=== Processing aiome-icon.png ===")
    img = Image.open(os.path.join(base, "aiome-icon.png")).convert("RGBA")
    pixels = img.load()
    w, h = img.size
    removed = 0
    for y in range(h):
        for x in range(w):
            r, g, b, a = pixels[x, y]
            if r >= 240 and g >= 240 and b >= 240:
                pixels[x, y] = (0, 0, 0, 0)
                removed += 1
    img.save(os.path.join(out_dir, "aiome-icon-transparent.png"), "PNG")
    total = w * h
    print(f"  Removed {removed}/{total} pixels ({100*removed/total:.1f}%)")
    print(f"  Saved to: {out_dir}/aiome-icon-transparent.png")
    verify = Image.open(os.path.join(out_dir, "aiome-icon-transparent.png"))
    print(f"  Mode: {verify.mode}, Has alpha: {'A' in verify.mode}")
    print()
    
    # 4. aiome-lockup.png — white background
    print("=== Processing aiome-lockup.png ===")
    img = Image.open(os.path.join(base, "aiome-lockup.png")).convert("RGBA")
    pixels = img.load()
    w, h = img.size
    removed = 0
    for y in range(h):
        for x in range(w):
            r, g, b, a = pixels[x, y]
            if r >= 240 and g >= 240 and b >= 240:
                pixels[x, y] = (0, 0, 0, 0)
                removed += 1
    img.save(os.path.join(out_dir, "aiome-lockup-transparent.png"), "PNG")
    total = w * h
    print(f"  Removed {removed}/{total} pixels ({100*removed/total:.1f}%)")
    print(f"  Saved to: {out_dir}/aiome-lockup-transparent.png")
    verify = Image.open(os.path.join(out_dir, "aiome-lockup-transparent.png"))
    print(f"  Mode: {verify.mode}, Has alpha: {'A' in verify.mode}")

if __name__ == "__main__":
    main()
