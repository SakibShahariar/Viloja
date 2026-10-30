# Dependency and license audit

Last reviewed: 2026-10-30

This document records the current dependency policy and known adoption gates. It is an engineering audit, not legal advice.

## Viloja's own license

The root \`LICENSE\` file explicitly says that Viloja's license has not been finalized. Do not assume that the repository grants permission to reuse, modify, or redistribute its code. Do not package or distribute Viloja until its owner selects a license and replaces the placeholder with the complete license text. The Cargo package is marked \`publish = false\` because this is a desktop application rather than a crates.io library; this does not grant a license or authorize binary distribution.

## Direct Rust dependencies

The manifest currently declares these direct dependencies:

| Manifest name | Crate name | Version requirement | Version observed in CI on 2026-10-30 | Crate license | Source |
| --- | --- | --- | --- | --- | --- |
| \`adw\` | \`libadwaita\` | \`0.7\` | \`0.7.2\` | MIT | crates.io |
| \`gio\` | \`gio\` | \`0.20\` | \`0.20.12\` | MIT | crates.io |
| \`glib\` | \`glib\` | \`0.20\` | \`0.20.12\` | MIT | crates.io |
| \`gtk\` | \`gtk4\` | \`0.9\` | \`0.9.7\` | MIT | crates.io |

The resolved versions above are observations from the 2026-10-30 GitHub Actions build, not pinned requirements. The repository does not currently track a \`Cargo.lock\`; therefore the complete transitive graph can change as compatible crate releases are published. The CI audit runs against the dependency graph resolved for that run. Commit a reviewed lockfile before preparing a reproducible release.

Package metadata and license references:
- [libadwaita 0.7.2](https://docs.rs/crate/libadwaita/0.7.2)
- [gio 0.20.12](https://docs.rs/crate/gio/0.20.12)
- [glib 0.20.12](https://docs.rs/crate/glib/0.20.12)
- [gtk4 0.9.7](https://docs.rs/crate/gtk4/0.9.7)

## Automated policy

The root \`deny.toml\` is the machine-enforced dependency policy. GitHub Actions runs \`cargo-deny\` after Cargo has resolved the graph. The unpublished Viloja workspace package is excluded from third-party dependency license evaluation via \`[licenses.private] ignore = true\`; its own license remains unresolved and distribution remains blocked as described above.

- **Licenses:** only Apache-2.0, MIT, and Unicode-3.0 (including permitted combinations of those terms) are explicitly accepted in the current dependency graph. Unknown, unrecognized, GPL, AGPL, LGPL, or other unapproved license expressions fail the check until reviewed and deliberately added.
- **Advisories:** the RustSec advisory database is checked. Vulnerable, yanked, or unmaintained dependencies are not silently ignored.
- **Bans:** wildcard dependency requirements are denied; duplicate versions are reported as warnings for review.
- **Sources:** dependencies from unknown registries or Git sources are denied. The crates.io registry is the only approved source.
- **Exceptions:** there are currently no license exceptions or ignored security advisories.

The workflow output is the authoritative per-run record of the resolved transitive packages, their license findings, and advisory findings. A passing audit means the resolved graph met this policy at that run; it is not a permanent guarantee about future versions.

## Backend candidates that are not dependencies

These projects are research candidates only. None is currently listed in \`Cargo.toml\`.

| Candidate | License information found | Decision |
| --- | --- | --- |
| [Valent](https://github.com/andyholmes/valent) | GPL-3.0-or-later, as recorded in the feasibility research | Do not link, copy, or package it until the intended integration boundary and license obligations are reviewed. |
| [rquickshare](https://github.com/Martichou/rquickshare) | GPL-3.0, as recorded in the feasibility research | Do not adopt until the intended distribution and linking model is reviewed. |
| [Packet](https://github.com/nozwock/packet) | GPL-3.0, as recorded in the feasibility research | Reference only; do not reuse code before review. |
| [GnomeQS](https://github.com/weversonl/gnome-quick-share) | Its README declares AGPL-3.0, while the checked-in \`LICENSE\` begins with the GNU GPL version 3 text. | **Blocked:** do not use or copy its code until its maintainers clarify the discrepancy and the repository's licensing is consistent. |

The GnomeQS discrepancy is external to Viloja and cannot be resolved by changing Viloja's files. The upstream maintainers must clarify whether AGPL-3.0 or GPL-3.0 applies. Until then, GnomeQS is not an approved source or dependency.

## Maintenance and security review limits

The automated advisory check detects published RustSec advisories and dependency maintenance/yank notices available to cargo-deny. It does not replace a manual review of upstream release activity, maintainer responsiveness, project governance, protocol security, or the security of code vendored outside Cargo. Reassess those factors before adopting each new backend or non-Cargo component.

## Re-running the audit

The audit runs on every push to \`main\` and every pull request through the Rust CI workflow. For a local audit, first resolve the Cargo dependency graph, then run:

\`\`\`sh
cargo check
cargo deny check --all-features
\`\`\`

When changing dependencies, inspect the complete cargo-deny output. Do not add a license exception, advisory ignore, registry, or Git source merely to make CI green; record the reason and review the affected dependency first.

## Remaining gates

- Select and add Viloja's own license before distribution.
- Commit a reviewed \`Cargo.lock\` before a reproducible release.
- Obtain upstream clarification of GnomeQS's README/LICENSE mismatch before any GnomeQS use.
- Reassess the license and security implications before adopting Valent, rquickshare, Packet, or another backend.
