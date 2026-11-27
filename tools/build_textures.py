"""Builds the Image Overlay texture pack (effector-textures-v<N>.zip) from the source zip.

    python tools/build_textures.py <source.zip> <out.zip>

The source has one folder per category (A_DUST, B_GRUNGE, C_GLASS, F_PAPER, G_PLASTIC…; the
letter prefix only orders them). macOS junk (`._*`, `__MACOSX`) and non-images are skipped.
Every image is renamed `<category>-<n>`, shrunk to MAX_SIDE, saved as JPEG (grayscale when it
has no real colour) and gets a 256 px thumbnail. `manifest.json` lists everything; the app reads
it from `<support>/Textures` (see src/textures.rs). The zip is stored (JPEGs don't compress).
"""
import io
import json
import re
import sys
import zipfile

from PIL import Image, ImageStat

MAX_SIDE = 2560
QUALITY = 85
THUMB = 256
VERSION = 1


def natural(s):
    return [int(t) if t.isdigit() else t.lower() for t in re.split(r"(\d+)", s)]


def main(src, out):
    z = zipfile.ZipFile(src)
    groups = {}
    for info in z.infolist():
        parts = info.filename.split("/")
        name = parts[-1]
        if info.is_dir() or len(parts) < 2 or "__MACOSX" in parts or name.startswith("._"):
            continue
        if not name.lower().endswith((".jpg", ".jpeg", ".png", ".tif", ".tiff", ".webp")):
            continue
        groups.setdefault(parts[0], []).append(info)
    cats = []
    with zipfile.ZipFile(out, "w", zipfile.ZIP_STORED) as o:
        for folder in sorted(groups):
            cid = re.sub(r"^[A-Z]_", "", folder).lower()
            cname = cid.capitalize()
            items = []
            for n, info in enumerate(sorted(groups[folder], key=lambda i: natural(i.filename.split("/")[-1])), 1):
                im = Image.open(io.BytesIO(z.read(info)))
                im.load()
                rgb = im.convert("RGB")
                small = rgb.resize((128, 128))
                sat = ImageStat.Stat(small.convert("HSV").getchannel("S")).mean[0]
                img = rgb.convert("L") if sat < 12 else rgb
                img.thumbnail((MAX_SIDE, MAX_SIDE), Image.LANCZOS)
                b = io.BytesIO()
                img.save(b, "JPEG", quality=QUALITY, optimize=True, progressive=False)
                file = f"{cid}/{cid}-{n}.jpg"
                o.writestr(file, b.getvalue())
                t = img.copy()
                t.thumbnail((THUMB, THUMB), Image.LANCZOS)
                tb = io.BytesIO()
                t.convert("RGB").save(tb, "JPEG", quality=80)
                thumb = f"thumbs/{cid}/{cid}-{n}.jpg"
                o.writestr(thumb, tb.getvalue())
                items.append({"id": f"{cid}/{cid}-{n}", "name": f"{cname} {n}", "file": file, "thumb": thumb, "w": img.width, "h": img.height})
                print(f"{file} {img.width}x{img.height} {'gray' if img.mode == 'L' else 'rgb'} {len(b.getvalue()) // 1024} KB", flush=True)
            cats.append({"id": cid, "name": cname, "items": items})
        o.writestr("manifest.json", json.dumps({"version": VERSION, "categories": cats}, indent=1))
    print("categories:", ", ".join(f"{c['name']} ({len(c['items'])})" for c in cats))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
