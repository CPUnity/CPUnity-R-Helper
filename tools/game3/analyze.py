import numpy as np, wave, json, sys
from scipy.signal import stft
from scipy.ndimage import gaussian_filter1d
name=sys.argv[1]
w=wave.open(name+'.wav'); sr=w.getframerate(); x=np.frombuffer(w.readframes(w.getnframes()),np.int16).astype(np.float32)/32768
dur=len(x)/sr; hop=256;n=1024
f,t,Z=stft(x,sr,nperseg=n,noverlap=n-hop,window='hann',padded=False); fs=sr/hop
L=np.log1p(np.abs(Z)*30)
def band(a,b):
    sel=(f>=a)&(f<b); e=L[sel].mean(0); return np.maximum(0,np.diff(e,prepend=e[0]))
E={'low':band(30,150),'mid':band(150,2000),'high':band(2000,9000)}
tot=sum(E[k]/(E[k].std()+1e-9) for k in E); env=gaussian_filter1d(tot,1.0); tt=t
res=[]
for bpm in np.arange(70,181,.1):
    p=60/bpm; best=(0,0)
    for ph in np.arange(0,p,.01):
        v=np.interp(np.arange(ph,dur-1,p),tt,env).mean()
        if v>best[0]: best=(v,ph)
    res.append((best[0],bpm,best[1]))
res.sort(reverse=True)
print('dur',round(dur,2)); [print(round(v,3),round(b,1),round(ph,3)) for v,b,ph in res[:6]]
np.savez(name+'_E.npz',t=t,low=E['low'],mid=E['mid'],high=E['high'])
x2=x
bpm,ph=res[0][1],res[0][2]
bar=60/bpm*4
rms=[float(np.sqrt((x2[int(sr*(ph+b*bar)):int(sr*(ph+(b+1)*bar))]**2).mean())) for b in range(int((dur-ph)/bar))]
print('bars',len(rms),' '.join(f'{v:.2f}' for v in rms))
json.dump({'bpm':float(bpm),'ph':float(ph),'dur':float(dur),'rms':rms},open(name+'_meta.json','w'))
