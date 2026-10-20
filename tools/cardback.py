import sys; sys.path.insert(0, '.')
from cardface import *
from courts import clip_hatch

def tri_mask(pts):
    m = Image.new('L', (W, H), 0)
    ImageDraw.Draw(m).polygon(pts, fill=255)
    return m

def mask_lines(clip, draw_fn):
    layer = Image.new('L', (W, H), 0)
    draw_fn(ImageDraw.Draw(layer))
    return ImageChops.multiply(layer, clip)

def gem_big(size, bright=True):
    """A cut gem in the soul blue: eight facets round a table, with a glint."""
    S = size * 3
    im = Image.new('RGBA', (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    c = S / 2
    R_ = S * 0.46
    tones = [(190, 226, 255), (120, 176, 240), (70, 120, 210), (40, 80, 170), (60, 100, 190), (110, 168, 236), (170, 214, 252), (210, 236, 255)]
    pts = [(c + R_ * math.cos(math.pi / 8 + i * math.pi / 4), c + R_ * math.sin(math.pi / 8 + i * math.pi / 4)) for i in range(8)]
    inner = [(c + R_ * 0.5 * math.cos(math.pi / 8 + i * math.pi / 4), c + R_ * 0.5 * math.sin(math.pi / 8 + i * math.pi / 4)) for i in range(8)]
    for i in range(8):
        j = (i + 1) % 8
        d.polygon([pts[i], pts[j], inner[j], inner[i]], fill=tones[(i + 5) % 8] + (255,))
    d.polygon(inner, fill=(150, 200, 250, 255))
    d.polygon([inner[0], inner[1], inner[2], inner[3]], fill=(215, 238, 255, 255))
    for i in range(8):
        d.line([pts[i], inner[i]], fill=(14, 36, 84, 255), width=int(S * 0.012))
    d.polygon(pts, outline=(14, 36, 84, 255), width=int(S * 0.04))
    d.polygon(inner, outline=(14, 36, 84, 200), width=int(S * 0.012))
    # a star of light
    d.line([(c - S * 0.2, c - S * 0.2), (c - S * 0.05, c - S * 0.05)], fill=(255, 255, 255, 230), width=int(S * 0.02))
    return im.resize((size, size), Image.LANCZOS)

def star_poly(c, r_out, r_in, n, rot=0):
    pts = []
    for i in range(n * 2):
        a = rot + math.pi * i / n
        r_ = r_out if i % 2 == 0 else r_in
        pts.append((c[0] + r_ * math.cos(a), c[1] + r_ * math.sin(a)))
    return pts

def build_back():
    img = plate()
    ink = new_mask(); d = ImageDraw.Draw(ink)
    frame_ink(ink, d)
    cx, cy = W / 2, H / 2
    # the field: inside the inner frame
    fx0, fy0, fx1, fy1 = 168, 168, W - 168, H - 168
    d.rectangle([fx0, fy0, fx1, fy1], outline=255, width=7)
    d.rectangle([fx0 + 22, fy0 + 22, fx1 - 22, fy1 - 22], outline=255, width=3)
    corners = [(fx0, fy0), (fx1, fy0), (fx1, fy1), (fx0, fy1)]
    # the four triangles cut by the diagonals
    top = tri_mask([(fx0, fy0), (fx1, fy0), (cx, cy)])
    bot = tri_mask([(fx0, fy1), (fx1, fy1), (cx, cy)])
    lef = tri_mask([(fx0, fy0), (fx0, fy1), (cx, cy)])
    rig = tri_mask([(fx1, fy0), (fx1, fy1), (cx, cy)])
    fan = new_mask(); fd = ImageDraw.Draw(fan)
    def rays(dd, center, n=240, w=3):
        for i in range(n):
            a = 2 * math.pi * i / n
            dd.line([(center[0] + 330 * math.cos(a), center[1] + 330 * math.sin(a)), (center[0] + 2600 * math.cos(a), center[1] + 2600 * math.sin(a))], fill=255, width=w)
    ink.paste(255, mask=mask_lines(top, lambda dd: rays(dd, (cx, cy))))
    ink.paste(255, mask=mask_lines(bot, lambda dd: rays(dd, (cx, cy))))
    def lattice(dd, step=46, w=4):
        for i in range(-60, 90):
            o = i * step
            dd.line([(o, 0), (o + H, H)], fill=255, width=w)
            dd.line([(o + H, 0), (o, H)], fill=255, width=w)
    ink.paste(255, mask=mask_lines(lef, lambda dd: lattice(dd)))
    ink.paste(255, mask=mask_lines(rig, lambda dd: lattice(dd)))
    # a dot in every lattice diamond of the sides, so that they read as quilting
    for tri in (lef, rig):
        pass
    # the diagonals, as double rules
    for (a, b) in zip(corners, ((cx, cy),) * 4):
        d.line([a, b], fill=255, width=12)
    # the medallion
    d.ellipse([cx - 470, cy - 470, cx + 470, cy + 470], fill=0)
    # (knock the lines out of the medallion's disc first, then draw it)
    disc = Image.new('L', (W, H), 0); ImageDraw.Draw(disc).ellipse([cx - 470, cy - 470, cx + 470, cy + 470], fill=255)
    ink_arr = np.asarray(ink).astype(np.uint8)
    ink_arr = np.where(np.asarray(disc) > 0, 0, ink_arr).astype(np.uint8)
    ink = Image.fromarray(ink_arr); d = ImageDraw.Draw(ink)
    for r_, w in ((468, 12), (440, 4), (404, 4), (250, 5), (232, 3)):
        d.ellipse([cx - r_, cy - r_, cx + r_, cy + r_], outline=255, width=w)
    beads(d, (cx, cy), 422, 96, 6)
    rosette(d, (cx, cy), 250, 396, 16)
    d.polygon(star_poly((cx, cy), 232, 150, 12, math.pi / 12), outline=255, width=5)
    d.ellipse([cx - 120, cy - 120, cx + 120, cy + 120], outline=255, width=6)
    # corner fleurons
    for (x, y) in corners:
        sx = 1 if x < cx else -1
        sy = 1 if y < cy else -1
        ox, oy = x + sx * 118, y + sy * 118
        d.ellipse([ox - 74, oy - 74, ox + 74, oy + 74], fill=0)
        for r_, w in ((74, 7), (56, 3)):
            d.ellipse([ox - r_, oy - r_, ox + r_, oy + r_], outline=255, width=w)
        rosette(d, (ox, oy), 14, 52, 8)
    lay_ink(img, ink)
    # the soul gem in the middle, with a faint halo behind it
    over(img, gem_big(260), (cx, cy), shadow=1.0)
    # four small gems on the diagonals
    for (x, y) in corners:
        pass
    return finish(img)
