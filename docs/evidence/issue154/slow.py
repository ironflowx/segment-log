#!/usr/bin/env python3
"""Table rows of one cap directory of slow.sh: usage slow.py DIR."""
import sys,re,csv,statistics as st,glob,os
O=sys.argv[1]; R=64*2**20
def met(f,k):
    l=next(l for l in open(f) if l.startswith("METRIC "+k+" ")); return {a:int(b) for a,b in re.findall(r"(\w+)=(\d+)",l)}
def F(x): return "%.2f (%.2f-%.2f)"%(st.median(x),min(x),max(x))
for v in [l.split()[0] for l in open(O+"/binaries.txt")]:
    rows=[];pl={"first":[],"roll":[],"other":[]}
    for f in sorted(glob.glob(f"{O}/paced-{v}-*/log")):
        a=met(f,"append_ns"); b=1+2*met(f,"device_fdatasync_ns")["p99"]/1e6
        rows.append((a["p50"]/1e6,a["p99"]/1e6,a["p999"]/1e6,a["max"]/1e6,b,a["p99"]/1e6<=b))
        g=list(csv.DictReader(open(os.path.dirname(f)+".groups.tsv"),delimiter="\t")); m=st.median(int(r["submits"]) for r in g); pos=64;seg=0;t=dict(first=0,roll=0,other=0)
        for r in g:
            if pos>=R: seg+=1;pos=64
            x=pos/2**20;n=int(r["submits"])
            if n>=2*m: t["roll" if seg and x<3.5 else "first" if seg==0 and any(q-.5<=x<q+2 for q in(4,8,16,32)) else "other"]+=n
            pos+=int(r["frame_bytes"])
        for k in t: pl[k].append(t[k])
    print(f"{v:6} p50 {F([x[0] for x in rows])} p99 {F([x[1] for x in rows])} p999 {F([x[2] for x in rows])} max {F([x[3] for x in rows])} bound {F([x[4] for x in rows])} within {sum(x[5] for x in rows)}/{len(rows)} big {({k:(min(x),max(x)) for k,x in pl.items()})}")
