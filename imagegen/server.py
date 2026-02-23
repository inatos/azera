"""
Azera Image Generation Server
─────────────────────────────
Serves Animagine XL 3.1 with step-level progress tracking.
"""

import os, gc, json, random, base64, logging, threading
from io import BytesIO
from typing import Optional, Dict, Any, List

import torch
import uvicorn
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")
logger = logging.getLogger("imagegen")

app = FastAPI(title="Azera ImageGen")

CACHE_DIR = os.environ.get("HF_HOME", "/models")

# ── Runtime state ───────────────────────────────────────────────────────────────
_pipe = None
_progress = {"step": 0, "total_steps": 0, "percentage": 0.0, "active": False}
_progress_lock = threading.Lock()


def _load_pipeline():
    global _pipe
    if _pipe is not None:
        return _pipe

    from diffusers import StableDiffusionXLPipeline

    logger.info("Loading Animagine XL 3.1 …")
    _pipe = StableDiffusionXLPipeline.from_pretrained(
        "cagliostrolab/animagine-xl-3.1",
        torch_dtype=torch.float16,
        use_safetensors=True,
        cache_dir=CACHE_DIR,
    )
    _pipe.to("cuda")
    logger.info("✔ Animagine XL 3.1 ready")
    return _pipe


def _step_callback(pipe, step_index, timestep, callback_kwargs):
    """Called by diffusers after each denoising step."""
    with _progress_lock:
        total = _progress["total_steps"]
        current = step_index + 1
        _progress["step"] = current
        _progress["percentage"] = (current / total * 100.0) if total > 0 else 0.0
    return callback_kwargs


# ── Request schema ──────────────────────────────────────────────────────────────
class GenerateRequest(BaseModel):
    prompt: str = ""
    negative_prompt: str = ""
    width: int = 1024
    height: int = 1024
    steps: int = 28
    cfg_scale: float = 7.0
    seed: int = -1
    override_settings: Optional[Dict[str, Any]] = None
    override_settings_restore_afterwards: bool = True
    init_images: Optional[List[str]] = None
    denoising_strength: float = 0.75


# ── Endpoints ───────────────────────────────────────────────────────────────────
@app.get("/")
def root():
    return {"status": "ok", "model": "animagine-xl-3.1"}


@app.get("/sdapi/v1/sd-models")
def list_models():
    return [{
        "title": "animagine-xl-3.1",
        "model_name": "Animagine XL 3.1",
        "description": "Anime / manga generation (SDXL fine-tune)",
        "hash": "", "sha256": "",
        "filename": "cagliostrolab/animagine-xl-3.1",
    }]


@app.get("/sdapi/v1/progress")
def get_progress():
    with _progress_lock:
        return {
            "step": _progress["step"],
            "total_steps": _progress["total_steps"],
            "percentage": _progress["percentage"],
            "active": _progress["active"],
        }


@app.post("/sdapi/v1/txt2img")
def txt2img(req: GenerateRequest):
    try:
        pipe = _load_pipeline()
    except Exception as exc:
        raise HTTPException(500, detail=str(exc))

    seed = req.seed if req.seed != -1 else random.randint(0, 2**32 - 1)
    gen = torch.Generator("cuda").manual_seed(seed)

    with _progress_lock:
        _progress["step"] = 0
        _progress["total_steps"] = req.steps
        _progress["percentage"] = 0.0
        _progress["active"] = True

    logger.info("Generating: '%s' %dx%d (%d steps)", req.prompt[:60], req.width, req.height, req.steps)

    try:
        result = pipe(
            prompt=req.prompt,
            negative_prompt=req.negative_prompt or None,
            height=req.height,
            width=req.width,
            num_inference_steps=req.steps,
            guidance_scale=req.cfg_scale,
            generator=gen,
            callback_on_step_end=_step_callback,
        )
    except Exception as exc:
        with _progress_lock:
            _progress["active"] = False
        raise HTTPException(500, detail=str(exc))

    image = result.images[0]

    buf = BytesIO()
    image.save(buf, format="PNG")
    b64 = base64.b64encode(buf.getvalue()).decode()

    with _progress_lock:
        _progress["step"] = req.steps
        _progress["percentage"] = 100.0
        _progress["active"] = False

    logger.info("✔ Done (seed=%d)", seed)

    return {
        "images": [b64],
        "parameters": {},
        "info": json.dumps({"seed": seed}),
    }


@app.post("/sdapi/v1/img2img")
def img2img(req: GenerateRequest):
    return txt2img(req)


# ── Main ────────────────────────────────────────────────────────────────────────
if __name__ == "__main__":
    logger.info("🎨 Azera ImageGen starting on :7860")
    uvicorn.run(app, host="0.0.0.0", port=7860, log_level="info")
