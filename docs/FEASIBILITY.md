# Feasibility and research checklist

This is a research plan, not a compatibility guarantee. Findings below are from a source/documentation audit on 2026-10-19. They do not replace build tests or real-device transfer tests.

## Initial repository audit (2026-10-19)

### Quick Share candidates

| Project | What the source currently indicates | Assessment for Viloja |
| --- | --- | --- |
| [GnomeQS](https://github.com/weversonl/gnome-quick-share) | Rust, GTK4/libadwaita, with a separate Rust core for discovery and transfers. Its core manifest enables experimental Bluetooth support by default. The README declares AGPL-3.0, while the checked-in LICENSE file begins with the GNU GPL v3 text. | Closest stack match and worth investigating first, but **do not copy/link it until the license discrepancy is resolved** and its public core API/build requirements are checked. |
| [Packet](https://github.com/nozwock/packet) | Rust + GTK4/libadwaita; partial Quick Share implementation. README says Bluetooth must be enabled and both devices need the same Wi-Fi network with mDNS. It uses the rquickshare library dependency. Its D-Bus API is currently limited to device name and visibility, not a documented general send/receive API. GPL-3.0 license. | Strong reference for GTK UI and protocol integration, but its current external API is not enough to use the full app as a transfer backend. Investigate the library directly and audit its API/license before adopting it. |
| [Scotty](https://github.com/scottydev40/Scotty) | Qt/QML app on Google's Nearby code, Apache-2.0. Its README describes BLE discovery/pairing, shared-Wi-Fi transfers, hotspot-based off-network transfers, and Bluetooth fallback. The README also warns that discovery depends on a specific kernel/Bluetooth setup; the project is early-stage. | Most promising feature reference for router-free/off-network transport, but not a GTK app and the underlying C++/Bazel integration is a larger engineering task. Must test hardware compatibility before depending on it. |
| [rquickshare](https://github.com/Martichou/rquickshare) | Rust Quick Share implementation, GPL-3.0. Its README documents Wi-Fi-LAN-only operation and a same-network requirement. | Relevant protocol/library candidate, but not a solution for router-free transfers by itself. Audit API stability and dependency license compatibility. |

**Important:** Quick Share compatibility and connection paths are separate questions. A client that works on the same Wi-Fi network does not automatically support Wi-Fi Direct, hotspot mode, Bluetooth transfer fallback, or sending while the laptop is off-network. Scotty's README reports those additional paths, but Viloja must not claim them until they are tested on supported hardware.

### KDE Connect candidates

| Project | What the source currently indicates | Assessment for Viloja |
| --- | --- | --- |
| [Valent](https://github.com/andyholmes/valent) | An implementation of the KDE Connect protocol written in C on GNOME platform libraries. It is GPL-3.0-or-later and currently describes itself as alpha. Its source exposes devices through D-Bus/Object Manager and device actions, including sharing. | **Best GNOME-native candidate to investigate first.** Determine whether its D-Bus API or libraries can support Viloja's desired device/transfer flows without duplicating the protocol. Alpha status and API stability are risks. |
| [KDE Connect desktop](https://github.com/KDE/kdeconnect-kde) | Mature Qt-based implementation with a UI-agnostic core and D-Bus interfaces. Its documentation describes file, URL, clipboard, notification and other device features. GPL v2/v3 licensing is indicated in the project README. | Strong reference implementation, but integrating its core directly may pull in Qt/KDE dependencies. Assess D-Bus integration versus using Valent before deciding. |
| [GSConnect](https://github.com/GSConnect/gnome-shell-extension-gsconnect) | Complete KDE Connect implementation focused on GNOME Shell, with file sharing and phone integration. Its README says development is community-driven and relies on contributions rather than dedicated developers. | Useful reference for GNOME integration and protocol behavior; evaluate maintenance and whether an extension-centric architecture suits a standalone GTK app. |

### Initial direction (not yet a final dependency decision)

- Keep Viloja's main interface GTK4/libadwaita and keep the two protocol backends independent.
- For KDE Connect, prototype discovery and a file transfer through Valent before considering implementing the protocol ourselves.
- For Quick Share, compare the effort of using GnomeQS's Rust core with using the rquickshare dependency. Resolve GnomeQS's license-file/README mismatch first. Keep Scotty as the reference for router-free transport, not as an assumed drop-in GTK backend.
- Do not simply wrap Packet's current D-Bus API and call that a complete backend: the documented API does not expose general file-transfer operations.
- Before choosing an approach, build the smallest proof of concept that discovers a real Android phone, sends a file, receives a file, handles rejection/failure, and shuts down cleanly.

This is an engineering assessment, not legal advice. Review the exact licenses of source files, dependencies, and packaged assets before reusing code.

## Connectivity scenarios to test

| Scenario | What must be verified |
| --- | --- |
| Both devices on the same Wi-Fi network | Discovery, authentication, transfer, firewall behavior |
| Devices on different networks | Expected to fail unless a supported route exists; document actual behavior |
| No internet, but both devices share a local network | Whether discovery and transfers work entirely locally |
| Laptop provides a hotspot | Whether Android can join it and the chosen backend can discover and transfer |
| Wi-Fi Direct | Adapter, driver, kernel, and peer compatibility; do not assume universal Linux support |
| Bluetooth-assisted discovery or fallback | Whether the protocol and both devices support the specific operation |

A local hotspot can provide a router-free local network in some setups, but it is not proof that Quick Share will work over that path. Verify the actual protocol and Android behavior before presenting it as a feature.

## Hardware validation

The initial development machine is a Dell Latitude 7480 running Fedora 44 with GNOME. Its exact Wi-Fi adapter and driver capabilities still need to be identified. Record the adapter model, kernel/driver, supported interface modes, and relevant logs in test notes; do not infer Wi-Fi Direct support from the laptop model alone.

Useful Linux checks during implementation include lspci -nnk, lsusb, and iw list. Redact MAC addresses and other identifiers before sharing output publicly.

## Licensing and security audit

Before adopting code or a dependency:

- Confirm the repository's current license and license of vendored components.
- Check whether the license permits the intended linking, packaging, and distribution model.
- Review maintenance activity, release history, known security issues, and dependency health.
- Understand pairing, identity verification, encryption, incoming-file handling, and temporary-file cleanup.
- Avoid implementing cryptographic primitives or weakening protocol verification to make a demo work.

## Evidence required before claiming support

For every supported scenario, record the app/backend version, OS and Android versions, network setup, adapter/driver, exact test steps, outcome, and known limitations. Mark untested scenarios as **unknown**, not supported.
