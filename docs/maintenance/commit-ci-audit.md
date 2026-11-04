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
- Checkpoint SHA: `a573874643d160008f1e6b31ef6d6c4f7ac22552`
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


### Incremental audit: 2026-11-04 (adapter boundary documentation)

Reviewed commit `a573874643d160008f1e6b31ef6d6c4f7ac22552` after the previous checkpoint.

| Commit SHA | Subject | CI result | Workflow run | Notes |
|---|---|---|---|---|
| `a573874643d160008f1e6b31ef6d6c4f7ac22552` | docs(architecture): record Valent adapter boundary (#7) | green | [41855215011](https://github.com/SakibShahariar/Viloja/actions/runs/41855215011) | The cargo-check job completed successfully: Rust check, cargo-deny, and 38/38 tests passed. The commit documents that GTK still orchestrates discovery through Valent-specific helpers and the shared `SharingBackend` contract is not yet wired into the frontend. |

- Checkpoint: `a573874643d160008f1e6b31ef6d6c4f7ac22552`
- Unresolved red entries in this reviewed range: none.


### Incremental audit: 2026-11-04 (shared device contract)

Reviewed every commit after checkpoint `a573874643d160008f1e6b31ef6d6c4f7ac22552` through `fdc2a17bfc1925ff242b17ea90eeff234152d349`. Push-triggered Rust CI runs were queried by exact head SHA; results below are the completed run conclusions.

| Commit SHA | Subject | CI result | Workflow run | Notes |
|---|---|---|---|---|
| `69d6dd0eba269aba14f473abe2f69a7de6e9ced8` | docs(maintenance): record adapter boundary CI verification | green | [41862409662](https://github.com/SakibShahariar/Viloja/actions/runs/41862409662) | Completed successfully. |
| `1b98c69e1ed89e940b8ed17cf233d55575c4fced` | chore(license): adopt GPL-3.0-or-later (#38) | green | [41862972264](https://github.com/SakibShahariar/Viloja/actions/runs/41862972264) | Completed successfully. |
| `ecc726b1328f337e0f94ba2b5d2a471e9c2fcef2` | docs(readme): identify GPL-3.0-or-later license (#38) | green | [41862979658](https://github.com/SakibShahariar/Viloja/actions/runs/41862979658) | Completed successfully. |
| `600abbf9888e1c8a16d24985f6080963d75cae06` | chore(cargo): declare GPL-3.0-or-later metadata (#38) | green | [41862986025](https://github.com/SakibShahariar/Viloja/actions/runs/41862986025) | Completed successfully. |
| `057fe365470ba248a0464b7038d484aabe0b3098` | docs(maintenance): record project GPL license (#38) | green | [41862993746](https://github.com/SakibShahariar/Viloja/actions/runs/41862993746) | Completed successfully. |
| `12ce3835c960286905df461880970cf9e8f766c9` | docs(agents): enforce evidence-based acceptance criteria | green | [41863813814](https://github.com/SakibShahariar/Viloja/actions/runs/41863813814) | Completed successfully. |
| `d478354fd90b3c41cae41ad76797b63aa900a44b` | feat(valent): map discovery into backend contract (#7) | red | [41874616019](https://github.com/SakibShahariar/Viloja/actions/runs/41874616019) | Cargo check rejected a misplaced derive attribute applied to a constant. Corrected by the following commit; failure details were inspected from the job log. |
| `1794a7e3e13adc1c1d8349445ba4b9fdd859edfb` | fix(valent): keep device metadata derive on its type (#7) | green | [41874670943](https://github.com/SakibShahariar/Viloja/actions/runs/41874670943) | cargo check, cargo-deny, and all 42 Rust tests passed. |
| `fdc2a17bfc1925ff242b17ea90eeff234152d349` | refactor(ui): render shared backend device models (#7) | green | [41874987009](https://github.com/SakibShahariar/Viloja/actions/runs/41874987009) | cargo check, cargo-deny, and all 42 Rust tests passed. GTK discovery rows and selection pruning now consume the shared `backend::Device` model; Valent still owns discovery orchestration. |

#### Resolved CI regression

The only red run in this range was introduced by `d478354fd90b3c41cae41ad76797b63aa900a44b`. Its `#[derive(Debug, Clone)]` attribute was accidentally left above the new backend identifier constant instead of the `ValentDeviceInfo` struct. Commit `1794a7e3e13adc1c1d8349445ba4b9fdd859edfb` moved the derive attribute back to the struct; the associated run passed cargo check, cargo-deny, and 42/42 tests. The subsequent UI model migration also passed all checks and tests.

- Checkpoint: `fdc2a17bfc1925ff242b17ea90eeff234152d349`
- Unresolved red or pending entries in this reviewed range: none.


### Incremental audit: 2026-11-04 (snapshot-driven capability presentation)

Reviewed commit `0deef819c6de7f21562debb59f3fa6377d288224` after the previous checkpoint.

| Commit SHA | Subject | CI result | Workflow run | Notes |
|---|---|---|---|---|
| `0deef819c6de7f21562debb59f3fa6377d288224` | refactor(ui): derive capabilities from backend snapshot (#8) | green | [41875662617](https://github.com/SakibShahariar/Viloja/actions/runs/41875662617) | cargo check, cargo-deny, and all 42 Rust tests passed. Capability presentation now derives from the shared backend snapshot. |

- Checkpoint: `0deef819c6de7f21562debb59f3fa6377d288224`
- Unresolved red or pending entries in this reviewed range: none.


### Incremental audit: 2026-11-04 (Valent shared adapter)

Reviewed commits after checkpoint `0deef819c6de7f21562debb59f3fa6377d288224` through `5c1360941402e769ae99ae7a9010aed43cca49ad`.

| Commit SHA | Subject | CI result | Workflow run | Notes |
|---|---|---|---|---|
| `4556361708da56e39ebc5e7eaf39282699615853` | feat(valent): implement shared discovery backend adapter (#7) | red | [41876227590](https://github.com/SakibShahariar/Viloja/actions/runs/41876227590) | Cargo check found the D-Bus connection was moved into the refresh closure and then borrowed for signal subscription. |
| `025f32b13859c991132d8fe36b22b528c322e591` | fix(valent): pass borrowed D-Bus reply type (#7) | red | [41876279265](https://github.com/SakibShahariar/Viloja/actions/runs/41876279265) | The reply type borrow was corrected, but the connection was still moved into the closure; the same E0382 remained. |
| `5c1360941402e769ae99ae7a9010aed43cca49ad` | fix(valent): retain connection for signal subscriptions (#7) | green | [41876486509](https://github.com/SakibShahariar/Viloja/actions/runs/41876486509) | cargo check, cargo-deny, and all 45 Rust tests passed. The refresh closure now owns a cloned connection while the adapter retains the original for subscriptions. |

#### Resolved CI regressions

Both red runs were caused by the refresh closure taking ownership of the same connection later needed to register signal subscriptions. The final correction retains a separate cloned connection for the callback and borrows the original for subscription setup. The final run passed all checks and 45/45 tests.

- Checkpoint: `5c1360941402e769ae99ae7a9010aed43cca49ad`
- Unresolved red or pending entries in this reviewed range: none.
