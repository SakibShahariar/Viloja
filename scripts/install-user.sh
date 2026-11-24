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

icon_source=""
for candidate in "$repo_root"/data/viloja.*; do
    if [[ -f "$candidate" ]]; then
        icon_source="$candidate"
        break
    fi
done

if [[ -n "$icon_source" ]]; then
    icon_ext="${icon_source##*.}"
    case "$icon_ext" in
        svg) icon_dir="$destdir$prefix/share/icons/hicolor/scalable/apps" ;;
        *) icon_dir="$destdir$prefix/share/icons/hicolor/512x512/apps" ;;
    esac
    install -Dm644 "$icon_source" "$icon_dir/$desktop_id.$icon_ext"
    icon_line="Icon=$desktop_id"
else
    icon_line="Icon=folder-publicshare-symbolic"
fi

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
$icon_line
Terminal=false
Categories=Network;FileTransfer;
StartupNotify=true
EOF

if [[ -z "$destdir" ]]; then
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$prefix/share/applications" >/dev/null 2>&1 || true
    fi
    if [[ -n "$icon_source" ]] && command -v gtk4-update-icon-cache >/dev/null 2>&1; then
        gtk4-update-icon-cache -q -f "$prefix/share/icons/hicolor" >/dev/null 2>&1 || true
    fi
fi

echo "Installed $prefix/bin/viloja"
echo "Installed $prefix/share/applications/$desktop_id.desktop"
if [[ -n "$icon_source" ]]; then
    echo "Installed $prefix/share/icons/hicolor/*/apps/$desktop_id.$icon_ext"
fi
echo "Launch it from GNOME Activities or run: $prefix/bin/viloja"
