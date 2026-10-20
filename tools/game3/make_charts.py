import numpy as np, json
TRACKS={'track':dict(bpm=124.0,ph=0.02,dur=99.24),'track2':dict(bpm=140.0,ph=0.04,dur=156.0)}
def feats(name,bpm,ph,dur):
    d=np.load(name+'_E.npz'); tt=d['t']; beat=60/bpm; slot=beat/4
    n=int((dur-ph-0.3)/slot)
    def at(env,time,r=.035):
        m=(tt>=time-r)&(tt<=time+r); return env[m].max() if m.any() else 0
    S={k:np.array([at(d[k],ph+i*slot) for i in range(n)]) for k in ('low','mid','high')}
    for k in S: S[k]=S[k]/np.percentile(S[k],95)
    # loudness per bar from the energy envelope
    bar=16; e=np.array([ (S['low'][b*bar:(b+1)*bar].mean()+S['mid'][b*bar:(b+1)*bar].mean()+S['high'][b*bar:(b+1)*bar].mean()) for b in range(n//bar)])
    return S,e,n,slot
CFG={
 'easy':  dict(lanes=4,grid=(0,2),thr=dict(low=(.38,.6,9),mid=(.45,.62,9),high=(.55,.8,9)),gap=3,cap=1),
 'normal':dict(lanes=6,grid=(0,1,2,3),thr=dict(low=(.3,.45,.95),mid=(.38,.5,.85),high=(.5,.6,.95)),gap=2,cap=2),
 'hard':  dict(lanes=8,grid=(0,1,2,3),thr=dict(low=(.22,.32,.55),mid=(.28,.38,.55),high=(.32,.42,.62)),gap=2,cap=3),
}
def lane_sets(diff):
    if diff=='easy':   return dict(low=[0,1],mid=[2,3],high=[3,2])
    if diff=='normal': return dict(low=[1,2,0],mid=[3,4,5],high=[5,0,4,1,3,2])
    return dict(low=[0,1,2,3],mid=[3,4,2,5],high=[5,6,7,4])
def make(name,bpm,ph,dur,diff):
    S,e,n,slot=feats(name,bpm,ph,dur); c=CFG[diff]; L=lane_sets(diff)
    use=[0]*c['lanes']; med=np.median(e); free=[-99]*c['lanes']; cnt={'low':0,'mid':0,'high':0}
    out=[]; last_lane=-1
    for i in range(n):
        b=i//16; j=i%4
        if j not in c['grid']: continue
        calm=b<len(e) and e[b]<0.72*med
        cand=[]
        for band in ('low','mid','high'):
            tb,t8,t16=c['thr'][band]
            thr=tb if j==0 else t8 if j==2 else t16
            if calm: thr=thr*1.35 if j in(0,2) else 99
            v=S[band][i]
            if v>thr: cand.append((v/thr,band))
        cand.sort(reverse=True)
        placed=[]
        for score,band in cand:
            if len(placed)>=(2 if (diff!='easy' and (i%16==0 or score>1.5)) else 1): break
            ls=L[band]; lane=None
            order=[ls[(cnt[band]+k)%len(ls)] for k in range(len(ls))]
            if diff=='hard': order=sorted(order,key=lambda q:use[q])
            for l in order:
                if i-free[l]>=c['gap'] and l not in [p[1] for p in placed] and not (diff=='hard' and abs(l-last_lane)==0):
                    lane=l;break
            if lane is None: continue
            cnt[band]+=1
            placed.append((score,lane,band))
        for s,l,bd in placed:
            free[l]=i; last_lane=l; use[l]+=1; out.append((i,l,s))
    # cap per beat: keep the strongest
    keep=[]; 
    byb={}
    for it in out: byb.setdefault(it[0]//4,[]).append(it)
    for bidx,items in sorted(byb.items()):
        items.sort(key=lambda z:-z[2]); keep+=items[:c['cap']]
    keep.sort()
    return [[round(ph+i*slot,3),l] for i,l,s in keep]
res={}
for name,p in TRACKS.items():
    res[name]={'bpm':p['bpm'],'off':p['ph'],'len':p['dur']}
    for diff in CFG:
        ch=make(name,p['bpm'],p['ph'],p['dur'],diff); res[name][diff]=ch
        lc=np.bincount([l for _,l in ch],minlength=CFG[diff]['lanes'])
        print(name,diff,len(ch),'notes',round(len(ch)/p['dur'],2),'/s lanes',lc.tolist())
json.dump(res,open('charts.json','w'),separators=(',',':'))
