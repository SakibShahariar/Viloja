# Architecture direction

This document describes the intended boundaries. It is not a claim that these components have been implemented.

## Design principles

1. **One application, multiple independent backends.** Shared UI does not imply a shared wire protocol.
2. **Capabilities are explicit.** The UI asks each backend what it can do instead of assuming feature parity.
3. **Security decisions stay visible.** Pairing, identity verification, incoming-transfer approval, and trust state must not be hidden behind convenience abstractions.
4. **Graceful degradation.** An unavailable backend or unsupported transport should not prevent other backends from working.
5. **GNOME-first.** Prefer GTK4, libadwaita, GNOME platform conventions, accessibility, and system color-scheme support.

## Proposed layers

### 1. GNOME application layer

Owns navigation, device and transfer presentation, notifications, preferences, and user-facing error messages. It should not implement protocol-specific networking.

### 2. Backend interface

A backend should expose a consistent application-level contract for:

- Starting and stopping discovery.
- Reporting device updates and backend status.
- Reporting supported actions and transport capabilities.
- Sending files and receiving incoming-transfer requests.
- Reporting progress and terminal outcomes.
- Cancelling operations when the underlying implementation safely supports it.

The interface should represent unsupported operations explicitly rather than silently emulating them.

### 3. Independent backend adapters

Each adapter owns the details of its integration, including discovery, protocol messages, authentication, encryption, transfer negotiation, networking, and protocol-specific errors. Initial candidates are Android Quick Share interoperability and KDE Connect.

Implementation choices—native Rust libraries, a maintained external process, or another integration boundary—must follow a source, licensing, maintenance, and security audit. Avoid duplicating a protocol implementation without a clear reason.

### 4. Shared transfer model

The shared model can normalize presentation fields such as sender/device label, file metadata, progress, timestamps, and outcome. It must not erase backend-specific security state or imply that two protocols have interchangeable trust models.

### 5. Platform services

Investigate GNOME notifications, file chooser and destination handling, desktop actions, and optional Nautilus integration. These are enhancements, not prerequisites for the first proof of concept.

## Initial milestone sequence

1. Document supported use cases and non-goals.
2. Audit candidate projects, protocol documentation, licenses, and dependencies.
3. Prove one backend can discover a device and complete a transfer.
4. Define the backend interface from real requirements.
5. Add the second backend without coupling its lifecycle to the first.
6. Build polished shared flows, error states, and settings.
7. Add optional shell/file-manager integration only after core transfers are reliable.

## Open questions

- Which maintained libraries or companion processes can legally and reliably provide each protocol?
- What permissions, background services, and firewall behavior are required?
- Which connection paths work without a shared router, and on which hardware?
- How should backend trust identities be represented without confusing users?
- What test devices and automated protocol fixtures can support regression testing?
