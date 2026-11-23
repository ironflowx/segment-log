#!/usr/bin/env python3
"""Writer stalls in the 60 ms after each finished closing run, beside a control window 200-260 ms
before it: usage arena.py GLOB..., where each match holds t.groups.tsv and t.events.tsv of a traced run."""
import csv,sys,glob,collections
def load(d):
    R=lambda f:list(csv.DictReader(open(f"{d}/t.{f}.tsv"),delimiter="\t"))
    return [e for e in R("events") if e["event"]=="pending"],[{k:int(v) for k,v in r.items()} for r in R("groups")]
def win(g,lo,hi):
    w=[x for x in g if lo<=x["formed"]<=hi]
    big=sum(x["count"] for x in w if x["count"]>=102)
    gap=max([(b["written"]-a["written"])/1e6 for a,b in zip(w,w[1:])]+[0])
    enc=max([(x["encoded"]-x["formed"])/1e6 for x in w]+[0])
    return gap,enc,big
for pat in sys.argv[1:]:
  A=collections.defaultdict(list)
  for d in sorted(glob.glob(pat)):
    P,g=load(d)
    for i,e in enumerate(P):
        t=int(e["end"])
        for name,lo,hi in(("after +0..60ms",t,t+60e6),("control -260..-200ms",t-260e6,t-200e6)):
            A[name].append(win(g,lo,hi))
  for name,v in A.items():
    print(f"{pat.split('/')[-1]:14} {name:21} n={len(v)} max-gap ms {[round(x[0],1) for x in v]} max-encode ms {[round(x[1],1) for x in v]} records in big groups {sum(x[2] for x in v)//len(v)}/roll")
