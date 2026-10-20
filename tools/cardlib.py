"""Engraved sterling-silver playing cards for the Devil's Table (rendered once, shipped as PNG).
Everything is drawn at 2x the shipped size and scaled down, so the linework stays crisp."""
import math, random
import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont, ImageChops, ImageOps

W, H = 1536, 2150                  # working size (the shipped card is 384 x 538)
R = int(W * 0.07)                  # corner radius
INK = np.array([15, 17, 27], dtype=np.float32)
SERIF = '/usr/share/fonts/truetype/dejavu/DejaVuSerifCondensed-Bold.ttf'
BLUE_DEEP = (118, 176, 240)
BLUE_BRIGHT = (186, 226, 255)
CLUB, DIAMOND, HEART, SPADE = 0, 1, 2, 3
RANKS = '2345678910JQKA'

def new_mask():
    return Image.new('L', (W, H), 0)

def blur(a, s):
    return np.asarray(Image.fromarray(a).filter(ImageFilter.GaussianBlur(s)))

# ---------------------------------------------------------------- the plate
def rr_mask(inset=0, radius=R):
    m = Image.new('L', (W, H), 0)
    ImageDraw.Draw(m).rounded_rectangle([inset, inset, W - 1 - inset, H - 1 - inset], radius=max(radius, 2), fill=255)
    return m

_plate = None
def plate():
    """Satin silver: soft brushed grain, a broad dull sheen, a lit top-left edge and a shaded bottom-right."""
    global _plate
    if _plate is not None:
        return _plate.copy()
    rng = np.random.default_rng(7)
    yy, xx = np.mgrid[0:H, 0:W].astype(np.float32)
    base = np.array([170, 173, 180], dtype=np.float32)
    img = np.empty((H, W, 3), np.float32)
    img[:] = base
    # light falls from the upper left
    g = (xx / W * 0.45 + yy / H * 0.55)
    img *= (1.10 - 0.20 * g)[..., None]
    # brushed grain: long horizontal streaks
    n = rng.standard_normal((H, W)).astype(np.float32)
    n = np.asarray(Image.fromarray(((n * 40) + 128).clip(0, 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(0.8))).astype(np.float32)
    n = np.asarray(Image.fromarray(n.astype(np.uint8)).resize((W // 30, H)).resize((W, H), Image.BICUBIC)).astype(np.float32)
    img += ((n - 128) * 1.5)[..., None]
    img += rng.standard_normal((H, W, 1)).astype(np.float32) * 1.6
    # a broad dull sheen band, diagonal
    d = (xx * 0.62 + yy * 0.78) / 1000.0
    sheen = np.exp(-((d - 2.05) / 0.45) ** 2) * 22 + np.exp(-((d - 0.7) / 0.35) ** 2) * 9
    img += sheen[..., None]
    # the edge: lit along the top and left, shaded along the bottom and right
    a = np.asarray(rr_mask()).astype(np.float32) / 255
    sh = 9
    lit = np.clip(a - np.roll(np.roll(a, sh, 0), sh, 1), 0, 1)
    dark = np.clip(a - np.roll(np.roll(a, -sh, 0), -sh, 1), 0, 1)
    lit = blur((lit * 255).astype(np.uint8), 2.5).astype(np.float32) / 255
    dark = blur((dark * 255).astype(np.uint8), 2.5).astype(np.float32) / 255
    img += lit[..., None] * 40 - dark[..., None] * 46
    # a thin dark line right at the edge
    inner = np.asarray(rr_mask(5)).astype(np.float32) / 255
    img *= (1 - 0.30 * (a - inner))[..., None]
    _plate = np.clip(img, 0, 255)
    return _plate.copy()

# ---------------------------------------------------------------- paths
def rrect_path(inset, step=3.0):
    """Points along a rounded rectangle, inset from the edge: (x, y, inward normal x, inward normal y)."""
    x0, y0, x1, y1 = inset, inset, W - inset, H - inset
    r = max(R - inset, 10)
    pts = []
    def line(ax, ay, bx, by, nx, ny):
        L = math.hypot(bx - ax, by - ay)
        k = max(int(L / step), 1)
        for i in range(k):
            t = i / k
            pts.append((ax + (bx - ax) * t, ay + (by - ay) * t, nx, ny))
    def arc(cx, cy, a0, a1):
        L = r * abs(a1 - a0)
        k = max(int(L / step), 2)
        for i in range(k):
            a = a0 + (a1 - a0) * i / k
            pts.append((cx + r * math.cos(a), cy + r * math.sin(a), -math.cos(a), -math.sin(a)))
    line(x0 + r, y0, x1 - r, y0, 0, 1)
    arc(x1 - r, y0 + r, -math.pi / 2, 0)
    line(x1, y0 + r, x1, y1 - r, -1, 0)
    arc(x1 - r, y1 - r, 0, math.pi / 2)
    line(x1 - r, y1, x0 + r, y1, 0, -1)
    arc(x0 + r, y1 - r, math.pi / 2, math.pi)
    line(x0, y1 - r, x0, y0 + r, 1, 0)
    arc(x0 + r, y0 + r, math.pi, 1.5 * math.pi)
    return pts

def offset_line(pts, off, wave=None):
    out = []
    for i, (x, y, nx, ny) in enumerate(pts):
        o = off + (wave(i) if wave else 0)
        out.append((x + nx * o, y + ny * o))
    return out

def poly(d, pts, w, fill=255, closed=False):
    if closed:
        pts = list(pts) + [pts[0]]
    d.line(pts, fill=fill, width=int(w), joint='curve')

def guilloche(d, inset, amp, wave_len, w, rails=True):
    pts = rrect_path(inset)
    n = len(pts)
    # arc length per sample is ~3px: wave over `wave_len` px
    k = 2 * math.pi / (wave_len / 3.0)
    for ph in (0, math.pi):
        poly(d, offset_line(pts, 0, lambda i, ph=ph: amp * math.sin(i * k + ph)), w, closed=True)
    if rails:
        for o in (-amp - w * 1.8, amp + w * 1.8):
            poly(d, offset_line(pts, o), w * 0.8, closed=True)

# ---------------------------------------------------------------- suits
def heart_pts(n=160):
    out = []
    for i in range(n):
        t = 2 * math.pi * i / n
        x = 16 * math.sin(t) ** 3
        y = 13 * math.cos(t) - 5 * math.cos(2 * t) - 2 * math.cos(3 * t) - math.cos(4 * t)
        out.append((x / 17.0, -y / 17.0))    # y down, about -0.9..1.0
    return out

def suit_mask(suit, size):
    """The suit as an L mask in a (size x size) square, filled."""
    s = size
    m = Image.new('L', (s, s), 0)
    d = ImageDraw.Draw(m)
    cx = s / 2
    if suit == HEART:
        pts = [(cx + x * s * 0.50, s * 0.46 + y * s * 0.50) for x, y in heart_pts()]
        d.polygon(pts, fill=255)
    elif suit == DIAMOND:
        k = 40
        top, right, bot, left = (cx, s * 0.03), (cx + s * 0.40, s * 0.5), (cx, s * 0.97), (cx - s * 0.40, s * 0.5)
        pts = []
        for a, b in ((top, right), (right, bot), (bot, left), (left, top)):
            for i in range(k):
                t = i / k
                x, y = a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t
                # bow the sides in a little
                bow = math.sin(math.pi * t) * s * 0.035
                mx, my = cx - x, s * 0.5 - y
                L = math.hypot(mx, my) or 1
                pts.append((x + mx / L * bow, y + my / L * bow))
        d.polygon(pts, fill=255)
    elif suit == SPADE:
        pts = [(cx + x * s * 0.50, s * 0.54 - y * s * 0.50) for x, y in heart_pts()]
        d.polygon(pts, fill=255)
        # stem
        d.polygon([(cx - s * 0.05, s * 0.66), (cx + s * 0.05, s * 0.66), (cx + s * 0.20, s * 0.97), (cx - s * 0.20, s * 0.97)], fill=255)
        d.ellipse([cx - s * 0.22, s * 0.90, cx + s * 0.22, s * 0.99], fill=255)
    else:
        r = s * 0.235
        for (x, y) in ((0.5, 0.27), (0.27, 0.56), (0.73, 0.56)):
            d.ellipse([s * x - r, s * y - r, s * x + r, s * y + r], fill=255)
        d.polygon([(s * 0.5, s * 0.4), (s * 0.34, s * 0.58), (s * 0.66, s * 0.58)], fill=255)
        d.polygon([(cx - s * 0.05, s * 0.55), (cx + s * 0.05, s * 0.55), (cx + s * 0.19, s * 0.97), (cx - s * 0.19, s * 0.97)], fill=255)
        d.ellipse([cx - s * 0.21, s * 0.90, cx + s * 0.21, s * 0.99], fill=255)
    return m

def erode(m, px):
    a = np.asarray(m)
    return Image.fromarray(a).filter(ImageFilter.MinFilter(int(px) * 2 + 1)) if px >= 1 else m

def hatch_mask(size, spacing, width, angle_deg):
    m = Image.new('L', (size, size), 0)
    d = ImageDraw.Draw(m)
    a = math.radians(angle_deg)
    dx, dy = math.cos(a), math.sin(a)
    c = size / 2
    for i in range(-int(size * 1.5 / spacing), int(size * 1.5 / spacing)):
        o = i * spacing
        px, py = c - dy * o, c + dx * o
        d.line([(px - dx * size, py - dy * size), (px + dx * size, py + dy * size)], fill=255, width=max(int(width), 1))
    return m

_pips = {}
def polished_pip(suit, size):
    """The big pip: polished silver in an inked outline, a recessed engraved line, hatched shade in the
    lower right and a bright catch of light in the upper left. Returns an RGBA image (size x size)."""
    key = (suit, size)
    if key in _pips:
        return _pips[key].copy()
    ss = 2
    S = size * ss
    mask = suit_mask(suit, S)
    outline = max(int(S * 0.020), 2)
    body = erode(mask, outline)
    ring = ImageChops.subtract(mask, body)
    inner_line = ImageChops.subtract(erode(mask, int(S * 0.045)), erode(mask, int(S * 0.045) + max(int(S * 0.008), 1)))
    yy, xx = np.mgrid[0:S, 0:S].astype(np.float32)
    g = (xx + yy) / (2 * S)
    # polished: bright top left, a dark turn in the lower right, with a bright reflected rim under it
    metal = 238 - 150 * np.clip((g - 0.28) / 0.55, 0, 1) ** 1.2
    refl = np.exp(-((g - 0.93) / 0.05) ** 2) * 55
    metal = metal + refl
    cool = np.stack([metal * 0.97, metal * 0.99, metal * 1.03], -1)
    # hatching, only where the pip turns away from the light
    hat = np.asarray(hatch_mask(S, S * 0.030, max(S * 0.004, 1), 38)).astype(np.float32) / 255
    shade = np.clip((g - 0.52) / 0.18, 0, 1)
    body_a = np.asarray(erode(mask, int(S * 0.05))).astype(np.float32) / 255
    cool *= (1 - 0.55 * hat * shade * body_a)[..., None]
    # a specular spot
    sp = np.exp(-(((xx - S * 0.34) ** 2 + (yy - S * 0.30) ** 2) / (S * 0.10) ** 2)) * 90 * body_a
    cool += sp[..., None]
    out = np.zeros((S, S, 4), np.float32)
    out[..., :3] = np.clip(cool, 0, 255)
    ink = np.asarray(ring).astype(np.float32) / 255
    il = np.asarray(inner_line).astype(np.float32) / 255
    for c in range(3):
        out[..., c] = out[..., c] * (1 - ink) + INK[c] * ink
        out[..., c] = out[..., c] * (1 - 0.65 * il) + INK[c] * 0.65 * il
    out[..., 3] = np.asarray(mask).astype(np.float32)
    img = Image.fromarray(out.clip(0, 255).astype(np.uint8), 'RGBA').resize((size, size), Image.LANCZOS)
    _pips[key] = img
    return img.copy()

_soul = {}
def soul_pip(suit, size):
    """The small corner pip: the soul blue, with a dark navy outline and a glint (brighter blue for the red suits)."""
    key = (suit, size)
    if key in _soul:
        return _soul[key].copy()
    ss = 3
    S = size * ss
    mask = suit_mask(suit, S)
    ol = max(int(S * 0.045), 2)
    body = erode(mask, ol)
    ring = ImageChops.subtract(mask, body)
    col = np.array(BLUE_BRIGHT if suit in (DIAMOND, HEART) else BLUE_DEEP, np.float32)
    yy, xx = np.mgrid[0:S, 0:S].astype(np.float32)
    g = (xx + yy) / (2 * S)
    fill = col[None, None, :] * (1.08 - 0.34 * g)[..., None]
    gl = np.exp(-(((xx - S * 0.36) ** 2 + (yy - S * 0.32) ** 2) / (S * 0.12) ** 2)) * 120
    fill = fill + gl[..., None]
    navy = np.array([14, 36, 84], np.float32)
    ink = np.asarray(ring).astype(np.float32) / 255
    out = np.zeros((S, S, 4), np.float32)
    for c in range(3):
        out[..., c] = fill[..., c] * (1 - ink) + navy[c] * ink
    out[..., 3] = np.asarray(mask).astype(np.float32)
    img = Image.fromarray(out.clip(0, 255).astype(np.uint8), 'RGBA').resize((size, size), Image.LANCZOS)
    _soul[key] = img
    return img.copy()

# ---------------------------------------------------------------- compositing
def over(img, layer, pos, rotate180=False, shadow=0.0):
    """Paste an RGBA layer (PIL) onto the float image `img` (H, W, 3) with its middle at pos. `shadow` (0..1)
    drops a soft shadow down and to the right of it, as if it stood proud of the plate."""
    if rotate180:
        layer = layer.rotate(180)
    x, y = int(pos[0] - layer.width / 2), int(pos[1] - layer.height / 2)
    a = np.asarray(layer).astype(np.float32)
    x0, y0 = max(x, 0), max(y, 0)
    x1, y1 = min(x + layer.width, W), min(y + layer.height, H)
    if x1 <= x0 or y1 <= y0:
        return
    if shadow > 0:
        pad = int(layer.width * 0.08) + 6
        al = Image.new('L', (layer.width + pad * 4, layer.height + pad * 4), 0)
        al.paste(layer.getchannel('A'), (pad * 2, pad * 2))
        al = np.asarray(al.filter(ImageFilter.GaussianBlur(pad * 0.55))).astype(np.float32) / 255
        off = int(layer.width * 0.035) + 2
        sx, sy = x - pad * 2 + off, y - pad * 2 + off
        gx0, gy0 = max(sx, 0), max(sy, 0)
        gx1, gy1 = min(sx + al.shape[1], W), min(sy + al.shape[0], H)
        if gx1 > gx0 and gy1 > gy0:
            part = al[gy0 - sy:gy1 - sy, gx0 - sx:gx1 - sx]
            img[gy0:gy1, gx0:gx1] *= (1 - 0.55 * shadow * part)[..., None]
    sub = a[y0 - y:y1 - y, x0 - x:x1 - x]
    al = sub[..., 3:4] / 255
    img[y0:y1, x0:x1] = img[y0:y1, x0:x1] * (1 - al) + sub[..., :3] * al

def lay_ink(img, mask, strength=1.0, relief=True, col=None):
    """Lay an ink mask over the plate: a pale lip below and right of every engraved line (so that it looks cut
    into the metal), then the dark ink."""
    m = np.asarray(mask).astype(np.float32) / 255
    if relief:
        lip = np.roll(np.roll(m, 3, 0), 3, 1) * (1 - m)
        img += lip[..., None] * 34 * strength
        shade = np.roll(np.roll(m, -2, 0), -2, 1) * (1 - m)
        img -= shade[..., None] * 14 * strength
    for c in range(3):
        img[..., c] = img[..., c] * (1 - m * 0.94 * strength) + (INK if col is None else col)[c] * m * 0.94 * strength

def finish(img, alpha_mask=None):
    a = np.asarray(rr_mask() if alpha_mask is None else alpha_mask).astype(np.float32)
    out = np.dstack([np.clip(img, 0, 255), a]).astype(np.uint8)
    im = Image.fromarray(out, 'RGBA')
    return im.resize((W // 4, H // 4), Image.LANCZOS)

# ---------------------------------------------------------------- 3D pips
from scipy import ndimage as ndi

_chrome = {}
def chrome_pip(suit, size):
    """The big centre pip, in the round: an inflated, bevelled piece of polished silver standing proud of the
    plate on a visible edge, lit from the upper left, with a chrome horizon, a hard glint and a dark inked
    outline. RGBA, size x size (the piece itself takes the upper left 92 percent, its thickness the rest)."""
    key = (suit, size)
    if key in _chrome:
        return _chrome[key].copy()
    ss = 2
    S = size * ss
    body = int(S * 0.915)
    m_small = suit_mask(suit, body)
    mask = np.zeros((S, S), np.float32)
    mask[:body, :body] = np.asarray(m_small).astype(np.float32) / 255
    solid = mask > 0.5
    dist = ndi.distance_transform_edt(solid)
    dmax = dist.max()
    t = np.clip(dist / (dmax * 0.30), 0, 1)
    rim = np.sqrt(1 - (1 - t) ** 2)               # steep rounded bevel at the edge
    dome = ndi.gaussian_filter(solid.astype(np.float32), S * 0.07)   # smooth pillow, no medial creases
    dome = dome / dome.max()
    h = (0.55 * rim + 0.75 * dome) * solid
    h = ndi.gaussian_filter(h, S * 0.006)
    gy, gx = np.gradient(h)
    k = S * 0.085
    nx, ny, nz = -gx * k, -gy * k, np.ones_like(h)
    ln = np.sqrt(nx * nx + ny * ny + nz * nz)
    nx, ny, nz = nx / ln, ny / ln, nz / ln
    # light from the upper left, a little toward the viewer
    L = np.array([-0.45, -0.62, 0.64]); L = L / np.linalg.norm(L)
    diff = np.clip(nx * L[0] + ny * L[1] + nz * L[2], 0, 1)
    # chrome: the reflection of a sky above, a dark horizon and bright ground
    ry = 2 * nz * ny
    u = np.clip((-ry + 1) / 2, 0, 1)
    env = np.interp(u, [0.0, 0.18, 0.44, 0.50, 0.56, 0.80, 1.0], [0.10, 0.55, 0.22, 1.00, 0.18, 0.55, 0.95])
    Hh = L + np.array([0, 0, 1.0]); Hh = Hh / np.linalg.norm(Hh)
    spec = np.clip(nx * Hh[0] + ny * Hh[1] + nz * Hh[2], 0, 1) ** 70
    spec2 = np.clip(nx * Hh[0] + ny * Hh[1] + nz * Hh[2], 0, 1) ** 12 * 0.25
    base = np.array([192, 197, 208], np.float32)
    shade = (0.30 + 0.42 * env + 0.40 * diff)[..., None]
    col = base[None, None, :] * shade + (spec * 255)[..., None] + (spec2 * 80)[..., None]
    # a cold blue kiss in the shadow side, the reflected soul light
    col[..., 2] += (1 - diff)[..., None][..., 0] * 14
    out = np.zeros((S, S, 4), np.float32)
    # the thickness: copies of the shape stepped toward the lower right, darkening as they go
    depth = int(S * 0.045)
    for i in range(depth, 0, -1):
        sh = np.roll(np.roll(mask, i, 0), int(i * 0.75), 1)
        a = (sh > 0.5).astype(np.float32)
        side = (74 + 46 * (1 - i / depth))
        for c, v in enumerate((side, side + 3, side + 10)):
            out[..., c] = out[..., c] * (1 - a) + v * a
        out[..., 3] = np.maximum(out[..., 3], a * 255)
    top = mask.copy()
    for c in range(3):
        out[..., c] = out[..., c] * (1 - top) + np.clip(col[..., c], 0, 255) * top
    out[..., 3] = np.maximum(out[..., 3], top * 255)
    # the inked outline round the top face and a darker one round the foot of the wall
    er = ndi.binary_erosion(solid, iterations=max(int(S * 0.010), 2))
    ring = (solid & ~er).astype(np.float32)
    ring = ndi.gaussian_filter(ring, 0.7)
    for c in range(3):
        out[..., c] = out[..., c] * (1 - 0.88 * ring) + INK[c] * 0.88 * ring
    foot = ndi.binary_dilation(np.roll(np.roll(solid, depth, 0), int(depth * 0.75), 1), iterations=max(int(S * 0.006), 1)) & ~np.roll(np.roll(solid, depth, 0), int(depth * 0.75), 1)
    foot = ndi.gaussian_filter(foot.astype(np.float32), 0.8) * (out[..., 3] > 0)
    for c in range(3):
        out[..., c] = out[..., c] * (1 - 0.7 * foot) + INK[c] * 0.7 * foot
    img = Image.fromarray(out.clip(0, 255).astype(np.uint8), 'RGBA').resize((size, size), Image.LANCZOS)
    _chrome[key] = img
    return img.copy()

# ---------------------------------------------------------------- red and black
RED = np.array([184, 20, 34], dtype=np.float32)

def is_red(suit):
    return suit in (DIAMOND, HEART)

def suit_ink(suit):
    return RED if is_red(suit) else INK

_enamel = {}
def enamel_pip(suit, size):
    """A small pip in glossy enamel, red for hearts and diamonds and black for clubs and spades, with a dark
    outline and a catch of light."""
    key = (suit, size)
    if key in _enamel:
        return _enamel[key].copy()
    ss = 3
    S = size * ss
    mask = suit_mask(suit, S)
    ol = max(int(S * 0.04), 2)
    body = erode(mask, ol)
    ring = ImageChops.subtract(mask, body)
    if is_red(suit):
        top, bot = np.array([232, 52, 62], np.float32), np.array([150, 12, 28], np.float32)
        edge = np.array([60, 4, 12], np.float32)
    else:
        top, bot = np.array([104, 112, 134], np.float32), np.array([16, 17, 26], np.float32)
        edge = np.array([4, 5, 9], np.float32)
    yy, xx = np.mgrid[0:S, 0:S].astype(np.float32)
    g = np.clip((xx * 0.4 + yy * 0.9) / (1.3 * S), 0, 1)
    fill = top[None, None, :] * (1 - g)[..., None] + bot[None, None, :] * g[..., None]
    gl = np.exp(-(((xx - S * 0.36) ** 2 + (yy - S * 0.30) ** 2) / (S * 0.13) ** 2)) * (150 if is_red(suit) else 110)
    fill = fill + gl[..., None]
    inkm = np.asarray(ring).astype(np.float32) / 255
    out = np.zeros((S, S, 4), np.float32)
    for c in range(3):
        out[..., c] = fill[..., c] * (1 - inkm) + edge[c] * inkm
    out[..., 3] = np.asarray(mask).astype(np.float32)
    img = Image.fromarray(out.clip(0, 255).astype(np.uint8), 'RGBA').resize((size, size), Image.LANCZOS)
    _enamel[key] = img
    return img.copy()

def glint_mask(suit, side=190):
    """The top face of the big chrome pip as a white picture with its shape as the alpha, laid out exactly as
    chrome_pip lays it out in a square (the program shows a moving gleam through it)."""
    size = 760
    body = int(size * 0.915)
    m = suit_mask(suit, body)
    full = Image.new('L', (size, size), 0)
    full.paste(m, (0, 0))
    full = full.resize((side, side), Image.LANCZOS)
    out = Image.new('RGBA', (side, side), (255, 255, 255, 0))
    out.putalpha(full)
    return out

# ---------------------------------------------------------------- soul wells
_wells = {}
def soul_well(suit, size):
    """A round well sunk into the plate with a bezel of silver, deep navy within, and a soul-blue pip lit in it
    that gives off a halo. RGBA, size x size."""
    key = (suit, size)
    if key in _wells:
        return _wells[key].copy()
    ss = 2
    S = size * ss
    yy, xx = np.mgrid[0:S, 0:S].astype(np.float32)
    cx = cy = (S - 1) / 2
    r = np.hypot(xx - cx, yy - cy) / (S / 2)
    out = np.zeros((S, S, 4), np.float32)
    # bezel: a rolled ring from r .86 to 1.0, light on the upper left, dark lower right
    ang = np.arctan2(yy - cy, xx - cx)
    lit = 0.5 + 0.5 * np.cos(ang + 2.35)                      # brightest toward the upper left
    bez = np.array([196, 201, 212], np.float32)
    ring_t = np.clip((r - 0.84) / 0.16, 0, 1)
    ring_sh = (0.45 + 0.55 * lit) * (0.75 + 0.25 * np.sin(ring_t * math.pi))
    bezel = bez[None, None, :] * ring_sh[..., None]
    # the well: deep navy, shadowed along the upper left wall (light comes from there), glowing in the middle
    navy = np.array([8, 14, 34], np.float32)
    glow = np.exp(-(r / 0.55) ** 2)
    well = navy[None, None, :] + glow[..., None] * np.array([38, 92, 170], np.float32)
    wall = np.clip((r - 0.55) / 0.29, 0, 1) ** 2 * (0.35 + 0.65 * (1 - lit))
    well = well * (1 - 0.65 * wall[..., None])
    inwell = (r < 0.84)
    col = np.where(inwell[..., None], well, bezel)
    # inked line at the foot of the bezel and at its outer edge
    for rr, w, k in ((0.845, 0.012, 0.9), (0.995, 0.014, 0.9)):
        m = np.exp(-((r - rr) / w) ** 2)
        col = col * (1 - k * m[..., None]) + INK[None, None, :] * k * m[..., None]
    out[..., :3] = col
    out[..., 3] = np.clip((1.0 - r) * S / 2, 0, 1) * 255
    img = Image.fromarray(out.clip(0, 255).astype(np.uint8), 'RGBA')
    # the soul pip, bright, with a halo
    pip = enamel_pip(suit, int(S * 0.50))
    halo = Image.new('RGBA', (S, S), (0, 0, 0, 0))
    a = pip.getchannel('A').filter(ImageFilter.GaussianBlur(S * 0.05))
    glow_l = Image.new('RGBA', pip.size, (150, 210, 255, 0)); glow_l.putalpha(a.point(lambda v: min(255, int(v * 1.25))))
    px = ((S - pip.width) // 2, (S - pip.height) // 2 - int(S * 0.01))
    big = Image.new('RGBA', (S, S), (0, 0, 0, 0))
    big.paste(glow_l, px)
    img = Image.alpha_composite(img, big)
    big2 = Image.new('RGBA', (S, S), (0, 0, 0, 0)); big2.paste(pip, px)
    img = Image.alpha_composite(img, big2)
    # restore the circular alpha
    al = np.asarray(img.getchannel('A')).astype(np.float32) * (out[..., 3] / 255)
    img.putalpha(Image.fromarray(al.astype(np.uint8)))
    img = img.resize((size, size), Image.LANCZOS)
    _wells[key] = img
    return img.copy()
