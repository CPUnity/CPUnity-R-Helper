from cardface import *
cards=[pips_card('A',SPADE), pips_card('7',HEART), pips_card('10',DIAMOND), pips_card('5',CLUB)]
sheet=Image.new('RGBA',(4*394,548),(28,22,26,255))
for i,c in enumerate(cards): sheet.alpha_composite(c,(i*394+5,5))
sheet.save('/tmp/claude-0/-home-claude/c4a85d64-9404-57d6-9bf0-b6a97e150cba/scratchpad/cardprev/t2.png')
