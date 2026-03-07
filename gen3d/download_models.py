"""Download Hunyuan3D-2.1 model weights at container startup."""

import os
import logging

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")
logger = logging.getLogger("gen3d-download")

MODEL_ID = "tencent/Hunyuan3D-2.1"
CACHE_DIR = os.environ.get("HF_HOME", "/models")


def download():
    logger.info(f"Downloading {MODEL_ID} weights to {CACHE_DIR} ...")
    try:
        from huggingface_hub import snapshot_download
        snapshot_download(
            MODEL_ID,
            cache_dir=CACHE_DIR,
            local_dir_use_symlinks=True,
        )
        logger.info("✔ Model download complete")
    except Exception as e:
        logger.error(f"Failed to download model: {e}")
        raise


if __name__ == "__main__":
    download()
