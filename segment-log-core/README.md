# segment-log-core

Embedded append-only segment log for Ironflow X. The byte layout is format version 4, which adds the close frame to version 3, the baseline of 0.1, the first production release. From the 0.1 tag, every later build reads every format version from 3, and the API follows 0.x semver ([ADR 0064](https://github.com/ironflowx/segment-log/blob/v0.3.0/docs/decisions/0064-the-0-1-compatibility-promise.md)).

The API's meaning, errors and defaults are in [api.md](https://github.com/ironflowx/segment-log/blob/v0.3.0/docs/architecture/api.md), the only source for them.

With the `simulation` feature, a deterministic simulator can supply the storage and the clock, and run the log's background work as steps it schedules ([ADR 0072](https://github.com/ironflowx/segment-log/blob/v0.3.0/docs/decisions/0072-a-simulator-drives-the-log-through-public-storage-a-clock-and-steps.md)).
