from courts import *
cards=[court_card('K',SPADE), court_card('Q',HEART), court_card('J',DIAMOND)]
sheet=Image.new('RGBA',(3*394,548),(28,22,26,255))
for i,c in enumerate(cards): sheet.alpha_composite(c,(i*394+5,5))
sheet.save('/tmp/claude-0/-home-claude/c4a85d64-9404-57d6-9bf0-b6a97e150cba/scratchpad/cardprev/t3.png')
for n,c in zip('KQJ',cards): c.save('/tmp/claude-0/-home-claude/c4a85d64-9404-57d6-9bf0-b6a97e150cba/scratchpad/cardprev/c_'+n+'.png')
