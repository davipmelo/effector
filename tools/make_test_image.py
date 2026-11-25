# Generates a synthetic test photo (sky gradient, sun, colour patches, grey ramp, lights) for render tests.
import sys, math, random
from PIL import Image, ImageDraw, ImageFilter

W, H = 1500, 1000
img = Image.new('RGB', (W, H))
d = ImageDraw.Draw(img)
for y in range(H):
    t = y / H
    d.line([(0, y), (W, y)], fill=(int(40 + 120 * (1 - t)), int(70 + 100 * (1 - t)), int(140 + 90 * (1 - t))))
# ground
d.rectangle([0, int(H * 0.62), W, H], fill=(70, 90, 50))
# sun + bright lights
d.ellipse([1100, 120, 1260, 280], fill=(255, 245, 210))
random.seed(3)
for i in range(30):
    x, y = random.randint(50, W - 50), random.randint(int(H * 0.65), H - 40)
    d.ellipse([x - 5, y - 5, x + 5, y + 5], fill=(255, 230, 150))
# colour patches
cols = [(220, 40, 40), (240, 140, 30), (240, 220, 40), (60, 180, 70), (40, 200, 210), (40, 90, 220), (140, 60, 220), (230, 50, 150)]
for i, c in enumerate(cols):
    d.rectangle([60 + i * 120, 680, 160 + i * 120, 780], fill=c)
# grey ramp
for x in range(1000):
    v = int(x / 999 * 255)
    d.line([(60 + x, 820), (60 + x, 900)], fill=(v, v, v))
# a "face"-like blob
d.ellipse([500, 220, 760, 520], fill=(225, 175, 140))
d.ellipse([560, 320, 600, 360], fill=(40, 30, 30))
d.ellipse([660, 320, 700, 360], fill=(40, 30, 30))
d.arc([570, 380, 690, 470], 20, 160, fill=(120, 40, 40), width=8)
img = img.filter(ImageFilter.GaussianBlur(0.6))
img.save(sys.argv[1], quality=95)
