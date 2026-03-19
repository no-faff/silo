#!/usr/bin/env bash
set -euo pipefail

BINARY_NAME="silo"
DESKTOP_FILE="com.nofaff.Silo.desktop"
INSTALL_DIR="${HOME}/.local/bin"
DESKTOP_DIR="${HOME}/.local/share/applications"

echo "Installing Silo..."

# Copy binary
mkdir -p "${INSTALL_DIR}"
cp "${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

# Install .desktop file, substituting the correct binary path
mkdir -p "${DESKTOP_DIR}"
sed "s|Exec=silo|Exec=${INSTALL_DIR}/${BINARY_NAME}|" \
    "data/${DESKTOP_FILE}" > "${DESKTOP_DIR}/${DESKTOP_FILE}"

# Update desktop database
update-desktop-database "${DESKTOP_DIR}" 2>/dev/null || true

echo "Installed to ${INSTALL_DIR}/${BINARY_NAME}"
echo ""
echo "To set Silo as your default browser, run:"
echo "  silo"
echo ""
echo "Or set it manually:"
echo "  xdg-settings set default-web-browser ${DESKTOP_FILE}"
