#!/usr/bin/env bash
# Issue 206: paired.sh <repo> <out> <base> <head>, as root on a dedicated EPYC host after
# `scripts/m7-session.sh bootstrap` (Debian 12, /tmp on the second NVMe drive as ext4).
# Slows the host as issue 154's slow.sh and issue 200's grow.sh do: 24 cores, boost off, a top
# speed of CAP kHz. Waits until ext4lazyinit is done, then runs scripts/m1-compare-refs.sh: it builds
# both commits and runs five alternating pairs of the closed loop and five paced at 50,000 records/s,
# the measures ADR 0072's rule names. DRY=1 skips the slowing.
set -euo pipefail
repo=$1 out=$2 base=$3 head=$4
mkdir -p "$out"
out=$(cd "$out" && pwd)
[ ! -f "$HOME/.cargo/env" ] || . "$HOME/.cargo/env"
cd "$repo"
git fetch -q origin main simulation-api
git checkout -qf "$head"
C=/sys/devices/system/cpu
speed() { python3 -c 'import time; t=time.perf_counter(); x=sum(i*i for i in range(6000000)); print(round(time.perf_counter()-t,3))'; }
{ echo "as it comes: $(nproc) cpus, spin $(speed) s"; cat $C/cpu0/cpufreq/scaling_available_frequencies $C/cpu0/cpufreq/scaling_driver 2>/dev/null || true; } > "$out/slow.txt"
if [ -z "${DRY:-}" ]; then
  # Keep the 24 lowest core ids, which need not be contiguous; read every id before any CPU goes
  # offline, because an offline CPU loses its topology files.
  keep=" $(cat $C/cpu[0-9]*/topology/core_id | sort -nu | head -n 24 | tr '\n' ' ')"
  for c in $C/cpu[0-9]*; do
    case "$keep" in *" $(cat "$c/topology/core_id") "*) ;; *) echo 0 > "$c/online" ;; esac
  done
  [ "$(nproc)" -le 48 ]
  if [ -e $C/cpufreq/boost ]; then echo 0 > $C/cpufreq/boost; fi
  cap=${CAP:-1500000}
  for c in $C/cpu[0-9]*; do
    [ "$(cat "$c/online" 2>/dev/null || echo 1)" = 0 ] || echo "$cap" > "$c/cpufreq/scaling_max_freq"
  done
  [ "$(cat $C/cpu0/cpufreq/scaling_max_freq)" = "$cap" ]
fi
echo "slowed: $(nproc) cpus, boost $(cat $C/cpufreq/boost 2>/dev/null || echo none), max $(cat $C/cpu0/cpufreq/scaling_max_freq 2>/dev/null || echo unknown) kHz, spin $(speed) s" >> "$out/slow.txt"
{ lscpu; uname -r; findmnt /tmp; nvme list 2>/dev/null || true; for d in /sys/block/nvme*; do echo "$d $(cat "$d/device/firmware_rev" 2>/dev/null) $(cat "$d/queue/write_cache" 2>/dev/null)"; done; } > "$out/host.txt"
# mkfs leaves inode tables to a kernel thread that writes in the background.
for _ in $(seq 1 90); do pgrep ext4lazyinit >/dev/null || break; sleep 10; done
pgrep -c ext4lazyinit > "$out/lazyinit-left.txt" || true
SEGMENT_LOG_EVIDENCE_DIR=/tmp/issue206-data scripts/m1-compare-refs.sh "$base" "$head" "$out/compare" > "$out/compare.log" 2>&1
echo "all done" > "$out/done.txt"
