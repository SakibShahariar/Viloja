# Integration proof-of-concept plan

Status: planned, not yet executed. This document describes the smallest experiments to run before committing Viloja to a backend.

## Decision gates

Do not treat a successful build as proof of a working backend. Each candidate must pass the real-device transfer gates below, and the licensing gate must pass before any dependency is adopted or distributed.

## POC A — Valent over the user session D-Bus

### Why this route

Valent's source exports each device as a D-Bus object and exports the device's `GActionGroup` at the same object path. Plugin actions are namespaced, for example `share.files` or `share.uri`. This may let Viloja use Valent as a separately installed service without linking its C libraries into the Rust process.

Source references:
- [Valent device manager](https://github.com/andyholmes/valent/blob/main/src/libvalent/device/valent-device-manager.c) — device object and action-group export.
- [Valent device](https://github.com/andyholmes/valent/blob/main/src/libvalent/device/valent-device.c) — actions are aggregated from plugins.
- [Valent device plugin](https://github.com/andyholmes/valent/blob/main/src/libvalent/device/valent-device-plugin.c) — plugin action names and D-Bus export behavior.

### Runtime investigation on Fedora GNOME

1. Install/enable Valent through a trusted package source, pair an Android device, and verify a file transfer manually in Valent first.
2. Find the actual session-bus name and object-manager path for the installed Valent build. Do not hard-code a guessed bus name:
   ```fish
   busctl --user list | grep -i valent
   ```
3. Introspect the discovered service and object-manager path with `gdbus introspect --session --dest SERVICE_NAME --object-path OBJECT_PATH`. Replace placeholders with values found in step 2.
4. Enumerate exported device objects and their properties. Confirm which property/state represents a paired, connected device.
5. On a device object, query its exported action group and inspect each action's parameter signature before invoking anything. Confirm the installed build actually exposes a share action and determine whether it accepts a URI, a file list, or another parameter type.
6. Use a harmless, non-sensitive test file. Invoke the share action only after checking its signature and enabled state. Confirm the Android device receives it.
7. Send a file from Android to the laptop and observe Valent's incoming-share path. Determine whether incoming transfers can be observed through D-Bus or whether the initial Viloja integration must rely on Valent's UI/notification flow.
8. Repeat with an unpaired device, a disconnected device, a rejected transfer, a missing file, and service shutdown. Capture errors and make sure Viloja can recover if Valent restarts.

### Pass criteria

- Discover devices without parsing localized UI text.
- Reliably distinguish paired/connected from unavailable devices.
- Send a local file with the correct action parameter signature.
- Document incoming-file behavior and its limitations.
- Detect missing service, device disconnect, rejected transfer, and service restart without crashing.
- Confirm behavior on the actual packaged version intended for users.

**Likely limitation:** this is an integration with Valent, not a standalone KDE Connect implementation. Viloja must explain clearly when Valent is missing or not paired, and must not imply it owns the underlying connection.

## POC B — rquickshare Rust library

### What source inspection confirms

The library crate is named `rqs_lib` (under `core_lib`). Its public API includes `RQS`, `EndpointInfo`, `Visibility`, `SendInfo`, `OutboundPayload`, and `ChannelMessage`. `RQS::run()` starts the listener and returns a sender for outbound requests. Discovery reports endpoints through a broadcast channel. Transfer messages distinguish library-to-UI events from UI-to-library actions; inbound transfer actions include accept, reject, and cancel, and outbound cancellation is also represented.

Source references:
- [Public runtime API](https://github.com/Martichou/rquickshare/blob/master/core_lib/src/lib.rs)
- [Outbound request model](https://github.com/Martichou/rquickshare/blob/master/core_lib/src/manager.rs)
- [Transfer event/action model](https://github.com/Martichou/rquickshare/blob/master/core_lib/src/channel.rs)
- [Outbound protocol implementation](https://github.com/Martichou/rquickshare/blob/master/core_lib/src/hdl/outbound.rs)
- [Inbound protocol implementation](https://github.com/Martichou/rquickshare/blob/master/core_lib/src/hdl/inbound.rs)
- [Library manifest and features](https://github.com/Martichou/rquickshare/blob/master/core_lib/Cargo.toml)

### Harness shape

Keep this experiment separate from the application UI. Use a small Rust CLI with a Tokio runtime and structured logs. It should:

1. Create `RQS` with an explicit download directory and an initial visibility setting.
2. Subscribe to transfer messages before starting transfers; start the runtime and discovery, then print discovered endpoint details with identifiers redacted where appropriate.
3. Present a deliberate confirmation step before accepting an inbound transfer. Save incoming files only to a dedicated temporary test directory.
4. Send one small, non-sensitive file to a selected endpoint using the exact public `SendInfo` / `OutboundPayload::Files` types and the endpoint's discovered address/ID.
5. Surface transfer state, completion, rejection, cancellation, and disconnect events from `ChannelMessage`; correlate events by transfer ID rather than assuming message order.
6. Stop discovery and call the runtime's asynchronous shutdown method on exit. Test Ctrl-C and receiver/channel closure.
7. Record compiler output, dependency tree, protocol logs with secrets and identifiers redacted, and Android/device/network versions.

Use the crate's own current example/application as the source of truth for constructing endpoint and send values; do not guess fields or invent a wrapper around private internals.

### Pass criteria

- Builds reproducibly on the supported Fedora/Rust toolchain.
- Discovers a real Android Quick Share endpoint on the same local network.
- Sends and receives a file end-to-end.
- Requires explicit approval for incoming transfers.
- Handles reject, cancel, disconnect, malformed/unexpected messages, and shutdown.
- No panic, leaked task, or leftover partial file after failure.
- License and transitive dependency review passes before adoption.

### Known constraints and blockers

- The library manifest enables its `experimental` feature by default; this activates optional Bluetooth-related dependencies. Test both default and intentionally selected feature sets rather than assuming Bluetooth is required or harmless.
- The documented implementation path is Wi-Fi LAN. A successful same-network test does not establish hotspot, Wi-Fi Direct, or Bluetooth file-transfer support.
- The crate is GPL-3.0 according to the project's license metadata. Viloja's distribution and linking strategy must be chosen with that obligation in mind; do not copy code or ship the dependency before the licensing review.
- The current public API appears designed around an event-driven UI. Confirm whether it is sufficiently stable and whether errors are observable enough for a polished GNOME app.

## Compare results before choosing

| Gate | Valent D-Bus | rquickshare |
| --- | --- | --- |
| Build/integration | Runtime dependency and D-Bus availability | Rust dependency, native system/dependency requirements |
| Discovery | Exported paired/connected device objects | Quick Share endpoint discovery |
| Outbound file | Share action and its signature | `SendInfo` with file payload |
| Inbound file | Must determine D-Bus/UI/notification limitations | Accept/reject actions and download path |
| Cancellation/error recovery | D-Bus action/service/device lifecycle | Transfer state messages and runtime cancellation |
| User experience | May depend on Valent being installed and configured | Can be integrated more directly in Viloja |
| License gate | GPL-3.0-or-later compatibility review | GPL-3.0 compatibility review |
| Router-free connectivity | Unknown until tested with the specific setup | Not established by the documented Wi-Fi-LAN path |

## Result log template

For each test, record:

- Date and Viloja commit:
- Backend and exact version/commit:
- Fedora version, kernel, Rust/toolchain (for rquickshare):
- Android version and Quick Share/KDE Connect app version:
- Wi-Fi adapter and driver:
- Network topology (same router, no internet, laptop hotspot, etc.):
- Test case and file size:
- Result: pass / fail / blocked / not tested:
- Logs and error summary (redact names, IP/MAC addresses, tokens, and personal file paths):
- Follow-up issue:

Until these tests pass, keep both integrations experimental and do not advertise router-free operation.


## Preparing hardware investigations without claiming support

Issues #31, #32, and #33 require the actual development machine and Android
device. The repository now includes
`scripts/collect-connectivity-capabilities.sh` to collect a repeatable Linux
preflight report for those tests:

```sh
bash scripts/collect-connectivity-capabilities.sh
```

The report records the kernel/distribution, PCI and USB adapter/driver
information, Wi-Fi supported interface modes, radio block state, and detected
Bluetooth controllers. MAC addresses are redacted in the script output. Review
the output before sharing it, because host or device names may still be
identifying.

This is preparation only. It does not test an Android peer, prove hotspot or
Wi-Fi Direct interoperability, establish Bluetooth transfer support, or
satisfy the real-device acceptance criteria for issues #31–#33. Those results
must remain **not tested** until the actual hardware experiments are run.
