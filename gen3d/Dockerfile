# Needs devel image for CUDA toolkit (compiling custom rasterizer + DifferentiableRenderer)
FROM nvidia/cuda:12.4.1-devel-ubuntu22.04

ENV DEBIAN_FRONTEND=noninteractive
ENV HF_HOME=/models
ENV CUDA_HOME=/usr/local/cuda
ENV PATH=${CUDA_HOME}/bin:${PATH}
ENV LD_LIBRARY_PATH=${CUDA_HOME}/lib64:${LD_LIBRARY_PATH}
ENV TORCH_CUDA_ARCH_LIST="8.0;8.6;8.9;9.0"
ENV PYOPENGL_PLATFORM=egl
ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1

# System dependencies (OpenGL, EGL, build tools for CUDA extensions)
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential git wget curl cmake pkg-config \
    python3 python3-dev python3-pip python3-setuptools \
    libegl1-mesa-dev libglib2.0-0 libeigen3-dev \
    libglvnd0 libgl1 libglx0 libegl1 libgles2 \
    libglvnd-dev libgl1-mesa-dev libgles2-mesa-dev \
    libxrender1 libxi6 libsm6 libxext6 \
    mesa-utils-extra ninja-build && \
    ln -sf /usr/bin/python3 /usr/bin/python && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Install PyTorch (must match CUDA 12.4)
RUN pip3 install --no-cache-dir \
    torch==2.5.1 torchvision==0.20.1 torchaudio==2.5.1 \
    --index-url https://download.pytorch.org/whl/cu124

# Clone Hunyuan3D-2.1
RUN git clone https://github.com/Tencent-Hunyuan/Hunyuan3D-2.1.git hunyuan3d

# Install Python dependencies (skip Blender, Gradio, DeepSpeed, and Chinese mirrors)
RUN cd hunyuan3d && \
    grep -v -E '^(bpy|gradio|deepspeed|pythreejs|--extra-index-url)' requirements.txt \
    > requirements_filtered.txt && \
    pip3 install --no-cache-dir -r requirements_filtered.txt && \
    pip3 install --no-cache-dir hf_xet

# Install bpy stub — the texture pipeline imports bpy at module level
# (DifferentiableRenderer/mesh_utils.py) but we never call bpy-dependent
# functions (save_glb=False).  This stub allows the import to succeed.
COPY bpy_stub.py /tmp/bpy_stub.py
RUN SITE=$(python3 -c "import site; print(site.getsitepackages()[0])") && \
    mkdir -p "$SITE/bpy" && \
    cp /tmp/bpy_stub.py "$SITE/bpy/__init__.py" && \
    rm /tmp/bpy_stub.py

# Build custom_rasterizer (CUDA extension for PBR texture rendering)
RUN cd hunyuan3d/hy3dpaint/custom_rasterizer && \
    pip3 install --no-cache-dir -e .

# Build DifferentiableRenderer (CUDA mesh painter)
RUN cd hunyuan3d/hy3dpaint/DifferentiableRenderer && \
    bash compile_mesh_painter.sh

# Download RealESRGAN weights for texture upscaling
RUN mkdir -p hunyuan3d/hy3dpaint/ckpt && \
    wget -q https://github.com/xinntao/Real-ESRGAN/releases/download/v0.1.0/RealESRGAN_x4plus.pth \
    -P hunyuan3d/hy3dpaint/ckpt/

# Patch config paths to resolve from repo root (matches official docker/Dockerfile)
RUN cd hunyuan3d/hy3dpaint && \
    sed -i 's|self\.multiview_cfg_path = "cfgs/hunyuan-paint-pbr\.yaml"|self.multiview_cfg_path = "hy3dpaint/cfgs/hunyuan-paint-pbr.yaml"|' \
    textureGenPipeline.py
RUN cd hunyuan3d/hy3dpaint/utils && \
    sed -i 's|custom_pipeline = config\.custom_pipeline|custom_pipeline = os.path.join(os.path.dirname(__file__),"..","hunyuanpaintpbr")|' \
    multiview_utils.py

# Working directory = repo root (required for relative config paths)
WORKDIR /app/hunyuan3d

# Copy our custom server
COPY download_models.py server.py entrypoint.sh ./
RUN chmod +x entrypoint.sh

ENV OUTPUT_DIR=/app/outputs
ENV PYTHONPATH=/app/hunyuan3d:/app/hunyuan3d/hy3dpaint:$PYTHONPATH

VOLUME ["/app/outputs", "/models"]

EXPOSE 7861

HEALTHCHECK --interval=30s --timeout=10s --retries=3 --start-period=1800s \
    CMD curl -f http://localhost:7861/ || exit 1

ENTRYPOINT ["./entrypoint.sh"]
