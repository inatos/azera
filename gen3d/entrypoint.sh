#!/bin/bash
set -e

# Persist hy3dgen's internal model cache on the mounted volume.
# Without this, every container recreate re-downloads ~6.5 GB of DiT weights
# to /root/.cache/hy3dgen (ephemeral container storage).
mkdir -p /models/hy3dgen_cache
mkdir -p /root/.cache
ln -sfn /models/hy3dgen_cache /root/.cache/hy3dgen

echo "🧊 Azera Gen3D — checking models..."
python3 download_models.py
echo "✔ Models ready"

exec python3 server.py
