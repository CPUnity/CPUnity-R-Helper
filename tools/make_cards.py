"""Renders the 52 faces and the back of the Devil's Table cards into ../assets/cards (run from tools/)."""
import sys, os, io
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from multiprocessing import Pool
from courts import *
from cardback import build_back

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'assets', 'cards')
RANK_LABELS = ['2', '3', '4', '5', '6', '7', '8', '9', '10', 'J', 'Q', 'K', 'A']

def save(im, name):
    p = im.quantize(colors=256, method=Image.FASTOCTREE, dither=Image.FLOYDSTEINBERG)
    p.save(os.path.join(OUT, name), 'PNG', optimize=True)

def job(idx):
    if idx >= 100:
        glint_mask(idx - 100).save(os.path.join(OUT, 'glint%d.png' % (idx - 100)), optimize=True)
        return idx
    if idx == 52:
        save(build_back(), 'back.png')
        return idx
    suit, rank = divmod(idx, 13)
    label = RANK_LABELS[rank]
    im = court_card(label, suit) if label in 'JQK' else pips_card(label, suit)
    save(im, 'c%02d.png' % idx)
    return idx

if __name__ == '__main__':
    os.makedirs(OUT, exist_ok=True)
    todo = [int(a) for a in sys.argv[1:]] or list(range(53))
    with Pool(2) as pool:
        for i in pool.imap_unordered(job, todo):
            print('done', i, flush=True)
