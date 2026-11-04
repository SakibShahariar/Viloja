# Commit CI Audit Ledger

Persistent repository-specific record of commit-level CI checks and red-cross investigations.

## How to use this ledger

1. Read this file before auditing commit checks.
2. Check only commits newer than the checkpoint, plus older **red**, **pending**, or explicitly unresolved entries.
3. Record exact commit SHAs and observed results; never infer from missing badges or empty API responses.
4. Preserve failures and append diagnosis, fix, rerun, and final-outcome evidence.
5. Advance the checkpoint only after reviewing the range; carry unresolved entries forward.

## Result meanings

| Result | Meaning |
|---|---|
| **green** | Completed successful check/run directly observed. |
| **red** | Completed failing check/run directly observed. |
| **pending** | Check/run exists but is unfinished. |
| **unavailable** | Available evidence did not expose a result; not a pass or failure. |
| **not applicable** | Workflow configuration confirms no relevant check is triggered. |

## Checkpoint

- Repository: `SakibShahariar/Viloja`
- Default branch: `main`
- Checkpoint SHA: `644f438425f87e3e0dd4682a983392e2a9fae215`
- Initialized: 2026-11-02
- Baseline: latest branch tip queried through combined commit-status endpoint; empty status list recorded as **unavailable**, not green/red. Older history was not retroactively audited during setup.
- Next session: review commits after `644f438425f87e3e0dd4682a983392e2a9fae215`; carry unresolved red/pending entries forward.

## Initial baseline

| Commit SHA | Subject | CI result | Checked | Follow-up |
|---|---|---|---|---|
| `b3bcaef93d7dc12224fd604d6f8d59289f9a8ddf` | Default-branch tip at ledger initialization | unavailable | 2026-11-02 | Combined status lookup returned no entries. Revisit if new run or visible failure evidence appears. |

## Red-cross investigation log

No red result was confirmed from the status data retrieved during setup.

### Failure record template

- Commit SHA:
- Commit subject:
- First observed (date):
- Workflow / run URL:
- Failed job / step:
- Failure summary:
- Classification: regression / pre-existing / inconclusive
- Fix commit(s):
- Rerun attempt(s) and result:
- Final verified outcome:
- Remaining follow-up:

## Session history

| Date | Reviewed range / checkpoint | Result summary |
|---|---|---|
| 2026-11-02 | Initial checkpoint: `b3bcaef93d7dc12224fd604d6f8d59289f9a8ddf` | Latest commit queried; empty status response recorded as unavailable. Earlier history not retroactively audited. |


## Incremental audit: 2026-11-04

Reviewed the range after the previous checkpoint through `cb43463328feb0706466308595391b6fc5f401b9`. Every commit in this range has a completed successful Rust CI run.

| Commit SHA | Subject | CI result | Workflow run |
|---|---|---|---|
| `4263cda729e2dcd96bebe2b7deed6e15d875518a` | docs: add persistent commit CI audit ledger | green | [41456803288](https://github.com/SakibShahariar/Viloja/actions/runs/41456803288) |
| `77cb34cdfdb32f1ff0f0a02ea656b2cfb4b67eb5` | docs: document incremental commit CI auditing | green | [41456810696](https://github.com/SakibShahariar/Viloja/actions/runs/41456810696) |
| `d2c40f67870b4bf474a5a3a53e8954c42beef344` | docs: define issue triage categories | green | [41573542782](https://github.com/SakibShahariar/Viloja/actions/runs/41573542782) |
| `cb43463328feb0706466308595391b6fc5f401b9` | docs(agents): define administrative commit hygiene | green | [41597839910](https://github.com/SakibShahariar/Viloja/actions/runs/41597839910) |

### Resolved historical red runs

- Commit `8fab1d664fe72d79d05be1de227820d939fa6e1a`, run [41306883921](https://github.com/SakibShahariar/Viloja/actions/runs/41306883921): a backend fake test incorrectly expected a device to be reachable after a simulated disconnect and restart. Commit `157f7a9501839887e1cef35af892fde8699ef0b5` corrected the assertion; run [41306921374](https://github.com/SakibShahariar/Viloja/actions/runs/41306921374) passed.
- Commit `30f2bf68642e5fa82f7b963fdbdc0a1c6800bac0`, run [41307310826](https://github.com/SakibShahariar/Viloja/actions/runs/41307310826): compilation used the wrong type for Gio signal subscription IDs. Commit `277f7be82a232fad20c6398c6c7496938b759803` corrected the stored type to `gio::SignalSubscriptionId`; run [41308393587](https://github.com/SakibShahariar/Viloja/actions/runs/41308393587) passed.

Both failures were investigated from their job logs and their fixes were verified by later successful runs. No unresolved red or pending run remains in the reviewed range.


### Async backend contract follow-up

| Commit SHA | Subject | CI result | Workflow run | Notes |
|---|---|---|---|---|
| `95fefa4a6fd20a1d16c9426e7f000b69331beb6f` | feat(backend): expose object-safe async operations (#7) | red | [41836829073](https://github.com/SakibShahariar/Viloja/actions/runs/41836829073) | Initial patch contained a malformed duplicate function boundary; Rust reported an unexpected closing delimiter. |
| `38f5edde4ac793923107b62a3c6db134ef500eea` | fix(backend): correct async contract test module (#7) | green | [41836990640](https://github.com/SakibShahariar/Viloja/actions/runs/41836990640) | Removed the malformed block and corrected the misplaced API documentation; cargo check, cargo-deny, and all 34 tests passed. |

The next checkpoint remains `38f5edde4ac793923107b62a3c6db134ef500eea`; review newer commits and their final CI results in the next incremental audit.


| Commit SHA | Subject | CI result | Workflow run | Notes |
|---|---|---|---|---|
| `7f1e23e55e653b4d3aea2a7997104d6404024094` | test(backend): verify async operation cancellation (#7) | green | [41837436067](https://github.com/SakibShahariar/Viloja/actions/runs/41837436067) | cargo check, cargo-deny, and all 35 Rust tests passed. |


| Commit SHA | Subject | CI result | Workflow run | Notes |
|---|---|---|---|---|
| `432760a9525b59fed4f7158a6c82babf71e8e49b` | feat(backend): add pairing operations to shared contract (#7) | green | [41837966329](https://github.com/SakibShahariar/Viloja/actions/runs/41837966329) | Added async pair/unpair operations and device update events; cargo check, cargo-deny, and all 36 Rust tests passed. |


| Commit SHA | Subject | CI result | Workflow run | Notes |
|---|---|---|---|---|
| `1446db7e3de1a7b41d95b9da4f6cba6722c2aa37` | test(backend): cover unsupported pairing capability (#7) | green | [41838512434](https://github.com/SakibShahariar/Viloja/actions/runs/41838512434) | cargo check, cargo-deny, and all 37 Rust tests passed. |


### Typed backend error model

| Commit SHA | Subject | CI result | Workflow run | Notes |
|---|---|---|---|---|
| `a3659eb99bb24dc7265659d616779e9ed7a1304e` | feat(backend): add typed safe error guidance (#18) | red | [41839210100](https://github.com/SakibShahariar/Viloja/actions/runs/41839210100) | Two remaining fake-backend error literals were not migrated to the new typed constructor. |
| `186fedf1b7573a569b3091b0fc25361362b53c06` | fix(backend): complete typed error migration (#18) | green | [41839357369](https://github.com/SakibShahariar/Viloja/actions/runs/41839357369) | Completed the remaining constructors; cargo check, cargo-deny, and all 38 Rust tests passed. |


| Commit SHA | Subject | CI result | Workflow run | Notes |
|---|---|---|---|---|
| `f0ea148f90b602890d4a97940908c5142fe29f3f` | test(backend): cover all safe error categories (#18) | green | [41839812449](https://github.com/SakibShahariar/Viloja/actions/runs/41839812449) | cargo check, cargo-deny, and all 38 Rust tests passed; verified fixed user messages do not expose raw details. |


| Commit SHA | Subject | CI result | Workflow run | Notes |
|---|---|---|---|---|
| `3ab307e5a8cb3af776e4cff6bee00dae2251e6c0` | docs(security): record typed backend error privacy boundary (#18) | green | [41840304968](https://github.com/SakibShahariar/Viloja/actions/runs/41840304968) | cargo check, cargo-deny, and all 38 Rust tests passed. |


### Privacy-safe discovery errors

| Commit SHA | Subject | CI result | Workflow run | Notes |
|---|---|---|---|---|
| `b8a41cf7c62d8e94fd6472447b5cde3cfc14364c` | fix(privacy): hide raw Valent errors and device IDs (#18) | green | [41842032602](https://github.com/SakibShahariar/Viloja/actions/runs/41842032602) | cargo check, cargo-deny, and all 38 Rust tests passed. Raw D-Bus errors and device IDs are no longer exposed in discovery UI. |


### Incremental audit: 2026-11-04 (Valent extraction)

Reviewed commits after checkpoint `b8a41cf7c62d8e94fd6472447b5cde3cfc14364c` through `462e0968aea18e26b3cdfe9c86e101e11e2f0f15`.

| Commit SHA | Subject | CI result | Workflow run | Notes |
|---|---|---|---|---|
| `6301411a8452b26a067e55d0fcd3a8422dc930b8` | chore: remove unused D-Bus error binding (#18) | green | [41842589086](https://github.com/SakibShahariar/Viloja/actions/runs/41842589086) | cargo check, cargo-deny, and Rust tests completed successfully. |
| `30105f12936ecb9ff87a2e29c01ff1e191e59bbe` | refactor(valent): isolate discovery protocol model (#7) | red | [41851578423](https://github.com/SakibShahariar/Viloja/actions/runs/41851578423) | First extraction attempt left Valent constants private to the child module and retained an unused import. Cargo check failed with four E0603 errors. |
| `6007d22890d133237f0d574ea901a2993e892cab` | fix(valent): expose protocol constants to app layer (#7) | green | [41851761633](https://github.com/SakibShahariar/Viloja/actions/runs/41851761633) | Made protocol constants crate-visible and removed the unused import; cargo check, cargo-deny, and all 38 tests passed. This resolves the previous commit's compile regression. |
| `462e0968aea18e26b3cdfe9c86e101e11e2f0f15` | refactor(valent): move ObjectManager query behind adapter module (#7) | green | [41852357635](https://github.com/SakibShahariar/Viloja/actions/runs/41852357635) | Moved the ObjectManager method call into the Valent module; cargo check, cargo-deny, and all 38 tests passed. |

The extraction is still incomplete: D-Bus session-bus acquisition, signal subscriptions, and name watching are being moved behind the Valent module; the GTK frontend must ultimately consume the shared `SharingBackend` contract. No issue acceptance checkbox is marked complete yet.

- Checkpoint: `462e0968aea18e26b3cdfe9c86e101e11e2f0f15`
- Unresolved red entries in this reviewed range: `30105f12936ecb9ff87a2e29c01ff1e191e59bbe` (corrected by `6007d22890d133237f0d574ea901a2993e892cab`).


### Incremental audit: 2026-11-04 (Valent D-Bus boundary)

Reviewed commits after checkpoint `462e0968aea18e26b3cdfe9c86e101e11e2f0f15` through `644f438425f87e3e0dd4682a983392e2a9fae215`.

| Commit SHA | Subject | CI result | Workflow run | Notes |
|---|---|---|---|---|
| `4719abf7da0b617f7fb3d7183591fd175a08ad16` | refactor(valent): isolate D-Bus discovery plumbing (#7) | red | [41853180871](https://github.com/SakibShahariar/Viloja/actions/runs/41853180871) | Unresolved signal-cleanup helper in GTK code and incorrect watcher-handle type. |
| `22503fb859a2a7247e053a9ab779a41d0c74aa28` | fix(valent): route cleanup through adapter module (#7) | red | [41853355659](https://github.com/SakibShahariar/Viloja/actions/runs/41853355659) | Close-handler cleanup remained unresolved; watcher handle still used mismatched GIO types. |
| `ce8ec8bed0d5a0c371f93d254dda565c40f807c3` | fix(valent): complete signal cleanup migration (#7) | red | [41853468384](https://github.com/SakibShahariar/Viloja/actions/runs/41853468384) | Cleanup/imports were corrected, but watcher-handle types still did not match GIO. |
| `ecd3f4441fddef043060ea836f03c2d21449657d` | fix(valent): use the D-Bus watcher identifier type (#7) | red | [41853619669](https://github.com/SakibShahariar/Viloja/actions/runs/41853619669) | The watcher identifier is not publicly nameable through `gio::WatcherId`; unwatch and stored-handle types remained incompatible. |
| `d9dd0ede63caa2e5e7ec1c549ac82d9651f2b453` | fix(valent): wrap the D-Bus watcher handle type (#7) | red | [41853959087](https://github.com/SakibShahariar/Viloja/actions/runs/41853959087) | First wrapper attempt had a mismatched closing delimiter and incomplete unwatch conversion. |
| `5a199b187cd42218e1b29146b57c562b2eb8846b` | fix(valent): pass the wrapped watcher handle to GIO (#7) | red | [41854043568](https://github.com/SakibShahariar/Viloja/actions/runs/41854043568) | The underlying watcher type is inside a private GIO module and cannot be named by this crate. |
| `644f438425f87e3e0dd4682a983392e2a9fae215` | fix(valent): hide the private GIO watcher handle (#7) | green | [41854318284](https://github.com/SakibShahariar/Viloja/actions/runs/41854318284) | Wrapped the opaque watcher ID in a one-shot unwatch closure; cargo check, cargo-deny, and all 38 Rust tests passed. |

#### Resolved CI regressions

All red entries in this range were regressions introduced during the D-Bus boundary extraction. The successive fixes corrected the GTK cleanup references, moved the watcher handle behind a closure so the private GIO type is never named, and closed the watcher through that wrapper. Commit `644f438425f87e3e0dd4682a983392e2a9fae215` is the final correction; its associated run completed successfully with 38/38 tests. No red result remains unresolved in this range.

- Checkpoint: `644f438425f87e3e0dd4682a983392e2a9fae215`
