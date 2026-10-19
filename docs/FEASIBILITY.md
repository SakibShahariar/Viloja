# Feasibility and research checklist

This is a research plan, not a compatibility guarantee. All findings should include sources, versions, test setup, and limitations.

## Candidate integrations

### Android Quick Share

Investigate maintained Linux clients and relevant protocol documentation before choosing an implementation strategy. Audit the project's license, dependency licenses, release activity, security model, supported Android versions, and whether reuse as a library or external process is practical.

A successful discovery demo is not sufficient: validate an authenticated end-to-end file transfer, rejection of an incoming request, error handling, and repeat transfers.

### KDE Connect

Investigate whether an existing implementation can be integrated as a separate backend or companion process. Record which KDE Connect features are in scope for the first release. Do not assume every phone feature is required for a useful file-sharing MVP.

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

Useful Linux checks during implementation include `lspci -nnk`, `lsusb`, and `iw list`. Redact MAC addresses and other identifiers before sharing output publicly.

## Licensing and security audit

Before adopting code or a dependency:

- Confirm the repository's current license and license of vendored components.
- Check whether the license permits the intended linking, packaging, and distribution model.
- Review maintenance activity, release history, known security issues, and dependency health.
- Understand pairing, identity verification, encryption, incoming-file handling, and temporary-file cleanup.
- Avoid implementing cryptographic primitives or weakening protocol verification to make a demo work.

## Evidence required before claiming support

For every supported scenario, record the app/backend version, OS and Android versions, network setup, adapter/driver, exact test steps, outcome, and known limitations. Mark untested scenarios as **unknown**, not supported.
