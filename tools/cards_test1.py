import sys; sys.path.insert(0,'.')
from cardlib import *
img = plate()
ink = new_mask(); d = ImageDraw.Draw(ink)
# frames and the guilloche band
for ins, w in ((26, 7), (150, 5)):
    poly(d, [(x, y) for x, y, _, _ in rrect_path(ins)], w, closed=True)
guilloche(d, 88, 14, 150, 5)
lay_ink(img, ink)
# a few pips
over(img, polished_pip(HEART, 360), (W/2, H/2))
over(img, soul_pip(HEART, 150), (290, 560))
over(img, soul_pip(SPADE, 150), (290, 900))
over(img, soul_pip(DIAMOND, 150), (290, 1200))
over(img, soul_pip(CLUB, 150), (290, 1500))
finish(img).save('/tmp/claude-0/-home-claude/c4a85d64-9404-57d6-9bf0-b6a97e150cba/scratchpad/cardprev/t1.png')
sheet = Image.new('RGBA', (4*300, 300), (30,30,34,255))
for i,s in enumerate((CLUB,DIAMOND,HEART,SPADE)):
    p = polished_pip(s, 280)
    sheet.alpha_composite(p, (i*300+10, 10))
sheet.save('/tmp/claude-0/-home-claude/c4a85d64-9404-57d6-9bf0-b6a97e150cba/scratchpad/cardprev/pips.png')
