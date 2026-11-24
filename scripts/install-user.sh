#!/usr/bin/env bash
# Build Viloja in release mode and install it for the current user.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
prefix="${VILOJA_PREFIX:-$HOME/.local}"
destdir="${DESTDIR:-}"
desktop_id="io.github.SakibShahariar.Viloja"

if [[ ! -f "$repo_root/Cargo.toml" ]]; then
    echo "error: cannot find Cargo.toml next to scripts/install-user.sh" >&2
    exit 1
fi

echo "Building Viloja (release)..."
cargo build --release --manifest-path "$repo_root/Cargo.toml"

install -Dm755 "$repo_root/target/release/viloja" "$destdir$prefix/bin/viloja"

desktop_dir="$destdir$prefix/share/applications"
mkdir -p "$desktop_dir"
cat > "$desktop_dir/$desktop_id.desktop" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=Viloja
GenericName=File Sharing
Comment=Experimental nearby device sharing prototype
Exec="$prefix/bin/viloja"
Icon=folder-publicshare-symbolic
Terminal=false
Categories=Network;FileTransfer;
StartupNotify=true
EOF

if [[ -z "$destdir" ]] && command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$prefix/share/applications" >/dev/null 2>&1 || true
fi

echo "Installed $prefix/bin/viloja"
echo "Installed $prefix/share/applications/$desktop_id.desktop"
echo "Launch it from GNOME Activities or run: $prefix/bin/viloja"
