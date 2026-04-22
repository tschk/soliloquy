#!/bin/bash
# Stage Soliloquy artifacts into rootfs
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
WORK_DIR="${PROJECT_ROOT}/build/alpine"
ROOTFS_DIR="${WORK_DIR}/rootfs"

echo "Staging Soliloquy artifacts..."

cd "${PROJECT_ROOT}"

# Copy binaries
echo "Copying binaries..."
cp "target/x86_64-unknown-linux-musl/release/soliloquy_shell" "${ROOTFS_DIR}/usr/local/bin/"
cp "target/x86_64-unknown-linux-musl/release/sold" "${ROOTFS_DIR}/usr/local/bin/"
cp "target/x86_64-unknown-linux-musl/release/rv8" "${ROOTFS_DIR}/usr/local/bin/"

WAX_BIN="${PROJECT_ROOT}/../wax/build/wax"
if [ -x "${WAX_BIN}" ]; then
    cp "${WAX_BIN}" "${ROOTFS_DIR}/usr/local/bin/wax"
fi

# Create sol-session script
cat > "${ROOTFS_DIR}/usr/local/bin/sol-session" << 'EOF'
#!/bin/sh
# Soliloquy session launcher

# Wait for sold to be ready
while ! curl -s http://127.0.0.1:8080/health > /dev/null; do
    sleep 1
done

# Launch cage with Servo
exec cage -- soliloquy_shell --url http://127.0.0.1:8080
EOF

chmod +x "${ROOTFS_DIR}/usr/local/bin/sol-session"

# Stage browser bundle
mkdir -p "${ROOTFS_DIR}/usr/share/soliloquy/bundle"
echo "<html><body><h1>Soliloquy Appliance</h1><p>Loading...</p></body></html>" > "${ROOTFS_DIR}/usr/share/soliloquy/bundle/index.html"

echo "Artifacts staged."
