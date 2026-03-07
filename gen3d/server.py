"""
Azera 3D Generation Server
───────────────────────────
Hunyuan3D-2.1 shape generation with sequential CPU↔GPU offloading.

LOW_VRAM strategy (inspired by ComfyUI-Hunyuan3D-2.1):
  Phase 1 — DiT + conditioner on GPU → run diffusion → raw latents
  Phase 2 — offload DiT → VAE on GPU → decode latents → trimesh
  Peak VRAM = max(DiT, VAE) ≈ 7 GB instead of sum ≈ 10 GB.
"""

import os
import gc
import sys
import json
import base64
import logging
import threading
import uuid
import time
import asyncio
import warnings
import concurrent.futures
from io import BytesIO
from datetime import datetime
from typing import Optional

# Suppress deprecation warnings from diffusers/torchvision internals
warnings.filterwarnings("ignore", message=".*torch.backends.cuda.sdp_kernel.*", category=FutureWarning)

# Add Hunyuan3D module paths (must come before torch imports)
sys.path.insert(0, "./hy3dshape")
sys.path.insert(0, "./hy3dpaint")

import torch
import torch._dynamo
# Increase dynamo cache size — the attention module triggers recompilation
# due to rank mismatches (expected 2, actual 3).  Default limit (8) causes
# excessive "cache_size_limit hit" warnings.
torch._dynamo.config.cache_size_limit = 64
import uvicorn
from PIL import Image
from fastapi import FastAPI, HTTPException
from fastapi.responses import JSONResponse
from pydantic import BaseModel, Field

# ── Memory-efficient loading ────────────────────────────────────────────────────
# The Hunyuan3D pipeline calls:
#     ckpt = torch.load(checkpoint.ckpt, map_location='cpu')  # ~7 GB into RAM
#     model.load_state_dict(ckpt['model'])      # +6.5 GB (DiT 3.3B fp16)
#     vae.load_state_dict(ckpt['vae'])           # +1.5 GB
#     conditioner.load_state_dict(ckpt['conditioner'])  # +1 GB
#     # Peak: ~16 GB — exceeds WSL2 default (50% of 32 GB host RAM = 16 GB)
#
# Fix 1:  mmap=True  — lazy page loading; OS pages in data on demand.
# Fix 2:  _StagedDict — accessing a new top-level key auto-frees the previous
#         key's data (e.g. 'model' ~6.5 GB freed before 'vae' is touched).
# Fix 3:  assign=True — load_state_dict *replaces* model parameters with the
#         mmap'd tensors directly instead of copying, eliminating the 2x RAM
#         peak (model params + checkpoint data coexisting).
#         Combined peak RAM ≈ single largest model ≈ 6.5 GB.
_original_torch_load = torch.load
_original_load_state_dict = torch.nn.Module.load_state_dict


class _StagedDict(dict):
    """Dict wrapper that frees the previous value when a *different* key is read."""

    def __init__(self, src: dict):
        super().__init__(src)
        self._prev: list[str] = []

    def __getitem__(self, key):
        freed = False
        for old in self._prev:
            if old != key and old in self:
                dict.__delitem__(self, old)
                freed = True
        if freed:
            gc.collect()
        self._prev = [key]
        return dict.__getitem__(self, key)


def _mmap_torch_load(f, *args, **kwargs):  # noqa: ANN
    try:
        kwargs.setdefault("mmap", True)
        result = _original_torch_load(f, *args, **kwargs)
    except TypeError:
        # mmap kwarg not supported on this PyTorch build
        kwargs.pop("mmap", None)
        result = _original_torch_load(f, *args, **kwargs)
    # Wrap combined Hunyuan3D checkpoints for staged memory management
    if isinstance(result, dict) and {"model", "vae"} <= set(result.keys()):
        logger.info("Staged loading: checkpoint will auto-free sub-dicts between model loads")
        return _StagedDict(result)
    return result


def _assign_load_state_dict(self, state_dict, strict=True, **kwargs):  # noqa: ANN
    """Monkey-patched load_state_dict that uses assign=True.

    With assign=True (PyTorch 2.1+), parameters are replaced by the loaded
    tensors *in place* rather than copied.  Combined with mmap=True on
    torch.load, this means model parameters point directly to the memory-
    mapped file — no 2x RAM peak from simultaneous checkpoint + params.
    """
    kwargs.setdefault("assign", True)
    return _original_load_state_dict(self, state_dict, strict=strict, **kwargs)


torch.load = _mmap_torch_load  # type: ignore[assignment]
torch.nn.Module.load_state_dict = _assign_load_state_dict  # type: ignore[assignment]

# Apply torchvision compatibility fix from Hunyuan3D repo
try:
    from torchvision_fix import apply_fix
    apply_fix()
except (ImportError, Exception):
    pass

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")
logger = logging.getLogger("gen3d")

app = FastAPI(title="Azera Gen3D")

MODEL_ID = os.environ.get("MODEL_ID", "tencent/Hunyuan3D-2.1")
CACHE_DIR = os.environ.get("HF_HOME", "/models")
OUTPUT_DIR = os.environ.get("OUTPUT_DIR", "/app/outputs")
DISABLE_TEX = os.environ.get("DISABLE_TEX", "true").lower() in ("true", "1", "yes")
LOW_VRAM_MODE = os.environ.get("LOW_VRAM_MODE", "true").lower() in ("true", "1", "yes")
ENABLE_COMPILE = os.environ.get("ENABLE_COMPILE", "true").lower() in ("true", "1", "yes")
IMAGE_GEN_URL = os.environ.get("IMAGE_GEN_URL", "http://imagegen:7860")

# ── Inductor kernel cache (survives container restarts) ─────────────────────────
# torch.compile generates optimised Triton kernels on first inference.
# Caching avoids recompilation (~2-5 min) when the container restarts.
os.environ.setdefault("TORCHINDUCTOR_CACHE_DIR", "/models/torch_cache")

os.makedirs(OUTPUT_DIR, exist_ok=True)

# ── Runtime state ───────────────────────────────────────────────────────────────
_shape_pipe = None
_tex_pipe = None
_rembg = None
_ready = False
_progress = {"step": 0, "total_steps": 0, "percentage": 0.0, "status": "idle"}
_progress_lock = threading.Lock()

# Single-thread GPU executor — serialises all GPU work so the async event-loop
# stays free for health / progress polling.
_gpu_executor = concurrent.futures.ThreadPoolExecutor(max_workers=1, thread_name_prefix="gpu")


def _flush_vram():
    """Force-free all cached CUDA memory."""
    gc.collect()
    if torch.cuda.is_available():
        torch.cuda.empty_cache()


def _unload_shape_pipeline():
    """Fully unload shape pipeline from RAM.

    Models live on the Docker volume (/models).  With mmap=True + assign=True,
    reloading pages them in from disk (~30s) instead of holding ~9 GB in RAM
    permanently.  The OS file-cache keeps hot pages warm between runs.
    """
    global _shape_pipe
    if _shape_pipe is None:
        return
    logger.info("Unloading shape pipeline from RAM (volume-backed, will mmap on next request)")
    _shape_pipe = None
    gc.collect()
    _flush_vram()


def _set_progress(**kw):
    with _progress_lock:
        _progress.update(kw)


# ── Model loading ──────────────────────────────────────────────────────────────

def _load_shape_pipeline():
    """Load (or reload) the shape pipeline from the volume.

    Called on-demand when _shape_pipe is None.  With mmap=True + assign=True
    the weights are paged in from the Docker volume (~30s) instead of being
    held in RAM permanently.
    """
    global _shape_pipe

    logger.info("Loading shape pipeline: %s  (LOW_VRAM=%s)", MODEL_ID, LOW_VRAM_MODE)

    from hy3dshape import Hunyuan3DDiTFlowMatchingPipeline

    _shape_pipe = Hunyuan3DDiTFlowMatchingPipeline.from_pretrained(MODEL_ID)

    if LOW_VRAM_MODE:
        logger.info("Low-VRAM: parking shape pipeline on CPU")
        _shape_pipe.model.to("cpu")
        _shape_pipe.conditioner.to("cpu")
        _shape_pipe.vae.to("cpu")
        _flush_vram()

    if ENABLE_COMPILE:
        _compile_mode = "max-autotune-no-cudagraphs" if not LOW_VRAM_MODE else "default"
        logger.info("torch.compile: DiT + conditioner (mode=%s)", _compile_mode)
        try:
            _shape_pipe.model = torch.compile(
                _shape_pipe.model,
                backend="inductor",
                mode=_compile_mode,
                fullgraph=False,
                dynamic=False,
            )
            _shape_pipe.conditioner = torch.compile(
                _shape_pipe.conditioner,
                backend="inductor",
                mode=_compile_mode,
                fullgraph=False,
            )
            logger.info("torch.compile: models wrapped (compilation deferred to first inference)")
        except Exception as e:
            logger.warning("torch.compile failed, continuing without: %s", e)

    logger.info("Shape pipeline ready")


def _load_pipelines():
    """Startup init: load shape pipeline, optional texture pipeline, and rembg."""
    global _tex_pipe, _rembg, _ready

    _load_shape_pipeline()

    # ── Texture pipeline ─────────────────────────────────────────────────
    # In LOW_VRAM mode the paint pipeline is loaded lazily — AFTER shape
    # generation finishes and the DiT/conditioner/VAE are back on CPU.
    # This keeps peak VRAM = max(shape_peak, tex_peak) instead of sum.
    if not DISABLE_TEX:
        if LOW_VRAM_MODE:
            logger.info("Texture pipeline enabled (lazy — will load on first generation)")
            _tex_pipe = None  # Will be loaded on demand in _ensure_tex_pipeline()
        else:
            _tex_pipe = _load_tex_pipeline_now()
    else:
        logger.info("Texture pipeline disabled (DISABLE_TEX=true)")
        _tex_pipe = None

    # ── Background remover (lightweight) ────────────────────────────────
    from hy3dshape.rembg import BackgroundRemover
    _rembg = BackgroundRemover()
    logger.info("Background remover ready — server is accepting requests")
    if not DISABLE_TEX and LOW_VRAM_MODE:
        logger.info("NOTE: Texture pipeline will be loaded on first generation (adds ~60-120s one-time)")
    if ENABLE_COMPILE:
        logger.info("NOTE: First generation request will be slower (~2-5 min) due to one-time graph compilation")
    _ready = True


@app.on_event("startup")
async def _on_startup():
    """Load models in a daemon thread so health / progress endpoints stay live."""
    def _bg():
        try:
            _load_pipelines()
        except Exception:
            logger.error("Model loading failed", exc_info=True)

    threading.Thread(target=_bg, daemon=True).start()


# ── Lazy texture pipeline ───────────────────────────────────────────────────────

def _load_tex_pipeline_now():
    """Load the Hunyuan3DPaint pipeline immediately."""
    try:
        from textureGenPipeline import Hunyuan3DPaintPipeline, Hunyuan3DPaintConfig

        tex_res = int(os.environ.get("TEXTURE_RESOLUTION", "512"))
        tex_views = int(os.environ.get("TEXTURE_VIEWS", "4"))

        conf = Hunyuan3DPaintConfig(max_num_view=tex_views, resolution=tex_res)
        conf.realesrgan_ckpt_path = "hy3dpaint/ckpt/RealESRGAN_x4plus.pth"
        conf.multiview_cfg_path = "hy3dpaint/cfgs/hunyuan-paint-pbr.yaml"
        conf.custom_pipeline = "hy3dpaint/hunyuanpaintpbr"
        # Scale render/texture sizes proportionally to resolution
        # Default: resolution=1024 → render_size=2048, texture_size=4096
        # Our low-VRAM defaults: resolution=512 → render_size=1024, texture_size=2048
        conf.render_size = tex_res * 2
        conf.texture_size = tex_res * 4
        pipe = Hunyuan3DPaintPipeline(conf)
        logger.info("Texture pipeline ready (resolution=%d, render=%d, tex=%d, views=%d)",
                     tex_res, conf.render_size, conf.texture_size, tex_views)
        return pipe
    except Exception as e:
        logger.warning("Texture pipeline unavailable: %s", e)
        return None


def _ensure_tex_pipeline():
    """Lazy-load texture pipeline on first use (LOW_VRAM mode).

    Called *after* shape generation finishes and shape models are parked on CPU,
    so the full GPU VRAM budget is available for the paint models.
    """
    global _tex_pipe
    if _tex_pipe is not None:
        return _tex_pipe
    if DISABLE_TEX:
        return None

    logger.info("Lazy-loading texture pipeline (first use)…")
    _flush_vram()
    _tex_pipe = _load_tex_pipeline_now()
    return _tex_pipe


# ── Request schema ──────────────────────────────────────────────────────────────
class Generate3DRequest(BaseModel):
    prompt: Optional[str] = None
    image_base64: Optional[str] = None
    image_url: Optional[str] = None
    steps: int = Field(default=50, ge=10, le=200)
    guidance_scale: float = Field(default=7.5, ge=1.0, le=20.0)
    octree_resolution: int = Field(default=256)
    num_views: int = Field(default=6, ge=4, le=12)
    seed: int = Field(default=-1)
    remove_background: bool = True
    foreground_ratio: float = Field(default=0.9, ge=0.5, le=1.0)
    texture_size: int = Field(default=1024, ge=512, le=2048)
    output_format: str = Field(default="glb")
    custom_filename: Optional[str] = None
    enable_texture: bool = Field(default=True)


# ── Helper functions ────────────────────────────────────────────────────────────

def decode_image(image_base64: Optional[str] = None, image_url: Optional[str] = None) -> Optional[Image.Image]:
    """Decode image from base64 or URL."""
    if image_base64:
        data = base64.b64decode(image_base64)
        return Image.open(BytesIO(data)).convert("RGBA")
    elif image_url:
        import requests
        resp = requests.get(image_url, timeout=30)
        resp.raise_for_status()
        return Image.open(BytesIO(resp.content)).convert("RGBA")
    return None


def _cleanup_files(*paths):
    """Remove intermediate files, ignoring errors."""
    for p in paths:
        if p and os.path.exists(p):
            try:
                os.remove(p)
            except OSError:
                pass


# ── Text-to-image bridge ────────────────────────────────────────────────────────

def _text_to_image(prompt: str) -> Image.Image:
    """Generate a reference image from text via the imagegen service.

    Hunyuan3D-2.1 is image-conditioned only — no native text support.
    When the user provides only a prompt, we generate an image first.
    """
    import requests as http_requests
    logger.info("Text-only prompt — generating reference image via imagegen")
    _set_progress(status="generating_reference_image", percentage=5.0)

    resp = http_requests.post(
        f"{IMAGE_GEN_URL}/sdapi/v1/txt2img",
        json={
            "prompt": prompt,
            "negative_prompt": "text, watermark, blurry, low quality, multiple objects",
            "width": 512,
            "height": 512,
            "steps": 20,
            "cfg_scale": 7.0,
        },
        timeout=300,
    )
    resp.raise_for_status()
    data = resp.json()
    img_b64 = data["images"][0]
    img = Image.open(BytesIO(base64.b64decode(img_b64))).convert("RGBA")
    logger.info("Reference image generated (%dx%d)", img.width, img.height)
    return img


# ── GPU worker — runs inside _gpu_executor ──────────────────────────────────────

def _run_shape_generation(
    input_image, prompt, steps, guidance_scale, octree_resolution, seed,
):
    """
    Run shape generation, optionally with sequential DiT / VAE offloading.

    LOW_VRAM_MODE  (default — fits ≤12 GB VRAM):
      Phase 1  DiT + conditioner → GPU, run diffusion (output_type='latent')
      Phase 2  DiT + conditioner → CPU, VAE → GPU, decode latents → trimesh

    Standard mode  (all components stay on GPU):
      Single-pass  pipeline(image=...) → trimesh
    """
    # Reload models if they were unloaded (volume-backed, mmap'd in)
    if _shape_pipe is None:
        logger.info("Loading shape pipeline from volume...")
        _set_progress(status="loading", percentage=2.0)
        _load_shape_pipeline()

    pipe = _shape_pipe
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    generator = torch.Generator(device="cpu").manual_seed(seed)

    common_kwargs = dict(
        num_inference_steps=steps,
        guidance_scale=guidance_scale,
        octree_resolution=octree_resolution,
        generator=generator,
    )

    if LOW_VRAM_MODE:
        return _generate_low_vram(pipe, device, input_image, prompt, octree_resolution, common_kwargs)
    else:
        return _generate_standard(pipe, input_image, prompt, common_kwargs)


def _generate_low_vram(pipe, device, input_image, prompt, octree_resolution, kwargs):
    """Two-phase generation with threaded offload overlap.

    Pipeline parallelism: we offload DiT→CPU in a background thread while
    simultaneously loading VAE→GPU on the main thread, then join.

    Timeline (before):  DiT→CPU [29s] ──→ VAE→GPU [3s] ──→ decode [4s]
    Timeline (after):   DiT→CPU ──┐
                        VAE→GPU ──┘ overlap ──→ decode [4s]
    """

    # ── Phase 1: DiT diffusion → raw latents ────────────────────────────
    logger.info("Phase 1 — DiT + conditioner → GPU")
    _set_progress(status="diffusion", percentage=10.0)

    pipe.conditioner.to(device)
    pipe.model.to(device)
    # VAE stays on CPU: only its .latent_shape property is read by prepare_latents

    call_kwargs = dict(kwargs, output_type="latent")
    if input_image is not None:
        call_kwargs["image"] = input_image
    elif prompt:
        call_kwargs["prompt"] = prompt
    else:
        raise ValueError("No valid input (need image or prompt)")

    raw_latents = pipe(**call_kwargs)
    # _export(output_type='latent') returns latents tensor directly
    if isinstance(raw_latents, (list, tuple)):
        raw_latents = raw_latents[0]

    # ── Overlapped transition: DiT→CPU ‖ VAE→GPU ────────────────────────
    # Offload DiT/conditioner to CPU in a background thread while the main
    # thread loads VAE to GPU.  Both use separate CPU↔GPU DMA channels so
    # they genuinely overlap.  We join before inference starts.
    logger.info("Phase 1 done — overlapped transition: DiT→CPU ‖ VAE→GPU")
    _set_progress(status="decoding_mesh", percentage=50.0)
    t_swap = time.time()

    # Copy latents to CPU before the VRAM swap (tiny tensor, instant)
    latent_cpu = raw_latents.cpu()

    def _offload_dit_conditioner():
        pipe.model.to("cpu")
        pipe.conditioner.to("cpu")

    offload_thread = threading.Thread(target=_offload_dit_conditioner)
    offload_thread.start()

    # Meanwhile, load VAE to GPU on the main thread
    pipe.vae.to(device)

    offload_thread.join()
    logger.info("Phase transition took %.1fs (was ~29s sequential)", time.time() - t_swap)

    # Now safe to free CUDA caches
    gc.collect()
    if torch.cuda.is_available():
        torch.cuda.empty_cache()

    # ── Phase 2: VAE decode → trimesh ───────────────────────────────────
    logger.info("Phase 2 — VAE decoding mesh")

    # Enable FlashVDM acceleration if available (from ComfyUI optimisation)
    try:
        pipe.vae.enable_flashvdm_decoder(enabled=True, mc_algo="mc")
        logger.info("FlashVDM decoder enabled")
    except (AttributeError, Exception):
        pass

    with torch.no_grad():
        latent_dev = latent_cpu.to(device)
        scaled = (1.0 / pipe.vae.scale_factor) * latent_dev
        decoded = pipe.vae(scaled)
        mesh_outputs = pipe.vae.latents2mesh(
            decoded,
            bounds=(-1.01, -1.01, -1.01, 1.01, 1.01, 1.01),
            octree_resolution=octree_resolution,
            num_chunks=1,
            mc_algo="mc",
            mc_level=0.0,
        )

    logger.info("Phase 2 done — offloading VAE to CPU")
    pipe.vae.to("cpu")
    del latent_dev, latent_cpu, scaled, decoded, raw_latents
    _flush_vram()

    # Convert to trimesh
    mesh = _to_trimesh(mesh_outputs)
    return mesh


def _generate_standard(pipe, input_image, prompt, kwargs):
    """Single-pass generation — all components stay on GPU."""
    logger.info("Standard mode — full pipeline on GPU")
    _set_progress(status="generating_shape", percentage=10.0)

    if input_image is not None:
        kwargs["image"] = input_image
    elif prompt:
        kwargs["prompt"] = prompt
    else:
        raise ValueError("No valid input (need image or prompt)")

    result = pipe(**kwargs)
    mesh = result[0] if isinstance(result, (list, tuple)) else result
    return mesh


def _to_trimesh(mesh_outputs):
    """Convert VAE mesh outputs to trimesh object(s)."""
    try:
        from hy3dshape.pipelines import export_to_trimesh
    except ImportError:
        try:
            from hy3dshape.utils import export_to_trimesh
        except ImportError:
            try:
                from utils import export_to_trimesh
            except ImportError:
                logger.warning("export_to_trimesh not found — returning raw mesh data")
                return mesh_outputs

    meshes = export_to_trimesh(mesh_outputs)
    return meshes[0] if isinstance(meshes, (list, tuple)) and len(meshes) > 0 else meshes


def _run_texture_generation(initial_path, input_image, base_name):
    """Run texture generation inside _gpu_executor thread.

    Returns (output_obj_path, cleanup_files) on success, None on failure.
    Ensures VRAM is flushed before and after texture pipeline work.
    """
    _flush_vram()

    tex_pipe = _ensure_tex_pipeline()
    if tex_pipe is None:
        logger.warning("Texture pipeline unavailable — skipping")
        return None

    tex_start = time.time()
    output_obj_path = os.path.join(OUTPUT_DIR, f"{base_name}_textured.obj")

    logger.info("tex_pipe() starting — mesh=%s", initial_path)
    textured_obj = tex_pipe(
        mesh_path=initial_path,
        image_path=input_image,
        output_mesh_path=output_obj_path,
        save_glb=False,
    )
    logger.info("tex_pipe() done — Texture generation took %.1fs", time.time() - tex_start)

    cleanup = [output_obj_path, textured_obj]
    for ext in [".jpg", "_metallic.jpg", "_roughness.jpg", ".mtl"]:
        cleanup.append(textured_obj.replace(".obj", ext))

    # Release texture pipeline from CPU RAM so shape gen has headroom
    # on next run.  It'll be lazy-reloaded (~60s) on next texture request.
    global _tex_pipe
    _tex_pipe = None
    _flush_vram()
    # Return the textured_obj path (for PBR texture file derivation) + cleanup list
    return textured_obj, cleanup


def _use_untextured(mesh, initial_path, output_path, output_format, cleanup_files):
    """Fall back to untextured mesh output."""
    if output_format == "glb" and os.path.exists(initial_path):
        os.rename(initial_path, output_path)
        if initial_path in cleanup_files:
            cleanup_files.remove(initial_path)
    else:
        mesh.export(output_path)


# ── Endpoints ───────────────────────────────────────────────────────────────────

@app.get("/")
async def health():
    return {
        "status": "ready" if _ready else "loading",
        "model": MODEL_ID,
        "gpu": torch.cuda.is_available(),
        "low_vram": LOW_VRAM_MODE,
        "compiled": ENABLE_COMPILE,
        "texture": not DISABLE_TEX and _tex_pipe is not None,
    }


@app.get("/api/v1/progress")
async def get_progress():
    with _progress_lock:
        return dict(_progress)


@app.post("/api/v1/generate")
async def generate_3d(req: Generate3DRequest):
    if not _ready:
        raise HTTPException(status_code=503, detail="Models are still loading, please wait")

    if not req.prompt and not req.image_base64 and not req.image_url:
        raise HTTPException(status_code=400, detail="Either 'prompt' or 'image_base64'/'image_url' is required")

    try:
        _set_progress(step=0, total_steps=req.steps, percentage=0.0, status="preparing")

        # Seed
        seed = req.seed if req.seed >= 0 else int(torch.randint(0, 2**32 - 1, (1,)).item())

        # Decode input image
        input_image = decode_image(req.image_base64, req.image_url)

        # If text-only, generate a reference image first
        if input_image is None and req.prompt:
            try:
                input_image = _text_to_image(req.prompt)
            except Exception as e:
                logger.warning("Failed to generate reference image: %s", e)
                raise HTTPException(
                    status_code=400,
                    detail="Hunyuan3D requires a reference image. Text-to-image generation failed — please provide an image.",
                )

        if input_image is None:
            raise HTTPException(status_code=400, detail="A reference image is required (Hunyuan3D is image-conditioned only)")

        if req.remove_background and _rembg is not None:
            input_image = _rembg(input_image)

        # ── Shape generation (offloaded to GPU thread) ──────────────────────
        start = time.time()
        loop = asyncio.get_event_loop()
        mesh = await loop.run_in_executor(
            _gpu_executor,
            _run_shape_generation,
            input_image, req.prompt, req.steps, req.guidance_scale,
            req.octree_resolution, seed,
        )
        logger.info("Shape generation took %.1fs", time.time() - start)

        _set_progress(percentage=60.0, status="saving_shape")

        # Export untextured mesh
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        short_id = str(uuid.uuid4())[:8]
        base_name = req.custom_filename or f"gen3d_{timestamp}_{short_id}"

        initial_path = os.path.join(OUTPUT_DIR, f"{base_name}_initial.glb")
        mesh.export(initial_path)
        logger.info("Shape exported to %s", initial_path)

        # ── Texture generation ──────────────────────────────────────────────
        cleanup_files = [initial_path]
        filename = f"{base_name}.{req.output_format}"
        output_path = os.path.join(OUTPUT_DIR, filename)

        want_texture = not DISABLE_TEX and req.enable_texture
        if want_texture:
            _set_progress(percentage=65.0, status="generating_texture")
            logger.info("Starting texture generation…")

            try:
                textured_result = await loop.run_in_executor(
                    _gpu_executor,
                    _run_texture_generation,
                    initial_path, input_image, base_name,
                )

                if textured_result is not None:
                    textured_obj, tex_cleanup = textured_result
                    _set_progress(percentage=90.0, status="converting")

                    from hy3dpaint.convert_utils import create_glb_with_pbr_materials
                    textures = {
                        "albedo": textured_obj.replace(".obj", ".jpg"),
                        "metallic": textured_obj.replace(".obj", "_metallic.jpg"),
                        "roughness": textured_obj.replace(".obj", "_roughness.jpg"),
                    }
                    create_glb_with_pbr_materials(textured_obj, textures, output_path)
                    cleanup_files.extend(tex_cleanup)
                    logger.info("PBR GLB saved to %s", output_path)
                else:
                    # Texture generation returned None (failed gracefully)
                    _use_untextured(mesh, initial_path, output_path, req.output_format, cleanup_files)

            except Exception as tex_err:
                logger.warning("Texture failed, using untextured mesh: %s", tex_err, exc_info=True)
                _use_untextured(mesh, initial_path, output_path, req.output_format, cleanup_files)
        else:
            # No texture — use shape only
            _use_untextured(mesh, initial_path, output_path, req.output_format, cleanup_files)

        # Clean up intermediate files
        _cleanup_files(*cleanup_files)

        _set_progress(percentage=95.0, status="saving")

        # Save metadata sidecar
        meta_path = output_path + ".json"
        with open(meta_path, "w") as f:
            json.dump({
                "prompt": req.prompt,
                "seed": seed,
                "steps": req.steps,
                "guidance_scale": req.guidance_scale,
                "octree_resolution": req.octree_resolution,
                "num_views": req.num_views,
                "texture_size": req.texture_size,
                "output_format": req.output_format,
                "has_reference_image": input_image is not None,
                "low_vram_mode": LOW_VRAM_MODE,
            }, f)

        _set_progress(
            step=req.steps, total_steps=req.steps,
            percentage=100.0, status="complete",
        )

        return JSONResponse({
            "filename": filename,
            "format": req.output_format,
            "seed": seed,
            "file_size": os.path.getsize(output_path),
            "prompt": req.prompt,
            "steps": req.steps,
            "guidance_scale": req.guidance_scale,
            "octree_resolution": req.octree_resolution,
            "texture_size": req.texture_size,
        })

    except HTTPException:
        raise
    except Exception as e:
        logger.error("Generation failed: %s", e, exc_info=True)
        _set_progress(step=0, total_steps=0, percentage=0.0, status="error")
        raise HTTPException(status_code=500, detail=str(e))
    finally:
        # Unload shape pipeline — models live on the volume,
        # no need to hold ~9 GB in RAM between requests.
        _unload_shape_pipeline()
        with _progress_lock:
            if _progress.get("status") not in ("error", "complete"):
                _progress.update(step=0, total_steps=0, percentage=0.0, status="idle")
        _flush_vram()


@app.get("/api/v1/models")
async def list_models():
    return {"models": [{"name": MODEL_ID, "type": "3d", "formats": ["glb", "obj"]}]}


# ── Main ────────────────────────────────────────────────────────────────────────
if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=7861)
