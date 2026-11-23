#!/usr/bin/env bash
# Issue 154: paced and closed-loop runs of several commits, interleaved: refs.sh <repo> <out> name=commit...
set -euo pipefail
repo=$1 out=$2; shift 2
mkdir -p "$out" /tmp/m1/data
[ ! -f "$HOME/.cargo/env" ] || . "$HOME/.cargo/env"
export CARGO_TARGET_DIR=/root/refs-build
cd "$repo"; names=()
for r in "$@"; do
  git checkout -qf "${r#*=}"; names+=("${r%%=*}")
  cp "$(cargo test -q -p segment-log-core --lib --release --no-run --message-format=json 2>/dev/null |
    python3 -c 'import json,sys; print(next(x["executable"] for x in map(json.loads,sys.stdin) if x.get("executable")))')" "/root/r-${r%%=*}"
  echo "${r%%=*} $(git rev-parse --short HEAD) $(sha256sum "/root/r-${r%%=*}" | cut -c1-16)" | tee -a "$out/binaries.txt"
done
{ lscpu; uname -r; ldd --version | head -1; findmnt /tmp; nvme list 2>/dev/null || true; cat /sys/block/nvme*/queue/write_cache 2>/dev/null || true; } > "$out/host.txt"
# mkfs leaves inode tables to a kernel thread that writes in the background.
for _ in $(seq 1 90); do pgrep ext4lazyinit >/dev/null || break; sleep 10; done
pgrep -c ext4lazyinit > "$out/lazyinit-left.txt" || true
export SEGMENT_LOG_EVIDENCE_DIR=/tmp/m1/data SEGMENT_LOG_BENCH_CALLERS=1000 SEGMENT_LOG_BENCH_ROUNDS=${ROUNDS:-1000} SEGMENT_LOG_DEVICE_ROUNDS=1000
run() {
  mkdir -p "$1"; uptime > "$1/load.txt"; python3 scripts/m1-quiet.py "$1/load.txt" >/dev/null
  env "${@:2}" /usr/bin/time -v "/root/r-$m" --exact acceptance::evidence::measured_appends_survive_reopen --nocapture > "$1/log" 2>&1
}
n=${#names[@]} P=SEGMENT_LOG_BENCH_INTERVAL_US=20000
for i in $(seq 1 "${REPS:-5}"); do for k in $(seq 0 $((n - 1))); do m=${names[$(((k + i - 1) % n))]}
  run "$out/paced-$m-$i" $P SEGMENT_LOG_GROUPS_OUT="$out/paced-$m-$i.groups.tsv"
  run "$out/closed-$m-$i" X=1
done; done
for i in $(seq 1 "${TRACED:-3}"); do for m in "${names[@]}"; do
  run "$out/traced-$m-$i" $P SEGMENT_LOG_PIPELINE_OUT="$out/traced-$m-$i/t.tsv"; rm -f "$out/traced-$m-$i/t.tsv"
done; done
echo "all done" > "$out/done.txt"
