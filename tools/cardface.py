import sys; sys.path.insert(0, '.')
from cardlib import *
import cardlib

PIPCOLS = (0.37, 0.5, 0.63)
PIP_SIZE = 285
LAYOUT = {
    'A': [(1, .5)],
    '2': [(1, 0), (1, 1)],
    '3': [(1, 0), (1, .5), (1, 1)],
    '4': [(0, 0), (2, 0), (0, 1), (2, 1)],
    '5': [(0, 0), (2, 0), (0, 1), (2, 1), (1, .5)],
    '6': [(0, 0), (2, 0), (0, .5), (2, .5), (0, 1), (2, 1)],
    '7': [(0, 0), (2, 0), (0, .5), (2, .5), (0, 1), (2, 1), (1, .25)],
    '8': [(0, 0), (2, 0), (0, .5), (2, .5), (0, 1), (2, 1), (1, .25), (1, .75)],
    '9': [(0, 0), (2, 0), (0, 1/3), (2, 1/3), (0, 2/3), (2, 2/3), (0, 1), (2, 1), (1, .5)],
    '10': [(0, 0), (2, 0), (0, 1/3), (2, 1/3), (0, 2/3), (2, 2/3), (0, 1), (2, 1), (1, 1/6), (1, 5/6)],
}
Y0, Y1 = 0.215, 0.785

def rank_text_mask(label, size=300):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    f = ImageFont.truetype(SERIF, size)
    return m, d, f

RANK_X, RANK_Y, RANK_SIZE = 0.186, 0.118, 205
PIP_Y = 0.196

def draw_rank(ink, label, cx, cy, flip=False, size=RANK_SIZE):
    layer = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(layer)
    f = ImageFont.truetype(SERIF, size)
    # tight tracking for "10"
    glyphs = list(label)
    widths = [d.textlength(g, font=f) for g in glyphs]
    track = -size * 0.07 if len(glyphs) > 1 else 0
    total = sum(widths) + track * (len(glyphs) - 1)
    x = cx - total / 2
    for g, wd in zip(glyphs, widths):
        d.text((x, cy), g, font=f, fill=255, anchor='lm')
        x += wd + track
    if flip:
        layer = layer.rotate(180)
    ink.paste(255, mask=layer)

def frame_ink(ink, d, inner=True):
    for ins, w in ((24, 8), (150, 6)):
        poly(d, [(x, y) for x, y, _, _ in rrect_path(ins)], w, closed=True)
    guilloche(d, 87, 15, 150, 5)
    # four small diamonds on the band's mid-lines
    for (x, y) in ((W / 2, 87), (W / 2, H - 87), (87, H / 2), (W - 87, H / 2)):
        d.polygon([(x - 26, y), (x, y - 26), (x + 26, y), (x, y + 26)], fill=255)

def fine_field(ink_d, box, step=14, w=1):
    """Very fine hairlines laid across the field, like the ground of an engraving."""
    x0, y0, x1, y1 = box
    y = y0
    while y < y1:
        ink_d.line([(x0, y), (x1, y)], fill=60, width=w)
        y += step

def build_face(rank_label, suit, court=None):
    img = plate()
    ink = new_mask(); d = ImageDraw.Draw(ink)
    frame_ink(ink, d)
    return img, ink, d

WELL_Y = (0.222, 0.778)
WELL = 330
CENTRE = 760

def lay_rank(img, label, suit):
    rk = new_mask()
    draw_rank(rk, label, W * RANK_X, H * RANK_Y)
    draw_rank(rk, label, W * RANK_X, H * RANK_Y, flip=True)
    lay_ink(img, rk, col=suit_ink(suit))

def corner_pips(img, suit):
    for flip in (False, True):
        cx = W * RANK_X if not flip else W - W * RANK_X
        cy = H * PIP_Y if not flip else H - H * PIP_Y
        over(img, enamel_pip(suit, 130), (cx, cy), rotate180=flip)

def pips_card(rank_label, suit):
    img, ink, d = build_face(rank_label, suit)
    panel(ink, d)
    fine = new_mask(); fd = ImageDraw.Draw(fine)
    ground(fd, (W * 0.275, H * 0.135, W * 0.725, H * 0.865))
    lay_ink(img, fine, 0.30, relief=False)
    c = (W / 2, H / 2)
    med = new_mask(); md = ImageDraw.Draw(med)
    rosette(md, c, 340, 470, 22)
    for r_, w in ((482, 5), (330, 3)):
        md.ellipse([c[0] - r_, c[1] - r_, c[0] + r_, c[1] + r_], outline=255, width=w)
    beads(md, c, 500, 80, 5)
    lay_ink(img, med, 0.8)
    lay_ink(img, ink)
    lay_rank(img, rank_label, suit)
    corner_pips(img, suit)
    # the two wells the souls rise from, the big pip between them
    for i, yy in enumerate(WELL_Y):
        over(img, soul_well(suit, WELL), (W / 2, yy * H), rotate180=(i == 1), shadow=0.8)
    over(img, chrome_pip(suit, CENTRE), (W / 2 - 8, H / 2 - 8), shadow=1.0)
    return finish(img)


def panel(ink, d, box=(0.275, 0.125, 0.725, 0.875)):
    """The ruled panel round the middle of the card: a double line with notched corners."""
    x0, y0, x1, y1 = box[0] * W, box[1] * H, box[2] * W, box[3] * H
    n = 46
    for ins, w in ((0, 6), (20, 3)):
        a, b, c_, d_ = x0 + ins, y0 + ins, x1 - ins, y1 - ins
        n2 = n - ins * 0.6
        pts = [(a + n2, b), (c_ - n2, b), (c_, b + n2), (c_, d_ - n2), (c_ - n2, d_), (a + n2, d_), (a, d_ - n2), (a, b + n2)]
        d.line(pts + [pts[0]], fill=255, width=w, joint='curve')
    for (x, y) in ((x0, y0), (x1, y0), (x0, y1), (x1, y1)):
        d.ellipse([x - 9, y - 9, x + 9, y + 9], fill=255)

def ground(d, box):
    """The ground of an engraving: fine crossing lines, laid close, in the panel (kept faint by the caller)."""
    x0, y0, x1, y1 = box
    step = 22
    k = int((x1 - x0 + y1 - y0) / step) + 2
    for i in range(k):
        o = i * step
        d.line([(x0 + o, y0), (x0 + o - (y1 - y0), y1)], fill=255, width=2)
        d.line([(x0 + o - (y1 - y0), y0), (x0 + o, y1)], fill=255, width=2)

def rosette(d, c, r0, r1, n):
    """A ring of pointed petals between radius r0 and r1."""
    for i in range(n):
        a = 2 * math.pi * i / n
        da = math.pi / n * 0.85
        pts = []
        for t in range(0, 21):
            u = t / 20
            rad = r0 + (r1 - r0) * u
            wd = math.sin(math.pi * u) ** 0.8 * da
            pts.append((c[0] + rad * math.cos(a - wd), c[1] + rad * math.sin(a - wd)))
        for t in range(20, -1, -1):
            u = t / 20
            rad = r0 + (r1 - r0) * u
            wd = math.sin(math.pi * u) ** 0.8 * da
            pts.append((c[0] + rad * math.cos(a + wd), c[1] + rad * math.sin(a + wd)))
        d.line(pts + [pts[0]], fill=255, width=4, joint='curve')
        # a vein down the middle
        d.line([(c[0] + (r0 + 20) * math.cos(a), c[1] + (r0 + 20) * math.sin(a)), (c[0] + (r1 - 16) * math.cos(a), c[1] + (r1 - 16) * math.sin(a))], fill=255, width=3)

def beads(d, c, r, n, size):
    for i in range(n):
        a = 2 * math.pi * i / n
        x, y = c[0] + r * math.cos(a), c[1] + r * math.sin(a)
        d.ellipse([x - size, y - size, x + size, y + size], fill=255)
