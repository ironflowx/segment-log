# Host capture

Written by [run.sh](run.sh) before the measured runs, except `progress.txt`, which the driver appended as each run started and finished.

## host.txt

```text
Fri Nov 27 03:54:11 PM UTC 2026
commit e2f461f407f74ce009592def2fe2af59d2b766ae
?? docs/evidence/few-callers/
rustc 1.100.0 (0ef1bc386 2026-11-09)
binary: rustc
commit-hash: 0ef1bc386059596dada8fb65b50425aab2ea637a
commit-date: 2026-11-09
host: x86_64-unknown-linux-gnu
release: 1.100.0
LLVM version: 23.1.5
Linux cursor 6.12.113+ #1 SMP PREEMPT_DYNAMIC Thu Nov 26 16:06:57 UTC 2026 x86_64 x86_64 x86_64 GNU/Linux
PRETTY_NAME="Ubuntu 24.04.5 LTS"
NAME="Ubuntu"
VERSION_ID="24.04"
VERSION="24.04.5 LTS (Noble Numbat)"
VERSION_CODENAME=noble
ID=ubuntu
ID_LIKE=debian
HOME_URL="https://www.ubuntu.com/"
SUPPORT_URL="https://help.ubuntu.com/"
BUG_REPORT_URL="https://bugs.launchpad.net/ubuntu/"
PRIVACY_POLICY_URL="https://www.ubuntu.com/legal/terms-and-policies/privacy-policy"
UBUNTU_CODENAME=noble
LOGO=ubuntu-logo
virt kvm
dmi_sys_vendor 
dmi_product_name 
dmi_product_version 
4
Architecture:                            x86_64
CPU op-mode(s):                          32-bit, 64-bit
Address sizes:                           46 bits physical, 57 bits virtual
Byte Order:                              Little Endian
CPU(s):                                  4
On-line CPU(s) list:                     0-3
Vendor ID:                               GenuineIntel
Model name:                              Intel(R) Xeon(R) Processor
CPU family:                              6
Model:                                   207
Thread(s) per core:                      1
Core(s) per socket:                      4
Socket(s):                               1
Stepping:                                2
BogoMIPS:                                4800.00
Flags:                                   fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush dts mmx fxsr sse sse2 ss ht syscall nx pdpe1gb rdtscp lm constant_tsc pebs bts rep_good nopl xtopology nonstop_tsc cpuid tsc_known_freq pni pclmulqdq dtes64 vmx ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt tsc_deadline_timer aes xsave avx f16c rdrand hypervisor lahf_lm abm 3dnowprefetch cpuid_fault ssbd ibrs ibpb stibp ibrs_enhanced tpr_shadow flexpriority ept vpid ept_ad fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid avx512f avx512dq rdseed adx smap avx512ifma clflushopt clwb avx512cd sha_ni avx512bw avx512vl xsaveopt xsavec xgetbv1 xsaves avx_vnni avx512_bf16 wbnoinvd arat vnmi avx512vbmi umip pku ospke avx512_vbmi2 gfni vaes vpclmulqdq avx512_vnni avx512_bitalg avx512_vpopcntdq rdpid bus_lock_detect cldemote movdiri movdir64b fsrm md_clear serialize tsxldtrk amx_bf16 avx512_fp16 amx_tile amx_int8 flush_l1d arch_capabilities
Virtualization:                          VT-x
Hypervisor vendor:                       KVM
Virtualization type:                     full
L1d cache:                               192 KiB (4 instances)
L1i cache:                               128 KiB (4 instances)
L2 cache:                                8 MiB (4 instances)
L3 cache:                                320 MiB (1 instance)
NUMA node(s):                            1
NUMA node0 CPU(s):                       0-3
Vulnerability Gather data sampling:      Not affected
Vulnerability Indirect target selection: Not affected
Vulnerability Itlb multihit:             Not affected
Vulnerability L1tf:                      Not affected
Vulnerability Mds:                       Not affected
Vulnerability Meltdown:                  Not affected
Vulnerability Mmio stale data:           Not affected
Vulnerability Reg file data sampling:    Not affected
Vulnerability Retbleed:                  Not affected
Vulnerability Spec rstack overflow:      Not affected
Vulnerability Spec store bypass:         Mitigation; Speculative Store Bypass disabled via prctl
Vulnerability Spectre v1:                Mitigation; usercopy/swapgs barriers and __user pointer sanitization
Vulnerability Spectre v2:                Mitigation; Enhanced / Automatic IBRS; IBPB conditional; PBRSB-eIBRS SW sequence; BHI BHI_DIS_S
Vulnerability Srbds:                     Not affected
Vulnerability Tsa:                       Not affected
Vulnerability Tsx async abort:           Mitigation; TSX disabled
Vulnerability Vmscape:                   Not affected
               total        used        free      shared  buff/cache   available
Mem:     16780623872 11633213440  2885935104    18812928  2626125824  5147410432
Swap:              0           0           0
Filesystem     Type 1K-blocks    Used Available Use% Mounted on
/dev/vdc       ext4 265591976 9003044 256572548   4% /
TARGET SOURCE   FSTYPE OPTIONS
/      /dev/vdc ext4   rw,relatime
NAME TYPE  SIZE ROTA MODEL TRAN   MOUNTPOINTS
vda  disk  256G    1       virtio 
vdb  disk  256G    1       virtio 
vdc  disk  256G    1       virtio /
--- queue vdc ---
write_cache write through
rotational 1
scheduler none [mq-deadline] kyber bfq 
fua 0
logical_block_size 512
physical_block_size 512
virtio_features 0000000000000000000000000000010010000000000000000000000000000000
 15:54:11 up 5 min,  0 user,  load average: 0.10, 0.18, 0.08
```

## flush-check.txt

```text
sync=False size=4096 n=200 min=1164 p50=1665 p99=3643 max=11960 clean_fdatasync_ns=3964913
sync=True size=4096 n=200 min=84698 p50=93562 p99=164773 max=166613 clean_fdatasync_ns=1726
sync=True size=262144 n=100 min=339474 p50=454192 p99=2189853 max=2256065 clean_fdatasync_ns=1436
```

## binary.sha256

```text
d98319ebb735590f633f02bd73bcbf9500f29e99ea284bfdce0436671f3f536d  /agent/repos/segment-log/target/release/build/segment-log-core/d93482b5e0e29db7/out/segment_log_core-d93482b5e0e29db7
```

## progress.txt

```text
start warmup-c1 callers=1 rounds=1000 device_rounds=8 2026-11-27T15:54:14Z
end warmup-c1 2026-11-27T15:54:15Z
start warmup-c4 callers=4 rounds=1000 device_rounds=8 2026-11-27T15:54:17Z
end warmup-c4 2026-11-27T15:54:19Z
start warmup-c16 callers=16 rounds=400 device_rounds=8 2026-11-27T15:54:21Z
end warmup-c16 2026-11-27T15:54:21Z
start warmup-c1000 callers=1000 rounds=20 device_rounds=8 2026-11-27T15:54:23Z
end warmup-c1000 2026-11-27T15:54:24Z
start c1-run1 callers=1 rounds=1000000 device_rounds=1000 2026-11-27T15:54:26Z
end c1-run1 2026-11-27T16:32:34Z
start c4-run1 callers=4 rounds=250000 device_rounds=1000 2026-11-27T16:32:36Z
end c4-run1 2026-11-27T16:44:20Z
start c16-run1 callers=16 rounds=62500 device_rounds=1000 2026-11-27T16:44:22Z
end c16-run1 2026-11-27T16:46:06Z
start c1000-run1 callers=1000 rounds=1000 device_rounds=1000 2026-11-27T16:46:08Z
end c1000-run1 2026-11-27T16:46:14Z
start c1-run2 callers=1 rounds=1000000 device_rounds=1000 2026-11-27T16:46:16Z
end c1-run2 2026-11-27T17:56:33Z
start c4-run2 callers=4 rounds=250000 device_rounds=1000 2026-11-27T17:56:36Z
end c4-run2 2026-11-27T18:03:12Z
start c16-run2 callers=16 rounds=62500 device_rounds=1000 2026-11-27T18:03:14Z
end c16-run2 2026-11-27T18:04:58Z
start c1000-run2 callers=1000 rounds=1000 device_rounds=1000 2026-11-27T18:05:00Z
end c1000-run2 2026-11-27T18:05:07Z
start c1-run3 callers=1 rounds=1000000 device_rounds=1000 2026-11-27T18:05:09Z
end c1-run3 2026-11-27T19:50:13Z
start c4-run3 callers=4 rounds=250000 device_rounds=1000 2026-11-27T19:50:15Z
end c4-run3 2026-11-27T19:57:00Z
start c16-run3 callers=16 rounds=62500 device_rounds=1000 2026-11-27T19:57:02Z
end c16-run3 2026-11-27T19:58:47Z
start c1000-run3 callers=1000 rounds=1000 device_rounds=1000 2026-11-27T19:58:49Z
end c1000-run3 2026-11-27T19:58:55Z
all done 2026-11-27T19:58:55Z
```
