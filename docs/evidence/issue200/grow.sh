#!/usr/bin/env bash
# Issue 200: grow.sh <repo> <out> base=commit fix=commit, as root on a 7502P with 32 cores, slowed as
# in issue 154's slow.sh: 24 cores, boost off, a top speed of CAP kHz. Interleaved runs of each commit:
#   plain   256-byte payloads       shrink  232 bytes from revision 226 on, so that the second
#           segment holds more keys (about 246,000) than its index was sized for (about 226,000)
set -euo pipefail
repo=$1 out=$2; shift 2
mkdir -p "$out" /tmp/m1/data
[ ! -f "$HOME/.cargo/env" ] || . "$HOME/.cargo/env"
export CARGO_TARGET_DIR=/root/refs-build
cd "$repo"; names=()
for r in "$@"; do
  git checkout -qf "${r#*=}"; names+=("${r%%=*}")
  for w in plain shrink; do
  [ $w = plain ] || python3 - <<'P'
p = 'segment-log-core/src/acceptance/evidence.rs'
s = open(p).read()
o = 'let mut payload = vec![0x5a; 256];'
assert s.count(o) == 1
open(p, 'w').write(s.replace(o, 'let mut payload = vec![0x5a; if revision < 226 { 256 } else { 232 }];'))
P
  cp "$(cargo test -q -p segment-log-core --lib --release --no-run --message-format=json 2>/dev/null |
    python3 -c 'import json,sys; print(next(x["executable"] for x in map(json.loads,sys.stdin) if x.get("executable")))')" "/root/r-${r%%=*}-$w"
  echo "${r%%=*}-$w $(git rev-parse --short HEAD) $(sha256sum "/root/r-${r%%=*}-$w" | cut -c1-16)" | tee -a "$out/binaries.txt"
  done
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
n=${#names[@]} P=SEGMENT_LOG_BENCH_INTERVAL_US=20000 C=/sys/devices/system/cpu
speed() { python3 -c 'import time; t=time.perf_counter(); x=sum(i*i for i in range(6000000)); print(round(time.perf_counter()-t,3))'; }
{ echo "as it comes: $(nproc) cpus, spin $(speed) s"; cat $C/cpu0/topology/thread_siblings_list $C/cpu0/cpufreq/scaling_available_frequencies $C/cpu0/cpufreq/scaling_driver 2>/dev/null || true; } >> "$out/slow.txt"
if [ -z "${DRY:-}" ]; then
  for c in $(seq 24 31) $(seq 56 63); do echo 0 > $C/cpu$c/online; done
  [ "$(nproc)" = 48 ]
  if [ -e $C/cpufreq/boost ]; then echo 0 > $C/cpufreq/boost; fi
fi
cap=${CAP:-1500000}
if [ -z "${DRY:-}" ]; then
  for c in $C/cpu[0-9]*; do [ "$(cat "$c/online" 2>/dev/null || echo 1)" = 0 ] || echo $cap > "$c/cpufreq/scaling_max_freq"; done
  [ "$(cat $C/cpu0/cpufreq/scaling_max_freq)" = $cap ]
fi
echo "$cap: $(nproc) cpus, boost $(cat $C/cpufreq/boost 2>/dev/null || echo none), spin $(speed) s" >> "$out/slow.txt"
paced() { run "$out/paced-$m-$i" $P SEGMENT_LOG_GROUPS_OUT="$out/paced-$m-$i.groups.tsv"; }
for i in $(seq 1 "${REPS:-5}"); do for k in $(seq 0 $((n - 1))); do
  v=${names[$(((k + i - 1) % n))]}
  m=$v-plain; paced; run "$out/closed-$m-$i" X=1
  m=$v-shrink; paced
done; done
for i in $(seq 1 "${TRACED:-3}"); do for v in "${names[@]}"; do for m in $v-plain $v-shrink; do
  run "$out/traced-$m-$i" $P SEGMENT_LOG_PIPELINE_OUT="$out/traced-$m-$i/t.tsv"; rm -f "$out/traced-$m-$i/t.tsv"
done; done; done
echo "all done" > "$out/done.txt"
