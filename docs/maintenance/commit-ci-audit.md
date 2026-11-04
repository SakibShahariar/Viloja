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
- Checkpoint SHA: `b8a41cf7c62d8e94fd6472447b5cde3cfc14364c`
- Initialized: 2026-11-02
- Baseline: latest branch tip queried through combined commit-status endpoint; empty status list recorded as **unavailable**, not green/red. Older history was not retroactively audited during setup.
- Next session: review commits after this checkpoint, including the ledger and `AGENTS.md` setup commits; investigate any older red/pending commit specifically identified by the owner.

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
