# Security review

Status: **partial static review of the current prototype**. This document is not a security certification and does not claim that file sharing is safe or complete.

Review basis: Viloja's current GTK4/libadwaita prototype, Valent session-D-Bus discovery, backend/transfer contracts, and privacy-safe diagnostics. The review must be repeated when real sending or receiving code is added.

## Current security boundary

- Viloja currently queries the user's session D-Bus for Valent device objects and displays discovered device names and connection state.
- Pairing and trust management remain the responsibility of Valent. Viloja does not implement identity verification or pairing.
- Viloja does not currently receive files, write received files, or implement an inbound-transfer approval flow.
- Viloja does not currently implement its own encryption or transport protocol. Transport security is delegated to the external backend and has not been independently verified here.
- The backend transfer types and fake contract tests are models for future integration; they are not evidence that real transfers or filesystem cleanup work.

## Findings

### 1. Inbound approval and file handling — not implemented

No production inbound-transfer handler or destination-file writer is present in the current prototype. Therefore the current application cannot auto-accept an inbound transfer through Viloja, but it also cannot yet provide a working explicit approval workflow.

Path traversal rejection, filename validation, collision handling, safe file creation, and cleanup of partial files are **not implemented or testable** until a real receive path exists. Do not treat the absence of a receiver as proof that these future safeguards have passed.

### 2. Pairing, identity, and encryption — delegated / unverified

Pairing is managed in Valent, not by Viloja. The current source does not verify peer identities or implement encryption itself. The exact security properties depend on the installed Valent version and its protocol implementation; this review has not validated those properties on a live device.

### 3. Session D-Bus discovery

The application queries the session bus for a fixed Valent service name and ObjectManager path. It reads exported device properties and renders device names and connection state. Device IDs are currently included in a widget tooltip for diagnostics of the discovery UI. Treat those identifiers as potentially sensitive and do not copy them into diagnostic logs.

A missing service, failed session-bus connection, or failed discovery call is surfaced as an unavailable/error state. Automatic service appearance/disappearance recovery is implemented, but still requires runtime validation in a GNOME session.

### 4. Diagnostic logging

The diagnostic logger accepts a closed set of event variants rather than arbitrary backend error strings, device names, addresses, tokens, or file paths. Discovery completion records only a numeric device count. The in-memory log is bounded to 500 entries.

Verbosity is session-only. Turning logging off prevents new events from being recorded; it does not erase entries already in memory. The Settings page provides a Clear diagnostics action to erase the in-memory entries and a Refresh diagnostics action to update the visible log. The copy/export action can still export earlier entries until they are cleared. Users should review copied diagnostics before sharing them.

## Required controls before inbound transfers ship

- [ ] Require an explicit user decision for every incoming transfer; default to no acceptance.
- [ ] Validate every received filename as a single safe path component. Reject absolute paths, separators, traversal components, control characters, and platform-reserved names.
- [ ] Resolve and validate the destination directory; defend against symlink and time-of-check/time-of-use path escapes.
- [ ] Create destination files without following symlinks and without overwriting existing files. Define predictable collision behavior.
- [ ] Write to uniquely created temporary files with restrictive permissions; atomically finalize only after successful verification.
- [ ] Remove temporary and partial files after rejection, failure, cancellation, disconnect, and application shutdown.
- [ ] Enforce reasonable file-count and size limits and handle disk-full / permission errors.
- [ ] Redact identifiers, paths, file names, addresses, tokens, and raw protocol errors from diagnostics by default.
- [ ] Add tests against traversal strings, symlink races, collisions, partial writes, cancellation, disconnect, and restart.
- [ ] Review the actual backend's pairing, identity verification, and encryption behavior and record its exact version.

## Validation still required

This is a source-level review only. It does not replace tests with an installed Valent service, a paired Android device, or a live GNOME session. Revisit the checklist as each real transfer capability is implemented. Keep issue #36 open until the required filesystem and lifecycle protections exist and are tested.
