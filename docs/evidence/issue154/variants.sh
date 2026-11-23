#!/usr/bin/env bash
# Issue 154: which part of the work after a closing run stalls the writer? Run as root on an EPYC
# auction box after `scripts/m7-session.sh bootstrap`: variants.sh <repo> <out>.
#   orig      115a60b as it is          leak  the rolled segment's key map is never freed
#   nofilter  the log filter is off     both  leak and nofilter
# REPS rounds of one paced run of each variant, then TRACED rounds with the pipeline trace.
set -euo pipefail
repo=$1 out=$2
mkdir -p "$out" /tmp/m1/data
[ ! -f "$HOME/.cargo/env" ] || . "$HOME/.cargo/env"
export CARGO_TARGET_DIR=/root/variants-build
cd "$repo"
bld() { cargo test -q -p segment-log-core --lib --release --no-run --message-format=json 2>/dev/null |
  python3 -c 'import json,sys; print(next(x["executable"] for x in map(json.loads,sys.stdin) if x.get("executable")))'; }
swap() { python3 - "$@" <<'P'
import sys
p, a, b = sys.argv[1:]
s = open(p).read()
assert s.count(a) == 1, (p, a, s.count(a))
open(p, 'w').write(s.replace(a, b))
P
}
W=segment-log-core/src/engine/writer.rs E=segment-log-core/src/acceptance/evidence.rs
free='.spawn(move || drop(keys));' keep='.spawn(move || std::mem::forget(keys));'
on='log_filter: crate::engine::LogFilterMode::Background,' off='log_filter: crate::engine::LogFilterMode::Off,'
cp "$(bld)" /root/v-orig
swap $W "$free" "$keep"; cp "$(bld)" /root/v-leak
swap $E "$on" "$off"; cp "$(bld)" /root/v-both
swap $W "$keep" "$free"; cp "$(bld)" /root/v-nofilter
swap $E "$off" "$on"
sha256sum /root/v-* | tee "$out/binaries.txt"
[ "$(cut -d' ' -f1 "$out/binaries.txt" | sort -u | wc -l)" = 4 ]
{ lscpu; uname -r; ldd --version | head -1; findmnt /tmp; nvme list 2>/dev/null || true
  cat /sys/block/nvme*/queue/write_cache 2>/dev/null || true; } > "$out/host.txt"
# mkfs leaves inode tables to a kernel thread that writes in the background.
for _ in $(seq 1 60); do pgrep ext4lazyinit >/dev/null || break; sleep 10; done
pgrep -c ext4lazyinit > "$out/lazyinit-left.txt" || true
export SEGMENT_LOG_EVIDENCE_DIR=/tmp/m1/data SEGMENT_LOG_BENCH_CALLERS=1000
export SEGMENT_LOG_BENCH_ROUNDS=${ROUNDS:-1000} SEGMENT_LOG_DEVICE_ROUNDS=1000 SEGMENT_LOG_BENCH_INTERVAL_US=20000
run() { # <dir> [env...]
  mkdir -p "$1"; uptime > "$1/load.txt"; python3 scripts/m1-quiet.py "$1/load.txt" >/dev/null
  env "${@:2}" "/root/v-$m" --exact acceptance::evidence::measured_appends_survive_reopen --nocapture > "$1/log" 2>&1
}
order=(orig leak nofilter both)
for i in $(seq 1 "${REPS:-5}"); do
  for k in 0 1 2 3; do m=${order[$(((k + i - 1) % 4))]}; run "$out/paced-$m-$i" SEGMENT_LOG_GROUPS_OUT="$out/paced-$m-$i.groups.tsv"; done
done
for i in $(seq 1 "${TRACED:-3}"); do for m in "${order[@]}"; do
  run "$out/traced-$m-$i" SEGMENT_LOG_PIPELINE_OUT="$out/traced-$m-$i/t.tsv"; rm -f "$out/traced-$m-$i/t.tsv"
done; done
echo "all done" > "$out/done.txt"
