# Measured run output

Stdout of the 12 measured runs. Warmup runs are not included. Stderr of every measured run was empty.

## c1-run1.log

```text

running 1 test
DIAGNOSTIC measured_storage=true phase_cpu=false
CONFIG callers=1 rounds=1000000 payload=256 records_per_submit=1 seed=none_literal_formula linger_ns=1000000 group_byte_cap=1048576 roll_bytes=67108864 queue_capacity=4096 pending_byte_budget=16777216
METRIC device_fdatasync_ns count=1000 min=221895 p50=294503 p99=1552124 p999=2282320 max=2577585 sum=386167460
DEVICE start_bytes=0 end_bytes=262144000 append_bytes=262144 failures=0
OPEN initial_ns=612595 records=0 segments=1 bytes=64
test acceptance::evidence::measured_appends_survive_reopen has been running for over 60 seconds
APPEND records=1000000 elapsed_ns=1699991119144 records_per_second=588.238 full=0 other_admission_errors=0 commit_errors=0 correctness_errors=0 peak_reserved_bytes=544 admission_call_sum_ns=3641239678
METRIC append_ns count=1000000 min=1083968 p50=1690880 p99=2747492 p999=3152691 max=53578015 sum=1699643168553
METRIC group_submits count=1000000 min=1 p50=1 p99=1 p999=1 max=1 sum=1000000
METRIC group_frame_bytes count=1000000 min=336 p50=336 p99=336 p999=336 max=336 sum=336000000
METRIC group_fdatasync_ns count=1000000 min=53967 p50=566764 p99=1562693 p999=1892739 max=49213842 sum=572786972910
STORAGE kind=Group.Append count=1000000 sum_ns=7912775518
STORAGE kind=Group.SyncData count=1000000 sum_ns=572786972910
STORAGE kind=KeyIndex.Other count=526 sum_ns=56681367
STORAGE kind=LogFilter.Other count=91 sum_ns=10683557
STORAGE kind=Lookup.Other count=11840 sum_ns=230444619
STORAGE kind=Open.Other count=7 sum_ns=110162
STORAGE kind=Open.SyncData count=1 sum_ns=146499
STORAGE kind=Open.SyncDir count=1 sum_ns=3376
STORAGE kind=Recovery.Other count=2 sum_ns=2386
STORAGE kind=Recovery.SyncAll count=1 sum_ns=27336
STORAGE kind=Recovery.SyncDir count=2 sum_ns=2983
STORAGE kind=Roll.Append count=20 sum_ns=17373883
STORAGE kind=Roll.Other count=10 sum_ns=1755870
STORAGE kind=Roll.SyncData count=15 sum_ns=21705478
STORAGE kind=Roll.SyncDir count=5 sum_ns=44105
END records=1000000 segments=6 bytes=354104596
ROLL index=0 writer_ns=4627076
ROLL index=1 writer_ns=6065703
ROLL index=2 writer_ns=37207150
ROLL index=3 writer_ns=4954572
ROLL index=4 writer_ns=5726006
REOPEN ns=11909876 start_records=1000000 end_records=1000000 start_segments=6 end_segments=6 start_bytes=354104596 end_bytes=354104596 correctness_errors=0
test acceptance::evidence::measured_appends_survive_reopen ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 321 filtered out; finished in 1702.38s
```

## c1-run2.log

```text

running 1 test
DIAGNOSTIC measured_storage=true phase_cpu=false
CONFIG callers=1 rounds=1000000 payload=256 records_per_submit=1 seed=none_literal_formula linger_ns=1000000 group_byte_cap=1048576 roll_bytes=67108864 queue_capacity=4096 pending_byte_budget=16777216
METRIC device_fdatasync_ns count=1000 min=210709 p50=264229 p99=1088450 p999=1780458 max=1951594 sum=319141543
DEVICE start_bytes=0 end_bytes=262144000 append_bytes=262144 failures=0
OPEN initial_ns=930179 records=0 segments=1 bytes=64
test acceptance::evidence::measured_appends_survive_reopen has been running for over 60 seconds
APPEND records=1000000 elapsed_ns=1769437786513 records_per_second=565.151 full=0 other_admission_errors=0 commit_errors=0 correctness_errors=0 peak_reserved_bytes=544 admission_call_sum_ns=3849026537
METRIC append_ns count=1000000 min=1080710 p50=1792982 p99=2742171 p999=3165276 max=59558777 sum=1769126859799
METRIC group_submits count=1000000 min=1 p50=1 p99=1 p999=1 max=1 sum=1000000
METRIC group_frame_bytes count=1000000 min=336 p50=336 p99=336 p999=336 max=336 sum=336000000
METRIC group_fdatasync_ns count=1000000 min=52310 p50=670301 p99=1555087 p999=1869358 max=36912092 sum=638757587716
STORAGE kind=Group.Append count=1000000 sum_ns=8164334994
STORAGE kind=Group.SyncData count=1000000 sum_ns=638757587716
STORAGE kind=KeyIndex.Other count=595 sum_ns=55669632
STORAGE kind=LogFilter.Other count=91 sum_ns=10252409
STORAGE kind=Lookup.Other count=11840 sum_ns=201601439
STORAGE kind=Open.Other count=7 sum_ns=402168
STORAGE kind=Open.SyncData count=1 sum_ns=161318
STORAGE kind=Open.SyncDir count=1 sum_ns=3063
STORAGE kind=Recovery.Other count=2 sum_ns=3021
STORAGE kind=Recovery.SyncAll count=1 sum_ns=18529
STORAGE kind=Recovery.SyncDir count=2 sum_ns=2959
STORAGE kind=Roll.Append count=20 sum_ns=26342379
STORAGE kind=Roll.Other count=10 sum_ns=1739293
STORAGE kind=Roll.SyncData count=15 sum_ns=22439946
STORAGE kind=Roll.SyncDir count=5 sum_ns=52669
END records=1000000 segments=6 bytes=354104596
ROLL index=0 writer_ns=4646423
ROLL index=1 writer_ns=32953946
ROLL index=2 writer_ns=6393729
ROLL index=3 writer_ns=56302453
ROLL index=4 writer_ns=6283928
REOPEN ns=22443192 start_records=1000000 end_records=1000000 start_segments=6 end_segments=6 start_bytes=354104596 end_bytes=354104596 correctness_errors=0
test acceptance::evidence::measured_appends_survive_reopen ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 321 filtered out; finished in 1771.92s
```

## c1-run3.log

```text

running 1 test
DIAGNOSTIC measured_storage=true phase_cpu=false
CONFIG callers=1 rounds=1000000 payload=256 records_per_submit=1 seed=none_literal_formula linger_ns=1000000 group_byte_cap=1048576 roll_bytes=67108864 queue_capacity=4096 pending_byte_budget=16777216
METRIC device_fdatasync_ns count=1000 min=216362 p50=279262 p99=1365774 p999=5230450 max=6939149 sum=394417409
DEVICE start_bytes=0 end_bytes=262144000 append_bytes=262144 failures=0
OPEN initial_ns=691215 records=0 segments=1 bytes=64
test acceptance::evidence::measured_appends_survive_reopen has been running for over 60 seconds
APPEND records=1000000 elapsed_ns=1816107057332 records_per_second=550.628 full=0 other_admission_errors=0 commit_errors=0 correctness_errors=0 peak_reserved_bytes=544 admission_call_sum_ns=4048752854
METRIC append_ns count=1000000 min=1078780 p50=1832232 p99=2699223 p999=3099432 max=157384955 sum=1815756647717
METRIC group_submits count=1000000 min=1 p50=1 p99=1 p999=1 max=1 sum=1000000
METRIC group_frame_bytes count=1000000 min=336 p50=336 p99=336 p999=336 max=336 sum=336000000
METRIC group_fdatasync_ns count=1000000 min=54118 p50=700966 p99=1510805 p999=1783130 max=15814734 sum=683291234284
STORAGE kind=Group.Append count=1000000 sum_ns=8829668208
STORAGE kind=Group.SyncData count=1000000 sum_ns=683291234284
STORAGE kind=KeyIndex.Other count=570 sum_ns=51670578
STORAGE kind=LogFilter.Other count=91 sum_ns=9933893
STORAGE kind=Lookup.Other count=11840 sum_ns=393411465
STORAGE kind=Open.Other count=7 sum_ns=146784
STORAGE kind=Open.SyncData count=1 sum_ns=152038
STORAGE kind=Open.SyncDir count=1 sum_ns=2656
STORAGE kind=Recovery.Other count=2 sum_ns=3708
STORAGE kind=Recovery.SyncAll count=1 sum_ns=27854
STORAGE kind=Recovery.SyncDir count=2 sum_ns=2891
STORAGE kind=Roll.Append count=20 sum_ns=29581023
STORAGE kind=Roll.Other count=10 sum_ns=5332348
STORAGE kind=Roll.SyncData count=15 sum_ns=20972836
STORAGE kind=Roll.SyncDir count=5 sum_ns=56817
END records=1000000 segments=6 bytes=354104596
ROLL index=0 writer_ns=13849138
ROLL index=1 writer_ns=3977600
ROLL index=2 writer_ns=46081620
ROLL index=3 writer_ns=5551995
ROLL index=4 writer_ns=4143708
REOPEN ns=12145417 start_records=1000000 end_records=1000000 start_segments=6 end_segments=6 start_bytes=354104596 end_bytes=354104596 correctness_errors=0
test acceptance::evidence::measured_appends_survive_reopen ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 321 filtered out; finished in 1819.13s
```

## c4-run1.log

```text

running 1 test
DIAGNOSTIC measured_storage=true phase_cpu=false
CONFIG callers=4 rounds=250000 payload=256 records_per_submit=1 seed=none_literal_formula linger_ns=1000000 group_byte_cap=1048576 roll_bytes=67108864 queue_capacity=4096 pending_byte_budget=16777216
METRIC device_fdatasync_ns count=1000 min=201798 p50=290549 p99=1640018 p999=2046556 max=2299991 sum=427465182
DEVICE start_bytes=0 end_bytes=262144000 append_bytes=262144 failures=0
OPEN initial_ns=1072455 records=0 segments=1 bytes=64
test acceptance::evidence::measured_appends_survive_reopen has been running for over 60 seconds
APPEND records=1000000 elapsed_ns=397921352393 records_per_second=2513.059 full=0 other_admission_errors=0 commit_errors=0 correctness_errors=0 peak_reserved_bytes=2176 admission_call_sum_ns=3223165868
METRIC append_ns count=1000000 min=199469 p50=1578814 p99=2544789 p999=3033960 max=53561317 sum=1590964895383
METRIC group_submits count=250011 min=1 p50=4 p99=4 p999=4 max=4 sum=1000000
METRIC group_frame_bytes count=250011 min=336 p50=1224 p99=1224 p999=1224 max=1224 sum=306000440
METRIC group_fdatasync_ns count=250011 min=53798 p50=501660 p99=1356987 p999=1717151 max=51740678 sum=126431592714
STORAGE kind=Group.Append count=250011 sum_ns=2211429058
STORAGE kind=Group.SyncData count=250011 sum_ns=126431592714
STORAGE kind=KeyIndex.Other count=626 sum_ns=24218232
STORAGE kind=LogFilter.Other count=84 sum_ns=7640423
STORAGE kind=Lookup.Other count=41108 sum_ns=343261252
STORAGE kind=Open.Other count=7 sum_ns=147786
STORAGE kind=Open.SyncData count=1 sum_ns=171384
STORAGE kind=Open.SyncDir count=1 sum_ns=2944
STORAGE kind=Recovery.Other count=2 sum_ns=3752
STORAGE kind=Recovery.SyncAll count=1 sum_ns=26908
STORAGE kind=Recovery.SyncDir count=2 sum_ns=2919
STORAGE kind=Roll.Append count=16 sum_ns=19218389
STORAGE kind=Roll.Other count=8 sum_ns=4100183
STORAGE kind=Roll.SyncData count=12 sum_ns=16627462
STORAGE kind=Roll.SyncDir count=4 sum_ns=41364
END records=1000000 segments=5 bytes=321895848
ROLL index=0 writer_ns=5243592
ROLL index=1 writer_ns=4734741
ROLL index=2 writer_ns=2882009
ROLL index=3 writer_ns=1590587
REOPEN ns=129522723 start_records=1000000 end_records=1000000 start_segments=5 end_segments=5 start_bytes=321895848 end_bytes=321895848 correctness_errors=0
test acceptance::evidence::measured_appends_survive_reopen ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 321 filtered out; finished in 400.94s
```

## c4-run2.log

```text

running 1 test
DIAGNOSTIC measured_storage=true phase_cpu=false
CONFIG callers=4 rounds=250000 payload=256 records_per_submit=1 seed=none_literal_formula linger_ns=1000000 group_byte_cap=1048576 roll_bytes=67108864 queue_capacity=4096 pending_byte_budget=16777216
METRIC device_fdatasync_ns count=1000 min=246185 p50=456430 p99=1736067 p999=2095555 max=2599067 sum=608654500
DEVICE start_bytes=0 end_bytes=262144000 append_bytes=262144 failures=0
OPEN initial_ns=1452025 records=0 segments=1 bytes=64
test acceptance::evidence::measured_appends_survive_reopen has been running for over 60 seconds
APPEND records=1000000 elapsed_ns=393500741987 records_per_second=2541.291 full=0 other_admission_errors=0 commit_errors=0 correctness_errors=0 peak_reserved_bytes=2176 admission_call_sum_ns=3026083910
METRIC append_ns count=1000000 min=628533 p50=1576119 p99=2473002 p999=2858779 max=22550549 sum=1573541292297
METRIC group_submits count=250014 min=1 p50=4 p99=4 p999=4 max=4 sum=1000000
METRIC group_frame_bytes count=250014 min=336 p50=1224 p99=1224 p999=1224 max=1224 sum=306000560
METRIC group_fdatasync_ns count=250014 min=55848 p50=504675 p99=1300553 p999=1608452 max=19351274 sum=123079364606
STORAGE kind=Group.Append count=250014 sum_ns=1966335737
STORAGE kind=Group.SyncData count=250014 sum_ns=123079364606
STORAGE kind=KeyIndex.Other count=602 sum_ns=14211832
STORAGE kind=LogFilter.Other count=84 sum_ns=8541251
STORAGE kind=Lookup.Other count=41107 sum_ns=282575204
STORAGE kind=Open.Other count=7 sum_ns=183037
STORAGE kind=Open.SyncData count=1 sum_ns=349809
STORAGE kind=Open.SyncDir count=1 sum_ns=3018
STORAGE kind=Recovery.Other count=2 sum_ns=3147
STORAGE kind=Recovery.SyncAll count=1 sum_ns=47640
STORAGE kind=Recovery.SyncDir count=2 sum_ns=3243
STORAGE kind=Roll.Append count=16 sum_ns=18088669
STORAGE kind=Roll.Other count=8 sum_ns=1297349
STORAGE kind=Roll.SyncData count=12 sum_ns=18562580
STORAGE kind=Roll.SyncDir count=4 sum_ns=20915
END records=1000000 segments=5 bytes=321892720
ROLL index=0 writer_ns=2729394
ROLL index=1 writer_ns=1646852
ROLL index=2 writer_ns=1510679
ROLL index=3 writer_ns=2466797
REOPEN ns=117192457 start_records=1000000 end_records=1000000 start_segments=5 end_segments=5 start_bytes=321892720 end_bytes=321892720 correctness_errors=0
test acceptance::evidence::measured_appends_survive_reopen ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 321 filtered out; finished in 396.42s
```

## c4-run3.log

```text

running 1 test
DIAGNOSTIC measured_storage=true phase_cpu=false
CONFIG callers=4 rounds=250000 payload=256 records_per_submit=1 seed=none_literal_formula linger_ns=1000000 group_byte_cap=1048576 roll_bytes=67108864 queue_capacity=4096 pending_byte_budget=16777216
METRIC device_fdatasync_ns count=1000 min=216206 p50=333277 p99=1926694 p999=2874822 max=3550085 sum=537175585
DEVICE start_bytes=0 end_bytes=262144000 append_bytes=262144 failures=0
OPEN initial_ns=1096829 records=0 segments=1 bytes=64
test acceptance::evidence::measured_appends_survive_reopen has been running for over 60 seconds
APPEND records=1000000 elapsed_ns=402486567398 records_per_second=2484.555 full=0 other_admission_errors=0 commit_errors=0 correctness_errors=0 peak_reserved_bytes=2176 admission_call_sum_ns=3169393197
METRIC append_ns count=1000000 min=253796 p50=1650820 p99=2442643 p999=2890203 max=21162850 sum=1609455200806
METRIC group_submits count=250008 min=1 p50=4 p99=4 p999=4 max=4 sum=1000000
METRIC group_frame_bytes count=250008 min=336 p50=1224 p99=1224 p999=1224 max=1224 sum=306000320
METRIC group_fdatasync_ns count=250008 min=55096 p50=583554 p99=1272967 p999=1610235 max=9365994 sum=132048349647
STORAGE kind=Group.Append count=250008 sum_ns=1971697172
STORAGE kind=Group.SyncData count=250008 sum_ns=132048349647
STORAGE kind=KeyIndex.Other count=585 sum_ns=12888214
STORAGE kind=LogFilter.Other count=84 sum_ns=8273189
STORAGE kind=Lookup.Other count=41109 sum_ns=322705699
STORAGE kind=Open.Other count=7 sum_ns=184073
STORAGE kind=Open.SyncData count=1 sum_ns=155088
STORAGE kind=Open.SyncDir count=1 sum_ns=2736
STORAGE kind=Recovery.Other count=2 sum_ns=5107
STORAGE kind=Recovery.SyncAll count=1 sum_ns=19625
STORAGE kind=Recovery.SyncDir count=2 sum_ns=3270
STORAGE kind=Roll.Append count=16 sum_ns=20786472
STORAGE kind=Roll.Other count=8 sum_ns=3544546
STORAGE kind=Roll.SyncData count=12 sum_ns=18127885
STORAGE kind=Roll.SyncDir count=4 sum_ns=45808
END records=1000000 segments=5 bytes=321892424
ROLL index=0 writer_ns=3547274
ROLL index=1 writer_ns=2922857
ROLL index=2 writer_ns=3010291
ROLL index=3 writer_ns=2084834
REOPEN ns=121073564 start_records=1000000 end_records=1000000 start_segments=5 end_segments=5 start_bytes=321892424 end_bytes=321892424 correctness_errors=0
test acceptance::evidence::measured_appends_survive_reopen ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 321 filtered out; finished in 405.37s
```

## c16-run1.log

```text

running 1 test
DIAGNOSTIC measured_storage=true phase_cpu=false
CONFIG callers=16 rounds=62500 payload=256 records_per_submit=1 seed=none_literal_formula linger_ns=1000000 group_byte_cap=1048576 roll_bytes=67108864 queue_capacity=4096 pending_byte_budget=16777216
METRIC device_fdatasync_ns count=1000 min=219086 p50=320098 p99=1699826 p999=2763577 max=3183021 sum=507303403
DEVICE start_bytes=0 end_bytes=262144000 append_bytes=262144 failures=0
OPEN initial_ns=901227 records=0 segments=1 bytes=64
test acceptance::evidence::measured_appends_survive_reopen has been running for over 60 seconds
APPEND records=1000000 elapsed_ns=100926607064 records_per_second=9908.190 full=0 other_admission_errors=0 commit_errors=0 correctness_errors=0 peak_reserved_bytes=8704 admission_call_sum_ns=2738894601
METRIC append_ns count=1000000 min=300298 p50=1596998 p99=2524754 p999=2913848 max=15497619 sum=1614360302686
METRIC group_submits count=62505 min=1 p50=16 p99=16 p999=16 max=16 sum=1000000
METRIC group_frame_bytes count=62505 min=336 p50=4776 p99=4776 p999=4776 max=4776 sum=298500200
METRIC group_fdatasync_ns count=62505 min=83268 p50=505423 p99=1308779 p999=1615779 max=14153097 sum=32186890213
STORAGE kind=Group.Append count=62505 sum_ns=679475394
STORAGE kind=Group.SyncData count=62505 sum_ns=32186890213
STORAGE kind=KeyIndex.Other count=561 sum_ns=10968537
STORAGE kind=LogFilter.Other count=84 sum_ns=4469364
STORAGE kind=Lookup.Other count=17446 sum_ns=439561586
STORAGE kind=Open.Other count=7 sum_ns=156831
STORAGE kind=Open.SyncData count=1 sum_ns=167389
STORAGE kind=Open.SyncDir count=1 sum_ns=2845
STORAGE kind=Recovery.Other count=2 sum_ns=3003
STORAGE kind=Recovery.SyncAll count=1 sum_ns=18327
STORAGE kind=Recovery.SyncDir count=2 sum_ns=2787
STORAGE kind=Roll.Append count=16 sum_ns=12887963
STORAGE kind=Roll.Other count=8 sum_ns=1026656
STORAGE kind=Roll.SyncData count=12 sum_ns=14958297
STORAGE kind=Roll.SyncDir count=4 sum_ns=45559
END records=1000000 segments=5 bytes=314791104
ROLL index=0 writer_ns=1542747
ROLL index=1 writer_ns=1265994
ROLL index=2 writer_ns=1687756
ROLL index=3 writer_ns=835652
REOPEN ns=52613062 start_records=1000000 end_records=1000000 start_segments=5 end_segments=5 start_bytes=314791104 end_bytes=314791104 correctness_errors=0
test acceptance::evidence::measured_appends_survive_reopen ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 321 filtered out; finished in 104.04s
```

## c16-run2.log

```text

running 1 test
DIAGNOSTIC measured_storage=true phase_cpu=false
CONFIG callers=16 rounds=62500 payload=256 records_per_submit=1 seed=none_literal_formula linger_ns=1000000 group_byte_cap=1048576 roll_bytes=67108864 queue_capacity=4096 pending_byte_budget=16777216
METRIC device_fdatasync_ns count=1000 min=221013 p50=390706 p99=1767901 p999=2260125 max=2666117 sum=573801853
DEVICE start_bytes=0 end_bytes=262144000 append_bytes=262144 failures=0
OPEN initial_ns=776923 records=0 segments=1 bytes=64
test acceptance::evidence::measured_appends_survive_reopen has been running for over 60 seconds
APPEND records=1000000 elapsed_ns=101369068810 records_per_second=9864.942 full=0 other_admission_errors=0 commit_errors=0 correctness_errors=0 peak_reserved_bytes=8704 admission_call_sum_ns=2662020117
METRIC append_ns count=1000000 min=237541 p50=1630635 p99=2503087 p999=2914047 max=12473268 sum=1621444301674
METRIC group_submits count=62517 min=1 p50=16 p99=16 p999=16 max=16 sum=1000000
METRIC group_frame_bytes count=62517 min=336 p50=4776 p99=4776 p999=4776 max=4776 sum=298500680
METRIC group_fdatasync_ns count=62517 min=87829 p50=554207 p99=1290763 p999=1613707 max=10797204 sum=32763036217
STORAGE kind=Group.Append count=62517 sum_ns=646874099
STORAGE kind=Group.SyncData count=62517 sum_ns=32763036217
STORAGE kind=KeyIndex.Other count=661 sum_ns=10756670
STORAGE kind=LogFilter.Other count=84 sum_ns=5972719
STORAGE kind=Lookup.Other count=17446 sum_ns=397806742
STORAGE kind=Open.Other count=7 sum_ns=141636
STORAGE kind=Open.SyncData count=1 sum_ns=248178
STORAGE kind=Open.SyncDir count=1 sum_ns=2715
STORAGE kind=Recovery.Other count=2 sum_ns=4016
STORAGE kind=Recovery.SyncAll count=1 sum_ns=26602
STORAGE kind=Recovery.SyncDir count=2 sum_ns=2722
STORAGE kind=Roll.Append count=16 sum_ns=9098500
STORAGE kind=Roll.Other count=8 sum_ns=437465
STORAGE kind=Roll.SyncData count=12 sum_ns=16270995
STORAGE kind=Roll.SyncDir count=4 sum_ns=30016
END records=1000000 segments=5 bytes=314789320
ROLL index=0 writer_ns=1037492
ROLL index=1 writer_ns=2475787
ROLL index=2 writer_ns=851229
ROLL index=3 writer_ns=434703
REOPEN ns=49126844 start_records=1000000 end_records=1000000 start_segments=5 end_segments=5 start_bytes=314789320 end_bytes=314789320 correctness_errors=0
test acceptance::evidence::measured_appends_survive_reopen ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 321 filtered out; finished in 104.33s
```

## c16-run3.log

```text

running 1 test
DIAGNOSTIC measured_storage=true phase_cpu=false
CONFIG callers=16 rounds=62500 payload=256 records_per_submit=1 seed=none_literal_formula linger_ns=1000000 group_byte_cap=1048576 roll_bytes=67108864 queue_capacity=4096 pending_byte_budget=16777216
METRIC device_fdatasync_ns count=1000 min=218737 p50=314233 p99=1528413 p999=2259044 max=3359473 sum=486955193
DEVICE start_bytes=0 end_bytes=262144000 append_bytes=262144 failures=0
OPEN initial_ns=681754 records=0 segments=1 bytes=64
test acceptance::evidence::measured_appends_survive_reopen has been running for over 60 seconds
APPEND records=1000000 elapsed_ns=102061100128 records_per_second=9798.052 full=0 other_admission_errors=0 commit_errors=0 correctness_errors=0 peak_reserved_bytes=8704 admission_call_sum_ns=2952091069
METRIC append_ns count=1000000 min=341953 p50=1662044 p99=2482924 p999=2958392 max=12089657 sum=1632559096444
METRIC group_submits count=62506 min=1 p50=16 p99=16 p999=16 max=16 sum=1000000
METRIC group_frame_bytes count=62506 min=336 p50=4776 p99=4776 p999=4776 max=4776 sum=298500240
METRIC group_fdatasync_ns count=62506 min=81701 p50=588856 p99=1265647 p999=1648848 max=9913324 sum=33465051266
STORAGE kind=Group.Append count=62506 sum_ns=618212783
STORAGE kind=Group.SyncData count=62506 sum_ns=33465051266
STORAGE kind=KeyIndex.Other count=678 sum_ns=12071662
STORAGE kind=LogFilter.Other count=84 sum_ns=5380330
STORAGE kind=Lookup.Other count=17444 sum_ns=406205906
STORAGE kind=Open.Other count=7 sum_ns=159700
STORAGE kind=Open.SyncData count=1 sum_ns=156028
STORAGE kind=Open.SyncDir count=1 sum_ns=2628
STORAGE kind=Recovery.Other count=2 sum_ns=3608
STORAGE kind=Recovery.SyncAll count=1 sum_ns=27141
STORAGE kind=Recovery.SyncDir count=2 sum_ns=2882
STORAGE kind=Roll.Append count=16 sum_ns=10206918
STORAGE kind=Roll.Other count=8 sum_ns=2268094
STORAGE kind=Roll.SyncData count=12 sum_ns=16445634
STORAGE kind=Roll.SyncDir count=4 sum_ns=24187
END records=1000000 segments=5 bytes=314793000
ROLL index=0 writer_ns=720781
ROLL index=1 writer_ns=1108248
ROLL index=2 writer_ns=3207217
ROLL index=3 writer_ns=533275
REOPEN ns=49344765 start_records=1000000 end_records=1000000 start_segments=5 end_segments=5 start_bytes=314793000 end_bytes=314793000 correctness_errors=0
test acceptance::evidence::measured_appends_survive_reopen ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 321 filtered out; finished in 104.85s
```

## c1000-run1.log

```text

running 1 test
DIAGNOSTIC measured_storage=true phase_cpu=false
CONFIG callers=1000 rounds=1000 payload=256 records_per_submit=1 seed=none_literal_formula linger_ns=1000000 group_byte_cap=1048576 roll_bytes=67108864 queue_capacity=4096 pending_byte_budget=16777216
METRIC device_fdatasync_ns count=1000 min=214257 p50=301088 p99=1466347 p999=1745941 max=1857029 sum=389701980
DEVICE start_bytes=0 end_bytes=262144000 append_bytes=262144 failures=0
OPEN initial_ns=801358 records=0 segments=1 bytes=64
APPEND records=1000000 elapsed_ns=3607258416 records_per_second=277218.842 full=0 other_admission_errors=0 commit_errors=0 correctness_errors=0 peak_reserved_bytes=544000 admission_call_sum_ns=699825172
METRIC append_ns count=1000000 min=369441 p50=3409981 p99=7168937 p999=9403344 max=14945271 sum=3575542261192
METRIC group_submits count=2001 min=1 p50=499 p99=883 p999=1000 max=1000 sum=1000000
METRIC group_frame_bytes count=2001 min=336 p50=147744 p99=261408 p999=296040 max=296040 sum=296080040
METRIC group_fdatasync_ns count=2001 min=130659 p50=279465 p99=1314197 p999=2124893 max=3983105 sum=979288505
STORAGE kind=Group.Append count=2001 sum_ns=175594267
STORAGE kind=Group.SyncData count=2001 sum_ns=979288505
STORAGE kind=KeyIndex.Other count=542 sum_ns=9033969
STORAGE kind=LogFilter.Other count=84 sum_ns=5350339
STORAGE kind=Lookup.Other count=2013 sum_ns=189263843
STORAGE kind=Open.Other count=7 sum_ns=161303
STORAGE kind=Open.SyncData count=1 sum_ns=179129
STORAGE kind=Open.SyncDir count=1 sum_ns=3104
STORAGE kind=Recovery.Other count=2 sum_ns=2807
STORAGE kind=Recovery.SyncAll count=1 sum_ns=32060
STORAGE kind=Recovery.SyncDir count=2 sum_ns=14336
STORAGE kind=Roll.Append count=16 sum_ns=8644477
STORAGE kind=Roll.Other count=8 sum_ns=640839
STORAGE kind=Roll.SyncData count=12 sum_ns=18046629
STORAGE kind=Roll.SyncDir count=4 sum_ns=50237
END records=1000000 segments=5 bytes=312686512
ROLL index=0 writer_ns=1985721
ROLL index=1 writer_ns=1087446
ROLL index=2 writer_ns=5394457
ROLL index=3 writer_ns=1327092
REOPEN ns=36856418 start_records=1000000 end_records=1000000 start_segments=5 end_segments=5 start_bytes=312686512 end_bytes=312686512 correctness_errors=0
test acceptance::evidence::measured_appends_survive_reopen ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 321 filtered out; finished in 6.31s
```

## c1000-run2.log

```text

running 1 test
DIAGNOSTIC measured_storage=true phase_cpu=false
CONFIG callers=1000 rounds=1000 payload=256 records_per_submit=1 seed=none_literal_formula linger_ns=1000000 group_byte_cap=1048576 roll_bytes=67108864 queue_capacity=4096 pending_byte_budget=16777216
METRIC device_fdatasync_ns count=1000 min=217303 p50=272512 p99=1230750 p999=1935358 max=1995537 sum=355092361
DEVICE start_bytes=0 end_bytes=262144000 append_bytes=262144 failures=0
OPEN initial_ns=830962 records=0 segments=1 bytes=64
APPEND records=1000000 elapsed_ns=3697054942 records_per_second=270485.566 full=0 other_admission_errors=0 commit_errors=0 correctness_errors=0 peak_reserved_bytes=544000 admission_call_sum_ns=623425198
METRIC append_ns count=1000000 min=521432 p50=3657435 p99=5807757 p999=9980802 max=17194848 sum=3665949662359
METRIC group_submits count=2005 min=1 p50=500 p99=849 p999=992 max=1000 sum=1000000
METRIC group_frame_bytes count=2005 min=336 p50=148040 p99=251344 p999=293672 max=296040 sum=296080200
METRIC group_fdatasync_ns count=2005 min=103342 p50=696048 p99=1224528 p999=2157459 max=2471761 sum=1265644892
STORAGE kind=Group.Append count=2005 sum_ns=225797662
STORAGE kind=Group.SyncData count=2005 sum_ns=1265644892
STORAGE kind=KeyIndex.Other count=367 sum_ns=6566584
STORAGE kind=LogFilter.Other count=70 sum_ns=4421894
STORAGE kind=Lookup.Other count=2017 sum_ns=212625862
STORAGE kind=Open.Other count=7 sum_ns=151437
STORAGE kind=Open.SyncData count=1 sum_ns=256896
STORAGE kind=Open.SyncDir count=1 sum_ns=2352
STORAGE kind=Recovery.Other count=2 sum_ns=2624
STORAGE kind=Recovery.SyncAll count=1 sum_ns=14484
STORAGE kind=Recovery.SyncDir count=2 sum_ns=2693
STORAGE kind=Roll.Append count=16 sum_ns=12282521
STORAGE kind=Roll.Other count=8 sum_ns=372369
STORAGE kind=Roll.SyncData count=12 sum_ns=15621312
STORAGE kind=Roll.SyncDir count=4 sum_ns=25987
END records=1000000 segments=5 bytes=312665848
ROLL index=0 writer_ns=1692595
ROLL index=1 writer_ns=1173715
ROLL index=2 writer_ns=476147
ROLL index=3 writer_ns=921000
REOPEN ns=35407700 start_records=1000000 end_records=1000000 start_segments=5 end_segments=5 start_bytes=312665848 end_bytes=312665848 correctness_errors=0
test acceptance::evidence::measured_appends_survive_reopen ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 321 filtered out; finished in 6.26s
```

## c1000-run3.log

```text

running 1 test
DIAGNOSTIC measured_storage=true phase_cpu=false
CONFIG callers=1000 rounds=1000 payload=256 records_per_submit=1 seed=none_literal_formula linger_ns=1000000 group_byte_cap=1048576 roll_bytes=67108864 queue_capacity=4096 pending_byte_budget=16777216
METRIC device_fdatasync_ns count=1000 min=229998 p50=319804 p99=1476335 p999=2372774 max=4338299 sum=453552767
DEVICE start_bytes=0 end_bytes=262144000 append_bytes=262144 failures=0
OPEN initial_ns=869446 records=0 segments=1 bytes=64
APPEND records=1000000 elapsed_ns=3451214638 records_per_second=289753.059 full=0 other_admission_errors=0 commit_errors=0 correctness_errors=0 peak_reserved_bytes=544000 admission_call_sum_ns=615820747
METRIC append_ns count=1000000 min=276225 p50=3390601 p99=5560648 p999=7429191 max=10245849 sum=3421693928030
METRIC group_submits count=1999 min=1 p50=501 p99=819 p999=998 max=1000 sum=1000000
METRIC group_frame_bytes count=1999 min=336 p50=148336 p99=242464 p999=295448 max=296040 sum=296079960
METRIC group_fdatasync_ns count=1999 min=122902 p50=518073 p99=1130624 p999=3662429 max=4901974 sum=1030232588
STORAGE kind=Group.Append count=1999 sum_ns=193154272
STORAGE kind=Group.SyncData count=1999 sum_ns=1030232588
STORAGE kind=KeyIndex.Other count=526 sum_ns=7563044
STORAGE kind=LogFilter.Other count=84 sum_ns=4725617
STORAGE kind=Lookup.Other count=2006 sum_ns=216617250
STORAGE kind=Open.Other count=7 sum_ns=169819
STORAGE kind=Open.SyncData count=1 sum_ns=178107
STORAGE kind=Open.SyncDir count=1 sum_ns=3012
STORAGE kind=Recovery.Other count=2 sum_ns=4434
STORAGE kind=Recovery.SyncAll count=1 sum_ns=84265
STORAGE kind=Recovery.SyncDir count=2 sum_ns=3125
STORAGE kind=Roll.Append count=16 sum_ns=9202866
STORAGE kind=Roll.Other count=8 sum_ns=1735400
STORAGE kind=Roll.SyncData count=12 sum_ns=14944871
STORAGE kind=Roll.SyncDir count=4 sum_ns=24755
END records=1000000 segments=5 bytes=312662592
ROLL index=0 writer_ns=733389
ROLL index=1 writer_ns=1046918
ROLL index=2 writer_ns=2106632
ROLL index=3 writer_ns=394105
REOPEN ns=36053664 start_records=1000000 end_records=1000000 start_segments=5 end_segments=5 start_bytes=312662592 end_bytes=312662592 correctness_errors=0
test acceptance::evidence::measured_appends_survive_reopen ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 321 filtered out; finished in 6.15s
```
