# Viloja

**A native GNOME app for nearby device discovery and wireless file sharing.**

Viloja aims to bring nearby-device file sharing and useful phone-to-desktop integration into one focused GNOME application. Its design goal is a polished GTK4/libadwaita experience with independent backends, so each supported sharing system can evolve without coupling the whole app to one protocol.

> **Status: frontend prototype.** The GTK4/libadwaita application shell and a basic Valent session-D-Bus device-query path are implemented. Live discovery has not yet been verified against an installed Valent service in a GNOME session. Pairing remains managed by Valent; inbound reception and actual file transfers are not implemented yet. The UI must not be mistaken for a working sharing application.

## Current prototype

- GNOME-native application window with Nearby, Send files, Transfers, History, and Settings pages.
- Native multi-file chooser for preparing a selection.
- Explicit empty states rather than fake discovered devices or simulated successful transfers.
- Backend-specific capabilities and status are intended to be represented honestly in the UI.

## Current backend limitations

The current Valent integration is experimental: it queries the session D-Bus for device objects, but discovery has not been verified against an installed Valent build in a live GNOME session. Pairing remains managed by Valent rather than Viloja. Outbound and inbound file transfers are not implemented. Router-free, hotspot, Wi-Fi Direct, and Bluetooth transfer support has not been verified and is not advertised.

The application repeats these limitations on its main pages so users see them before attempting unavailable actions. These statements describe the current Valent prototype only; they are not claims about future backends.

## Goals

- Feel at home on GNOME, using GTK4 and libadwaita.
- Explore support for Android Quick Share and KDE Connect as separate integrations.
- Present discovered devices, incoming requests, outgoing transfers, progress, errors, and history in one consistent UI.
- Keep protocol-specific discovery, authentication, networking, and transfer logic isolated behind backend interfaces.
- Prefer local-network transfers and investigate router-free options where the device and adapter support them.
- Be transparent about backend capabilities and limitations instead of promising every connection method everywhere.

## Run locally

On Fedora, install the native development dependencies and Rust toolchain:

```sh
sudo dnf install gcc pkg-config gtk4-devel libadwaita-devel
```

Then, from the repository root:

```sh
cargo run
```

The first Cargo build will download Rust dependencies. The application currently only provides the UI shell and file-selection preview; it does not send or receive files.

## Planned experience

- **Nearby:** discovered devices and connection status.
- **Send files:** choose files and a destination device.
- **Incoming transfers:** review sender and file details, then accept or reject.
- **Transfers:** progress, cancellation, and clear error reporting.
- **Settings:** backend availability, receiving preferences, and privacy controls.
- **History:** a local record of completed and failed transfers, with controls to clear it.

These remain design goals unless explicitly listed in the current prototype section.

## Architecture direction

Viloja will use a shared GNOME interface with separate backend implementations. A backend should report its capabilities and own its protocol-specific discovery, pairing, authentication, and transfer operations. The UI must not assume every backend supports the same features.

See [Architecture](docs/ARCHITECTURE.md), [Feasibility notes](docs/FEASIBILITY.md), and the [proof-of-concept plan](docs/POC_PLAN.md). The current implementation's security boundaries and unimplemented safeguards are recorded in the [security review](docs/SECURITY_REVIEW.md).

## License

The project license has **not been finalized yet**. See [LICENSE](LICENSE) for the current status. Do not assume the repository grants reuse or redistribution permissions until a license is selected and its full text is added.
