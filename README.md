# Viloja

**A native GNOME app for nearby device discovery and wireless file sharing.**

Viloja aims to bring nearby-device file sharing and useful phone-to-desktop integration into one focused GNOME application. Its design goal is a polished GTK4/libadwaita experience with independent backends, so each supported sharing system can evolve without coupling the whole app to one protocol.

> **Status: early planning.** Viloja is not yet a working file-transfer application. Protocol support, device discovery, transfer flows, and hardware-dependent connectivity options must be validated before implementation claims are made.

## Goals

- Feel at home on GNOME, using GTK4 and libadwaita.
- Explore support for Android Quick Share and KDE Connect as separate integrations.
- Present discovered devices, incoming requests, outgoing transfers, progress, errors, and history in one consistent UI.
- Keep protocol-specific discovery, authentication, networking, and transfer logic isolated behind backend interfaces.
- Prefer local-network transfers and investigate router-free options where the device and adapter support them.
- Be transparent about backend capabilities and limitations instead of promising every connection method everywhere.

## Planned experience

- **Overview:** connection status, nearby devices, and recent transfers.
- **Devices:** device details and available actions.
- **Send:** choose files and a destination device.
- **Incoming transfers:** review sender, file details, and accept or reject.
- **Transfers:** progress, cancellation, and clear error reporting.
- **Settings:** backend availability, receiving preferences, and privacy controls.
- **History:** a local record of completed and failed transfers, with controls to clear it.

These are design goals, not implemented features.

## Architecture direction

Viloja will use a shared GNOME interface with separate backend implementations. A backend should report its capabilities and own its protocol-specific discovery, pairing, authentication, and transfer operations. The UI must not assume every backend supports the same features.

See [Architecture](docs/ARCHITECTURE.md) and [Feasibility notes](docs/FEASIBILITY.md).

## Development status

No functional implementation has been committed yet. The immediate work is to validate protocol and licensing options, define backend boundaries, and build a small proof of concept before committing to a full implementation.

## Contributing

Planning, research, documentation, and proof-of-concept contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

## License

The project license has **not been finalized yet**. See [LICENSE](LICENSE) for the current status. Do not assume the repository grants reuse or redistribution permissions until a license is selected and its full text is added.
