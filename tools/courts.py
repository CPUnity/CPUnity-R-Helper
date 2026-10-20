import sys; sys.path.insert(0, '.')
from cardface import *

LW, LH = 700, 774            # one half of the court panel
CX = LW // 2

def canvas():
    m = Image.new('L', (LW, LH), 0)
    return m, ImageDraw.Draw(m)

def clip_hatch(m, poly_pts, angle, spacing, width, dens=1.0, curve=0.0):
    """Hatch lines in ink, clipped to a polygon."""
    clip = Image.new('L', (LW, LH), 0)
    ImageDraw.Draw(clip).polygon(poly_pts, fill=255)
    h = Image.new('L', (LW, LH), 0)
    hd = ImageDraw.Draw(h)
    a = math.radians(angle)
    dx, dy = math.cos(a), math.sin(a)
    n = int(math.hypot(LW, LH) / spacing) + 2
    cx, cy = LW / 2, LH / 2
    for i in range(-n, n):
        o = i * spacing
        px, py = cx - dy * o, cy + dx * o
        if curve:
            pts = []
            for t in range(-40, 41):
                u = t * 20
                pts.append((px + dx * u - dy * curve * math.sin(u / 160 + i * 0.3) * 6, py + dy * u + dx * curve * math.sin(u / 160 + i * 0.3) * 6))
            hd.line(pts, fill=255, width=width, joint='curve')
        else:
            hd.line([(px - dx * 1500, py - dy * 1500), (px + dx * 1500, py + dy * 1500)], fill=255, width=width)
    m.paste(255, mask=ImageChops.multiply(clip, h))

def outline(d, pts, w=8, closed=True):
    d.line(list(pts) + ([pts[0]] if closed else []), fill=255, width=w, joint='curve')

def ellipse_pts(cx, cy, rx, ry, n=90, a0=0, a1=2 * math.pi):
    return [(cx + rx * math.cos(a0 + (a1 - a0) * i / n), cy + ry * math.sin(a0 + (a1 - a0) * i / n)) for i in range(n + 1)]

def bez(p0, p1, p2, p3, n=40):
    pts = []
    for i in range(n + 1):
        t = i / n
        u = 1 - t
        pts.append((u**3 * p0[0] + 3 * u * u * t * p1[0] + 3 * u * t * t * p2[0] + t**3 * p3[0],
                    u**3 * p0[1] + 3 * u * u * t * p1[1] + 3 * u * t * t * p2[1] + t**3 * p3[1]))
    return pts

def mirror(pts):
    return [(LW - x, y) for x, y in pts]

def face(m, d, cy=360, rx=112, ry=146, beard=False, tilt=0, plead=False):
    """The face: a long oval, shaded down its right side, with brows, almond eyes, nose and mouth."""
    head = ellipse_pts(CX, cy, rx, ry)
    # a chin, a little pointed
    head = [(x, y + (max(0, (y - cy)) ** 2) * 0.0008 * 0) for x, y in head]
    outline(d, head, 8)
    # the shade on the right side
    shade = [(x, y) for x, y in head if x > CX + 28]
    clip_hatch(m, head, 72, 11, 3)          # (fine hatch over the whole face, lightly)
    # clear the lit side again by repainting nothing: instead hatch only the shaded part densely
    for (ex, s) in ((CX - 52, -1), (CX + 52, 1)):
        eye = ellipse_pts(ex, cy - 8, 34, 14, 40)
        d.polygon(eye, fill=0)
        outline(d, eye, 5)
        d.ellipse([ex - 9, cy - 20, ex + 9, cy - 2], fill=255)
        if plead:
            # brows drawn up and together at the inner ends, a tear under each eye
            inner = (ex + 44 * (1 if s < 0 else -1))
            outer = (ex - 44 * (1 if s < 0 else -1))
            d.line([(outer, cy - 32), (ex, cy - 50), (inner, cy - 66)], fill=255, width=8, joint='curve')
            d.line([(ex + 8 * s, cy + 12), (ex + 10 * s, cy + 40)], fill=255, width=4)
            d.ellipse([ex + 10 * s - 6, cy + 40, ex + 10 * s + 6, cy + 56], fill=255)
        else:
            d.line([(ex - 44, cy - 40 + (6 if s < 0 else 0)), (ex, cy - 50), (ex + 44, cy - 36 + (0 if s < 0 else 6))], fill=255, width=8, joint='curve')
    d.line([(CX + 4, cy - 14), (CX + 20, cy + 40), (CX + 2, cy + 58), (CX - 18, cy + 50)], fill=255, width=5, joint='curve')
    if not beard and plead:
        # a mouth drawn open and bent down: a cry for help
        mo = bez((CX - 34, cy + 104), (CX - 16, cy + 84), (CX + 16, cy + 84), (CX + 34, cy + 104)) + bez((CX + 30, cy + 108), (CX + 12, cy + 130), (CX - 12, cy + 130), (CX - 30, cy + 108))
        d.polygon(mo, fill=255)
    elif not beard:
        d.line(bez((CX - 40, cy + 92), (CX - 14, cy + 104), (CX + 14, cy + 104), (CX + 40, cy + 92)), fill=255, width=6)
        d.line(bez((CX - 30, cy + 98), (CX - 10, cy + 112), (CX + 10, cy + 112), (CX + 30, cy + 98)), fill=255, width=3)
    return head

def gem(layer_list, pos, r, suit_bright=False):
    layer_list.append((pos, r))

def ermine(d, pts_outline, rows, cols, box):
    """A trim of ermine: little tails on the white."""
    x0, y0, x1, y1 = box
    for j in range(rows):
        for i in range(cols):
            x = x0 + (x1 - x0) * (i + 0.5 * (j % 2)) / cols
            y = y0 + (y1 - y0) * (j + 0.5) / rows
            d.line([(x, y - 10), (x, y + 10)], fill=255, width=5)
            d.line([(x - 6, y + 4), (x, y + 12), (x + 6, y + 4)], fill=255, width=3)

def robe(m, d, suit, wide=0.86, cuff=True):
    """Shoulders and robe, with a collar of ermine, a jewelled chain and the suit running down the front."""
    L, Rr = CX - 330 * wide, CX + 330 * wide
    body = [(L, LH + 4), (L + 20, 640), (CX - 190, 560), (CX - 80, 520), (CX + 80, 520), (CX + 190, 560), (Rr - 20, 640), (Rr, LH + 4)]
    outline(d, body, 9)
    clip_hatch(m, body, 100, 13, 3)
    # the front opening
    outline(d, [(CX - 58, 524), (CX - 24, LH), (CX + 24, LH), (CX + 58, 524)], 6, closed=False)
    # ermine collar
    collar = [(CX - 200, 566), (CX - 90, 520), (CX + 90, 520), (CX + 200, 566), (CX + 130, 620), (CX - 130, 620)]
    d.polygon(collar, fill=0)
    outline(d, collar, 7)
    ermine(d, None, 3, 9, (CX - 150, 538, CX + 150, 614))
    # the chain and its pendant
    chain = bez((CX - 130, 600), (CX - 90, 700), (CX + 90, 700), (CX + 130, 600))
    d.line(chain, fill=255, width=8)
    for t in range(0, len(chain), 3):
        x, y = chain[t]
        d.ellipse([x - 9, y - 9, x + 9, y + 9], outline=255, width=4)
    # the suit, repeated down the front
    sm = suit_mask(suit, 56)
    for k, y in enumerate((690, 730)):
        m.paste(255, (CX - 28, y - 28), sm)

def crown_king(m, d):
    base = [(CX - 118, 232), (CX + 118, 232), (CX + 122, 188), (CX - 122, 188)]
    pts = [(CX - 122, 190), (CX - 128, 120), (CX - 76, 160), (CX - 40, 96), (CX, 150), (CX + 40, 96), (CX + 76, 160), (CX + 128, 120), (CX + 122, 190)]
    d.polygon(pts, fill=0)
    outline(d, pts + [(CX + 122, 232), (CX - 122, 232)], 9)
    outline(d, [(CX - 122, 190), (CX + 122, 190)], 5, closed=False)
    clip_hatch(m, pts + [(CX + 122, 232), (CX - 122, 232)], 62, 12, 3)
    for x, y in ((CX - 128, 112), (CX - 40, 88), (CX + 40, 88), (CX + 128, 112), (CX, 142)):
        d.ellipse([x - 13, y - 13, x + 13, y + 13], outline=255, width=6)
    # arches and the cross above them
    outline(d, bez((CX - 70, 130), (CX - 30, 30), (CX + 30, 30), (CX + 70, 130)), 7, closed=False)
    d.line([(CX, 62), (CX, 12)], fill=255, width=8)
    d.line([(CX - 22, 32), (CX + 22, 32)], fill=255, width=8)

def build_court(rank, suit):
    """Returns the half panel as an ink mask and the list of blue gems [(x, y, r)]."""
    m, d = canvas()
    gems = []
    robe(m, d, suit)
    d.rectangle([CX - 48, 480, CX + 48, 540], fill=0)          # the neck
    outline(d, [(CX - 48, 470), (CX - 52, 540)], 7, closed=False)
    outline(d, [(CX + 48, 470), (CX + 52, 540)], 7, closed=False)
    if rank == 'K':
        # hair falls to the shoulders on both sides, a full beard, the crown
        hair_l = bez((CX - 112, 280), (CX - 190, 330), (CX - 196, 470), (CX - 150, 560)) + bez((CX - 80, 540), (CX - 120, 470), (CX - 108, 380), (CX - 100, 330))
        d.polygon(hair_l, fill=0); outline(d, hair_l, 8)
        clip_hatch(m, hair_l, 80, 10, 3, curve=1.0)
        hair_r = mirror(hair_l)
        d.polygon(hair_r, fill=0); outline(d, hair_r, 8)
        clip_hatch(m, hair_r, 100, 10, 3, curve=1.0)
        head = face(m, d, beard=True, plead=True)
        beard = bez((CX - 106, 392), (CX - 120, 520), (CX - 40, 560), (CX, 580)) + bez((CX, 580), (CX + 40, 560), (CX + 120, 520), (CX + 106, 392))
        beard += [(CX + 60, 420), (CX, 450), (CX - 60, 420)]
        d.polygon(beard, fill=0); outline(d, beard, 8)
        clip_hatch(m, beard, 82, 9, 4, curve=1.4)
        moust = bez((CX, 440), (CX - 30, 428), (CX - 70, 440), (CX - 96, 470)) + bez((CX - 96, 470), (CX - 60, 456), (CX - 20, 462), (CX, 462))
        outline(d, moust, 6); outline(d, mirror(moust), 6)
        clip_hatch(m, moust, 85, 7, 3); clip_hatch(m, mirror(moust), 95, 7, 3)
        crown_king(m, d)
        gems.append((CX, 214, 15))
        # sceptre in the right hand
        d.line([(600, 250), (600, LH)], fill=255, width=12)
        d.ellipse([566, 190, 634, 258], fill=0); outline(d, ellipse_pts(600, 224, 34, 34), 8)
        d.line([(600, 150), (600, 190)], fill=255, width=8); d.line([(580, 166), (620, 166)], fill=255, width=8)
        gems.append((600, 224, 17))
    elif rank == 'Q':
        hair = bez((CX - 110, 250), (CX - 250, 280), (CX - 270, 520), (CX - 190, 640)) + bez((CX - 130, 600), (CX - 150, 500), (CX - 120, 420), (CX - 100, 380))
        d.polygon(hair, fill=0); outline(d, hair, 8)
        clip_hatch(m, hair, 78, 9, 3, curve=1.6)
        hair2 = mirror(hair)
        d.polygon(hair2, fill=0); outline(d, hair2, 8)
        clip_hatch(m, hair2, 102, 9, 3, curve=1.6)
        # hair over the brow
        cap = bez((CX - 112, 330), (CX - 100, 220), (CX + 100, 220), (CX + 112, 330)) + bez((CX + 70, 290), (CX + 20, 250), (CX - 40, 262), (CX - 112, 330))
        d.polygon(cap, fill=0); outline(d, cap, 7)
        clip_hatch(m, cap, 20, 9, 3, curve=1.0)
        head = face(m, d, plead=True)
        # a tiara
        tiara = [(CX - 96, 236), (CX - 100, 170), (CX - 52, 200), (CX, 120), (CX + 52, 200), (CX + 100, 170), (CX + 96, 236)]
        d.polygon(tiara, fill=0); outline(d, tiara, 9)
        clip_hatch(m, tiara, 62, 11, 3)
        for x, y in ((CX - 100, 164), (CX, 112), (CX + 100, 164)):
            d.ellipse([x - 12, y - 12, x + 12, y + 12], outline=255, width=6)
        gems.append((CX, 214, 14))
        # earrings and a necklace
        for s in (-1, 1):
            x = CX + s * 116
            d.line([(x, 384), (x, 420)], fill=255, width=5)
            gems.append((x, 436, 13))
        # a flower
        fx, fy = 112, 640
        for i in range(8):
            a = math.tau * i / 8 if hasattr(math, 'tau') else 2 * math.pi * i / 8
            d.polygon(ellipse_pts(fx + 30 * math.cos(a), fy + 30 * math.sin(a), 24, 15, 24, 0, 2 * math.pi), fill=0)
            outline(d, [(px, py) for px, py in ellipse_pts(fx + 30 * math.cos(a), fy + 30 * math.sin(a), 24, 15, 24)], 5)
        d.ellipse([fx - 14, fy - 14, fx + 14, fy + 14], fill=255)
        d.line([(fx, fy + 40), (fx - 6, LH)], fill=255, width=9)
    else:
        # the jack: short hair, a feathered cap, a halberd
        hair = bez((CX - 116, 300), (CX - 190, 330), (CX - 180, 430), (CX - 120, 450)) + [(CX - 100, 380), (CX - 106, 320)]
        d.polygon(hair, fill=0); outline(d, hair, 8)
        clip_hatch(m, hair, 80, 9, 3, curve=1.2)
        hair2 = mirror(hair)
        d.polygon(hair2, fill=0); outline(d, hair2, 8)
        clip_hatch(m, hair2, 100, 9, 3, curve=1.2)
        head = face(m, d)
        capb = [(CX - 126, 262), (CX - 100, 190), (CX + 20, 160), (CX + 130, 200), (CX + 118, 262), (CX + 60, 240), (CX - 60, 240)]
        d.polygon(capb, fill=0); outline(d, capb, 9)
        clip_hatch(m, capb, 30, 10, 3)
        # the feather
        fe = bez((CX + 60, 190), (CX + 240, 60), (CX + 330, 140), (CX + 270, 330)) + bez((CX + 270, 330), (CX + 250, 200), (CX + 180, 130), (CX + 60, 190))
        d.polygon(fe, fill=0); outline(d, fe, 8)
        for k in range(1, 12):
            t = k / 12
            ax, ay = bez((CX + 60, 190), (CX + 240, 60), (CX + 330, 140), (CX + 270, 330))[int(t * 40)]
            bx, by = bez((CX + 270, 330), (CX + 250, 200), (CX + 180, 130), (CX + 60, 190))[int((1 - t) * 40)]
            d.line([(ax, ay), (bx, by)], fill=255, width=3)
        gems.append((CX - 64, 226, 13))
        # a halberd
        d.line([(590, 760), (510, 220)], fill=255, width=12)
        axe = [(510, 220), (570, 250), (604, 330), (550, 360), (504, 300)]
        d.polygon(axe, fill=0); outline(d, axe, 8)
        clip_hatch(m, axe, 50, 10, 3)
        d.polygon([(500, 220), (498, 140), (516, 220)], fill=255)
        gems.append((CX, 535, 12))
    return m, gems

def monarch_rank(rank):
    return rank in ('Q', 'K')

SC = 1.18

def soul_glow():
    """The haze of a soul behind the figure: a pale blue light round the head and shoulders with tendrils
    rising off it. RGBA, LW x LH, in the scaled coordinates of the finished half."""
    yy, xx = np.mgrid[0:LH, 0:LW].astype(np.float32)
    r = np.hypot((xx - CX) / 330, (yy - 400) / 380)
    a = np.exp(-(r ** 2) * 1.3) * 0.85
    w = Image.new('L', (LW, LH), 0); wd = ImageDraw.Draw(w)
    rnd = random.Random(7)
    for i in range(9):
        x0 = CX + (i - 4) * 62 + rnd.uniform(-14, 14)
        pts = []
        for t in range(0, 60):
            u = t / 59
            y = 560 - u * 540
            x = x0 + math.sin(u * 7 + i) * 26 * u + (x0 - CX) * 0.25 * u
            pts.append((x, y))
        wd.line(pts, fill=int(130 + 90 * rnd.random()), width=5, joint='curve')
    w = np.asarray(w.filter(ImageFilter.GaussianBlur(7))).astype(np.float32) / 255
    a = np.clip(a + w * 0.6, 0, 0.92)
    col = np.zeros((LH, LW, 4), np.float32)
    col[..., 0] = 90; col[..., 1] = 160; col[..., 2] = 250
    col[..., 3] = a * 255
    return Image.fromarray(col.astype(np.uint8), 'RGBA')

def link(length, width, face_on):
    """One silver link (RGBA), lying along x."""
    pad = 8
    im = Image.new('RGBA', (length + pad * 2, width + pad * 2), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    box = [pad, pad, pad + length, pad + width]
    d.ellipse(box, fill=(14, 17, 27, 255))
    t = max(int(width * 0.30), 5) if face_on else int(width * 0.55)
    inner = [box[0] + 3, box[1] + 3, box[2] - 3, box[3] - 3]
    d.ellipse(inner, fill=(190, 196, 208, 255))
    ih = inner[3] - inner[1]
    if ih > 8:
        d.ellipse([inner[0] + 2, inner[1] + 2, inner[2] - 2, max(inner[1] + ih * 0.45, inner[1] + 3)], fill=(238, 241, 247, 255))
        d.ellipse([inner[0] + 3, min(inner[3] - ih * 0.38, inner[3] - 4), inner[2] - 3, inner[3] - 3], fill=(112, 120, 135, 255))
    else:
        d.line([(inner[0] + 4, inner[1] + ih / 2), (inner[2] - 4, inner[1] + ih / 2)], fill=(238, 241, 247, 255), width=2)
    if face_on:
        hole = [inner[0] + t, inner[1] + t, inner[2] - t, inner[3] - t]
        d.ellipse(hole, fill=(8, 12, 30, 255))
        d.ellipse([hole[0] - 2, hole[1] - 2, hole[2] + 2, hole[3] + 2], outline=(14, 17, 27, 255), width=2)
    return im

def lay_chain(layer, p0, p1, sag=0.0, size=48):
    pts = bez(p0, ((p0[0] * 2 + p1[0]) / 3, (p0[1] * 2 + p1[1]) / 3 + sag), ((p0[0] + p1[0] * 2) / 3, (p0[1] + p1[1] * 2) / 3 + sag), p1, 200)
    # walk along the curve at even steps
    out = []; acc = 0.0; last = pts[0]; out.append((last, 0.0))
    step = size * 0.78
    for p in pts[1:]:
        acc += math.hypot(p[0] - last[0], p[1] - last[1]); last = p
        if acc >= step:
            out.append((p, 0.0)); acc = 0.0
    n = len(out)
    for i, (p, _) in enumerate(out):
        q = out[min(i + 1, n - 1)][0] if i + 1 < n else p
        o = out[i - 1][0] if i > 0 else p
        ang = math.degrees(math.atan2(q[1] - o[1], q[0] - o[0]))
        lk = link(size, int(size * 0.62), i % 2 == 0) if i % 2 == 0 else link(int(size * 0.92), int(size * 0.30), False)
        lk = lk.rotate(-ang, expand=True, resample=Image.BICUBIC)
        layer.alpha_composite(lk, (int(p[0] - lk.width / 2), int(p[1] - lk.height / 2)))

def chains_layer():
    """The iron on a monarch: a collar at the neck and four taut chains running to the corners of the panel."""
    layer = Image.new('RGBA', (LW, LH), (0, 0, 0, 0))
    nx, ny = CX, int(492 * SC)
    lugl, lugr = (nx - 78, ny + 6), (nx + 78, ny + 6)
    for tgt, lug in (((10, 6), lugl), ((LW - 10, 6), lugr), ((10, LH), lugl), ((LW - 10, LH), lugr)):
        lay_chain(layer, tgt, lug, sag=22)
    d = ImageDraw.Draw(layer)
    # the collar: a band round the neck
    d.ellipse([nx - 92, ny - 34, nx + 92, ny + 48], fill=(14, 17, 27, 255))
    d.ellipse([nx - 86, ny - 28, nx + 86, ny + 42], fill=(170, 176, 190, 255))
    d.ellipse([nx - 78, ny - 22, nx + 78, ny + 18], fill=(232, 236, 244, 255))
    d.ellipse([nx - 66, ny - 8, nx + 66, ny + 34], fill=(100, 108, 124, 255))
    d.rectangle([nx - 92, ny - 50, nx + 92, ny - 30], fill=(0, 0, 0, 0))
    # the lock at the front, with a soul-blue stone
    d.ellipse([nx - 24, ny + 4, nx + 24, ny + 52], fill=(14, 17, 27, 255))
    d.ellipse([nx - 19, ny + 9, nx + 19, ny + 47], fill=(196, 202, 214, 255))
    d.ellipse([nx - 11, ny + 17, nx + 11, ny + 39], fill=(110, 180, 255, 255))
    d.ellipse([nx - 6, ny + 20, nx + 2, ny + 28], fill=(236, 248, 255, 255))
    for lx, ly in (lugl, lugr):
        d.ellipse([lx - 18, ly - 18, lx + 18, ly + 18], fill=(14, 17, 27, 255))
        d.ellipse([lx - 13, ly - 13, lx + 13, ly + 13], fill=(196, 202, 214, 255))
        d.ellipse([lx - 6, ly - 6, lx + 6, ly + 6], fill=(14, 17, 27, 255))
    return layer

def court_card(rank, suit):
    img = plate()
    ink = new_mask(); d = ImageDraw.Draw(ink)
    frame_ink(ink, d)
    panel(ink, d, (0.275, 0.125, 0.725, 0.875))
    x0, y0 = int(W / 2 - LW / 2), int(H * 0.14)
    half, gems = build_court(rank, suit)
    # the figure is drawn a little small for its panel: draw it 1.18 times larger from the top middle, and let the robe run off the midline
    S = 1.18
    big = half.resize((int(LW * S), int(LH * S)), Image.LANCZOS)
    half = big.crop((int((LW * S - LW) / 2), 0, int((LW * S - LW) / 2) + LW, LH))
    gems = [((gx - CX) * S + CX, gy * S, r * S) for gx, gy, r in gems]
    if monarch_rank(rank):
        gems = [g for g in gems if g[2] < 20] + [(CX - 52 * S + 0, 352 * S, 11), (CX + 52 * S, 352 * S, 11)]
    ink.paste(255, (x0, y0), half)
    ink.paste(255, (x0, y0 + LH), half.rotate(180))
    # the line where the two halves meet, with a diamond at its middle
    d.line([(x0 - 10, y0 + LH), (x0 + LW + 10, y0 + LH)], fill=255, width=7)
    d.polygon([(W / 2 - 34, y0 + LH), (W / 2, y0 + LH - 34), (W / 2 + 34, y0 + LH), (W / 2, y0 + LH + 34)], fill=255)
    fine = new_mask(); fd = ImageDraw.Draw(fine)
    ground(fd, (W * 0.265, H * 0.135, W * 0.735, H * 0.865))
    lay_ink(img, fine, 0.22, relief=False)
    monarch = rank in 'QK'
    if monarch:
        glow = soul_glow()
        over(img, glow, (x0 + LW / 2, y0 + LH / 2))
        over(img, glow, (x0 + LW / 2, y0 + LH * 1.5), rotate180=True)
    lay_ink(img, ink)
    lay_rank(img, rank, suit)
    corner_pips(img, suit)
    if monarch:
        ch = chains_layer()
        over(img, ch, (x0 + LW / 2, y0 + LH / 2), shadow=0.9)
        over(img, ch, (x0 + LW / 2, y0 + LH * 1.5), rotate180=True, shadow=0.9)
    # gems (soul blue) on both halves
    for (gx, gy, r) in gems:
        for flip in (False, True):
            px, py = (x0 + gx, y0 + gy) if not flip else (x0 + LW - gx, y0 + LH * 2 - gy)
            g = soul_pip(DIAMOND, int(r * 2.6))
            over(img, g, (px, py))
    # the suit, polished, at the corner of each half
    over(img, enamel_pip(suit, 190), (x0 + 96, y0 + 112), shadow=1.0)
    over(img, enamel_pip(suit, 190), (x0 + LW - 96, y0 + LH * 2 - 112), rotate180=True, shadow=1.0)
    return finish(img)
