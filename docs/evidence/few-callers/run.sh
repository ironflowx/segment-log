#!/usr/bin/env bash
# Few-caller closed-loop appends.
#
# The harness is acceptance::evidence::measured_appends_survive_reopen, the same
# test scripts/m1-evidence-guest.sh runs for the issue 49 capture. Caller count
# is SEGMENT_LOG_BENCH_CALLERS, which that test already reads (default 4).
# This script only sets it. It does not change options: default linger, the
# group's fdatasync before the acknowledgment, 256-byte payloads, one record
# per submit, no paced interval.
#
# Each measured run appends 1,000,000 records, the issue 49 closed-loop count.
# Rounds are 1,000,000 / callers. Scratch output stays outside the repository;
# set FEW_CALLERS_OUT to move it.
set -euo pipefail
cd "$(dirname "$0")/../../.."
out="${FEW_CALLERS_OUT:-/tmp/few-callers}"
data="$out/data"
rm -rf "$out"
mkdir -p "$data"
unset SEGMENT_LOG_BENCH_INTERVAL_US SEGMENT_LOG_BENCH_STORAGE SEGMENT_LOG_TEST_ROLL_BYTES \
    SEGMENT_LOG_BENCH_CPU SEGMENT_LOG_PIPELINE_OUT SEGMENT_LOG_GROUPS_OUT || true

{
    date -u
    echo "commit $(git rev-parse HEAD)"
    git status --porcelain
    rustc -Vv
    uname -a
    cat /etc/os-release
    echo "virt $(systemd-detect-virt 2>/dev/null || true)"
    for f in sys_vendor product_name product_version; do
        echo "dmi_$f $(cat /sys/class/dmi/id/$f 2>/dev/null || true)"
    done
    nproc
    lscpu
    free -b
    df -T "$data"
    findmnt -T "$data"
    lsblk -o NAME,TYPE,SIZE,ROTA,MODEL,TRAN,MOUNTPOINTS
    echo "--- queue vdc ---"
    for f in write_cache rotational scheduler fua logical_block_size physical_block_size; do
        echo "$f $(cat /sys/block/vdc/queue/$f)"
    done
    echo "virtio_features $(cat /sys/block/vdc/device/features)"
    uptime
} > "$out/host.txt"

# Same filesystem as the runs. A flush the guest ignores returns in about the
# time of a clean fdatasync; a flush it waits for does not.
python3 - << 'PY' > "$out/flush-check.txt"
import os, time
path = "/tmp/few-callers/flush-check.bin"
def trial(sync, n, size):
    fd = os.open(path, os.O_CREAT | os.O_RDWR | os.O_TRUNC, 0o644)
    buf = b"\x5a" * size
    times = []
    for _ in range(n):
        t0 = time.perf_counter_ns()
        os.write(fd, buf)
        if sync:
            os.fdatasync(fd)
        times.append(time.perf_counter_ns() - t0)
    t0 = time.perf_counter_ns()
    os.fdatasync(fd)
    clean = time.perf_counter_ns() - t0
    os.close(fd)
    times.sort()
    def q(num, den):
        return times[(len(times) * num + den - 1) // den - 1]
    print(f"sync={sync} size={size} n={n} min={times[0]} p50={q(50,100)} p99={q(99,100)} max={times[-1]} clean_fdatasync_ns={clean}")
trial(False, 200, 4096)
trial(True, 200, 4096)
trial(True, 100, 256 * 1024)
os.unlink(path)
PY

cargo test -p segment-log-core --lib --release --no-run --message-format=json > "$out/build.jsonl"
binary="$(python3 -c 'import json,sys; print(next(x["executable"] for x in map(json.loads,open(sys.argv[1])) if x.get("executable")))' "$out/build.jsonl")"
sha256sum "$binary" | tee "$out/binary.sha256"

rounds_for() {
    case "$1" in
        1) echo 1000000 ;;
        4) echo 250000 ;;
        16) echo 62500 ;;
        1000) echo 1000 ;;
        *) echo "unknown callers $1" >&2; exit 1 ;;
    esac
}
warmup_rounds_for() {
    case "$1" in
        1) echo 1000 ;;
        4) echo 1000 ;;
        16) echo 400 ;;
        1000) echo 20 ;;
    esac
}

run_one() {
    local callers="$1" rounds="$2" tag="$3" device_rounds="$4"
    python3 scripts/m1-quiet.py "$out/$tag.load"
    echo "start $tag callers=$callers rounds=$rounds device_rounds=$device_rounds $(date -u +%Y-%m-%dT%H:%M:%SZ)" | tee -a "$out/progress.txt"
    SEGMENT_LOG_EVIDENCE_DIR="$data" \
        SEGMENT_LOG_BENCH_CALLERS="$callers" \
        SEGMENT_LOG_BENCH_ROUNDS="$rounds" \
        SEGMENT_LOG_DEVICE_ROUNDS="$device_rounds" \
        SEGMENT_LOG_BENCH_DEADLINE_S=7200 \
        "$binary" --exact acceptance::evidence::measured_appends_survive_reopen --nocapture \
        > "$out/$tag.log" 2> "$out/$tag.err"
    echo "end $tag $(date -u +%Y-%m-%dT%H:%M:%SZ)" | tee -a "$out/progress.txt"
}

for callers in 1 4 16 1000; do
    run_one "$callers" "$(warmup_rounds_for "$callers")" "warmup-c$callers" 8
done
for run in 1 2 3; do
    for callers in 1 4 16 1000; do
        run_one "$callers" "$(rounds_for "$callers")" "c${callers}-run${run}" 1000
    done
done
echo "all done $(date -u +%Y-%m-%dT%H:%M:%SZ)" | tee -a "$out/progress.txt"
