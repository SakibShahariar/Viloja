#!/usr/bin/env bash
# Collect Linux radio capabilities for Viloja's hotspot, Wi-Fi Direct, and
# Bluetooth feasibility investigations. Output is intended for manual review.
set -euo pipefail

redact_macs() {
    sed -E 's/([[:xdigit:]]{2}:){5}[[:xdigit:]]{2}/[REDACTED-MAC]/g'
}

section() {
    printf '\n===== %s =====\n' "$1"
}

section "Operating system and kernel"
printf 'Kernel: '
uname -srmo
if [[ -r /etc/os-release ]]; then
    # shellcheck disable=SC1091
    . /etc/os-release
    printf 'Distribution: %s %s\n' "${PRETTY_NAME:-unknown}" "${VERSION_ID:-}"
fi

section "PCI devices and active drivers"
if command -v lspci >/dev/null 2>&1; then
    lspci -nnk | redact_macs
else
    printf 'lspci is not installed.\n'
fi

section "USB devices"
if command -v lsusb >/dev/null 2>&1; then
    lsusb | redact_macs
else
    printf 'lsusb is not installed.\n'
fi

section "Wi-Fi supported interface modes"
if command -v iw >/dev/null 2>&1; then
    iw list 2>&1 | awk '
        /Supported interface modes:/ { printing = 1; print; next }
        printing && /^[[:space:]]*\*/ { print; next }
        printing && /^[^[:space:]]/ { printing = 0 }
    '
else
    printf 'iw is not installed. Install the iw package to inspect capabilities.\n'
fi

section "Radio block state"
if command -v rfkill >/dev/null 2>&1; then
    rfkill list
else
    printf 'rfkill is not installed.\n'
fi

section "Bluetooth controllers"
if command -v bluetoothctl >/dev/null 2>&1; then
    bluetoothctl list 2>&1 | redact_macs
else
    printf 'bluetoothctl is not installed.\n'
fi

cat <<'NOTICE'

===== Manual test data still required =====
This report cannot establish Android compatibility, successful pairing,
hotspot operation, Wi-Fi Direct connectivity, or transfer fallback. Record
the Android version, backend version, network topology, and real test result
separately. Review output before sharing; redact hostnames or device details
that you consider private.
NOTICE
