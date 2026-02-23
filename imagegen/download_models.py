"""Pre-download Animagine XL 3.1 into the HF cache so first generation is fast."""

import os

os.environ.setdefault("HF_HOME", "/models")

from huggingface_hub import snapshot_download

cache = os.environ.get("HF_HOME", "/models")

print("  📦 cagliostrolab/animagine-xl-3.1")
try:
    snapshot_download(
        "cagliostrolab/animagine-xl-3.1",
        cache_dir=cache,
        ignore_patterns=["*.bin", "*.onnx", "*.pt", "*.msgpack"],
    )
    print("  ✓ ready")
except Exception as e:
    print(f"  ✗ {e}")
