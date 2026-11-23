#!/usr/bin/env bash
# Issue 154: does freeing the old key map stall the writer inside glibc malloc? Runs in a container:
#   docker run --rm --pids-limit 4096 --tmpfs /data:size=4g -v "$PWD":/src:ro -v OUT:/out \
#     rust:1.100.0-bookworm bash /src/docs/evidence/issue154/arena.sh
# orig: the code as it is. mxfast0: glibc without fastbins. leak: the old map is never freed.
# ROUNDS, ROLL and REPS change the load. A shim logs each allocator call over 0.3 ms by thread.
set -eu
cp -r /src /work && cd /work
export CARGO_TARGET_DIR=/build
bld() { cargo test -q -p segment-log-core --lib --release --no-run --message-format=json 2>/dev/null |
  python3 -c 'import json,sys; print(next(x["executable"] for x in map(json.loads,sys.stdin) if x.get("executable")))'; }
cp "$(bld)" /bin/orig
python3 - <<'P'
p = 'segment-log-core/src/engine/writer.rs'
s = open(p).read()
o = '.spawn(move || drop(keys));'
assert o in s
open(p, 'w').write(s.replace(o, '.spawn(move || std::mem::forget(keys));'))
P
cp "$(bld)" /bin/leak
cat > /shim.c <<'C'
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stddef.h>
#include <stdio.h>
#include <time.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/prctl.h>
extern void*__libc_malloc(size_t);extern void __libc_free(void*);extern void*__libc_realloc(void*,size_t);extern void*__libc_calloc(size_t,size_t);
static long long ns(clockid_t c){struct timespec t;clock_gettime(c,&t);return t.tv_sec*1000000000LL+t.tv_nsec;}
static void rep(const char*op,size_t n,long long w,long long c){char name[17]={0},b[160];prctl(PR_GET_NAME,name);int k=snprintf(b,sizeof b,"SLOW %s thread=%s size=%zu wall_us=%lld cpu_us=%lld at=%lld\n",op,name,n,w/1000,c/1000,ns(CLOCK_MONOTONIC)/1000);static int fd=-2;if(fd==-2)fd=open("/tmp/slow",O_WRONLY|O_CREAT|O_APPEND,0644);if(write(fd,b,k)){}}
#define T(op,n,call) long long c=ns(CLOCK_THREAD_CPUTIME_ID),w=ns(CLOCK_MONOTONIC);call;w=ns(CLOCK_MONOTONIC)-w;if(w>300000)rep(op,n,w,ns(CLOCK_THREAD_CPUTIME_ID)-c);
/* CPU time is taken only for calls over 1 ms, from a per-thread baseline kept cheap: read before only for sizes that can consolidate. */
#define TC(op,n,call) long long c=(n)>=1024?ns(CLOCK_THREAD_CPUTIME_ID):0,w=ns(CLOCK_MONOTONIC);call;w=ns(CLOCK_MONOTONIC)-w;if(w>300000)rep(op,n,w,c?ns(CLOCK_THREAD_CPUTIME_ID)-c:-1);
void*malloc(size_t n){void*p;TC("malloc",n,p=__libc_malloc(n));return p;}
void*calloc(size_t a,size_t n){void*p;TC("calloc",a*n,p=__libc_calloc(a,n));return p;}
void*realloc(void*q,size_t n){void*p;TC("realloc",n,p=__libc_realloc(q,n));return p;}
void free(void*p){T("free",0,__libc_free(p));}
C
gcc -O2 -shared -fPIC -o /shim.so /shim.c
export SEGMENT_LOG_EVIDENCE_DIR=/data SEGMENT_LOG_BENCH_CALLERS=1000 SEGMENT_LOG_BENCH_ROUNDS=${ROUNDS:-1000}
export SEGMENT_LOG_DEVICE_ROUNDS=10 SEGMENT_LOG_BENCH_INTERVAL_US=20000 SEGMENT_LOG_TEST_ROLL_BYTES=${ROLL:-67108864}
for i in $(seq 1 "${REPS:-3}"); do for m in orig mxfast0 leak; do
  d=/out/$m-$i; mkdir -p "$d"; rm -f /tmp/slow; touch /tmp/slow
  b=/bin/orig; [ $m != leak ] || b=/bin/leak
  t=; [ $m != mxfast0 ] || t=glibc.malloc.mxfast=0
  LD_PRELOAD=/shim.so GLIBC_TUNABLES=$t SEGMENT_LOG_PIPELINE_OUT=$d/t.tsv \
    "$b" --exact acceptance::evidence::measured_appends_survive_reopen --nocapture > "$d/log" 2>&1
  rm -f "$d/t.tsv"; rm -rf /data/*; cp /tmp/slow "$d/slow"
done; done
for m in orig mxfast0 leak; do
  python3 -I docs/evidence/issue154/arena.py "/out/$m-*"
  cat /out/$m-*/slow | { grep segment-log-wri || true; } | sed -E 's/.*wall_us=([0-9]+).*/\1/' |
    awk -v m=$m '{s+=$1} END {printf "%s: writer spent %.1f ms in %d slow allocator calls\n", m, s/1000, NR}'
done
