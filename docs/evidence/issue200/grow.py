#!/usr/bin/env python3
"""Table rows of a grow.sh run: usage grow.py DIR."""
import csv, glob, os, re, statistics as st, sys

O = sys.argv[1]
F = lambda x: "%.2f (%.2f-%.2f)" % (st.median(x), min(x), max(x)) if x else "-"


def metric(f, k):
    line = next(l for l in open(f) if l.startswith("METRIC " + k + " "))
    return {a: int(b) for a, b in re.findall(r"(\w+)=(\d+)", line)}


for v in [l.split()[0] for l in open(O + "/binaries.txt")]:
    a = [(metric(f, "append_ns"), 1 + 2 * metric(f, "device_fdatasync_ns")["p99"] / 1e6)
         for f in sorted(glob.glob(f"{O}/paced-{v}-*/log"))]
    c = [open(f).read() for f in sorted(glob.glob(f"{O}/closed-{v}-*/log"))]
    rate = [float(re.search(r"^APPEND .*records_per_second=([\d.]+)", t, re.M)[1]) for t in c]
    rss = [int(re.search(r"Maximum resident set size.*: (\d+)", t)[1]) / 1024 for t in c]
    print(v, *(f"{k} {F([m[k] / 1e6 for m, _ in a])}" for k in ("p50", "p99", "p999", "max")),
          f"bound {F([b for _, b in a])} within {sum(m['p99'] / 1e6 <= b for m, b in a)}/{len(a)}",
          f"closed {F(rate)} rss {F(rss)}")
    for d in sorted(glob.glob(f"{O}/traced-{v}-*")):
        read = lambda n: list(csv.DictReader(open(f"{d}/t.{n}.tsv"), delimiter="\t"))
        g = [{k: int(x) for k, x in r.items()} for r in read("groups")]
        rolls = [int(e["start"]) for e in read("events") if e["event"] == "roll"]
        # A segment's first record: the first group formed after its roll.
        firsts = [1] + [next((x["group"] for x in g if x["formed"] > t), 10**9) for t in rolls]
        out = []
        for want in (14336, 28672, 57344, 114688, 229376):
            for s, first in enumerate(firsts):
                if s > 1 or (s == 1) != (want == 229376):
                    continue
                near = [((y["written"] - x["written"]) / 1e6) for x, y in zip(g, g[1:])
                        if abs(y["group"] - first - want) < 1500]
                out.append(f"s{s}@{want}: {max(near):.1f}" if near else f"s{s}@{want}: none")
        gaps = sorted(((y["written"] - x["written"]) / 1e6, y["group"]) for x, y in zip(g, g[1:]))[-4:]
        print("  ", os.path.basename(d), "longest gap ms near", out, "| segments start", firsts[:4],
              "| longest gaps", [(round(t, 1), r) for t, r in gaps[::-1]])
