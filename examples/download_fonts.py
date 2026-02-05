#!/usr/bin/env python3
"""Download fonts required for examples."""

import os
import re
import zipfile
import fnmatch
from pathlib import Path
from urllib.request import urlopen, urlretrieve, Request
from tempfile import TemporaryDirectory

FONTS_DIR = Path(__file__).parent / "fonts"

# (url, pattern) - pattern to glob from zip, or None for direct download
FONTS = [
    # LXGW WenKai
    ("https://github.com/lxgw/LxgwWenKai/releases/download/v1.521/LXGWWenKai-Light.ttf", None),
    ("https://github.com/lxgw/LxgwWenKai/releases/download/v1.521/LXGWWenKai-Regular.ttf", None),
    ("https://github.com/lxgw/LxgwWenKai/releases/download/v1.521/LXGWWenKai-Medium.ttf", None),
    # Roboto (static only, not variable font)
    ("https://github.com/googlefonts/roboto-3-classic/releases/download/v3.015/Roboto_v3.015.zip", "*/static/Roboto-*.ttf"),
    # MapleMono NerdFont CN (includes CJK support)
    ("https://github.com/subframe7536/maple-font/releases/download/v7.9/MapleMono-NF-CN-unhinted.zip", "MapleMono-NF-CN-Regular.ttf"),
]

# Google Fonts to download via CSS API (family, weights, output_names)
# weights: list of (italic, weight) tuples
GOOGLE_FONTS = [
    ("Merriweather", [
        (False, 400, "Merriweather-Regular.ttf"),
        (False, 600, "Merriweather-SemiBold.ttf"),
        (False, 700, "Merriweather-Bold.ttf"),
        (True, 400, "Merriweather-Italic.ttf"),
    ]),
]


def download_google_font(family: str, italic: bool, weight: int, output_name: str):
    """Download a font from Google Fonts via CSS API."""
    dest = FONTS_DIR / output_name
    if dest.exists():
        print(f"  Already exists: {output_name}")
        return

    ital = 1 if italic else 0
    css_url = f"https://fonts.googleapis.com/css2?family={family.replace(' ', '+')}:ital,wght@{ital},{weight}"

    print(f"  Fetching {output_name}...")
    req = Request(css_url, headers={"User-Agent": "Mozilla/5.0"})
    with urlopen(req) as resp:
        css = resp.read().decode()

    # Extract font URL from CSS
    match = re.search(r'src:\s*url\((https://[^)]+\.ttf)\)', css)
    if not match:
        print(f"  WARNING: Could not find font URL for {output_name}")
        return

    font_url = match.group(1)
    urlretrieve(font_url, dest)
    print(f"  Saved {output_name}")


def extract_from_zip(zip_path: Path, pattern: str, dest_dir: Path):
    """Extract files matching pattern from zip, flatten to dest_dir.

    Pattern can include path components (e.g., '*/static/*.ttf').
    """
    with zipfile.ZipFile(zip_path) as zf:
        for name in zf.namelist():
            basename = os.path.basename(name)
            if not basename:
                continue
            # Match against full path or just basename
            if fnmatch.fnmatch(name, pattern) or fnmatch.fnmatch(basename, pattern):
                dest = dest_dir / basename
                if not dest.exists():
                    print(f"  Extracting {basename}")
                    with zf.open(name) as src, open(dest, "wb") as dst:
                        dst.write(src.read())


def download_font(url: str, pattern: str | None):
    """Download font from url. If pattern is set, extract matching files from zip."""
    filename = url.split("/")[-1].split("?")[0]
    if not filename.endswith((".ttf", ".otf", ".zip")):
        filename = "font.zip"

    print(f"Downloading {url[:80]}...")

    if pattern:
        # Download zip and extract
        with TemporaryDirectory() as tmpdir:
            zip_path = Path(tmpdir) / filename
            urlretrieve(url, zip_path)
            extract_from_zip(zip_path, pattern, FONTS_DIR)
    else:
        # Direct download
        dest = FONTS_DIR / filename
        if not dest.exists():
            urlretrieve(url, dest)
            print(f"  Saved {filename}")
        else:
            print(f"  Already exists: {filename}")


def main():
    FONTS_DIR.mkdir(exist_ok=True)

    for url, pattern in FONTS:
        download_font(url, pattern)

    for family, variants in GOOGLE_FONTS:
        print(f"Downloading {family} from Google Fonts...")
        for italic, weight, output_name in variants:
            download_google_font(family, italic, weight, output_name)

    print(f"\nDone! Fonts are in: {FONTS_DIR}")


if __name__ == "__main__":
    main()
