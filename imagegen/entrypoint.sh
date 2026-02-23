#!/bin/bash
set -e

echo "🎨 Azera ImageGen — checking models..."
python download_models.py
echo "✔ Models ready"

exec python server.py
