FROM pytorch/pytorch:2.5.1-cuda12.4-cudnn9-runtime

ENV DEBIAN_FRONTEND=noninteractive
ENV HF_HOME=/models

RUN apt-get update && \
    apt-get install -y --no-install-recommends curl git && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Install dependencies
RUN pip install --no-cache-dir \
    diffusers \
    "transformers>=4.47.0" \
    accelerate \
    safetensors \
    fastapi \
    "uvicorn[standard]" \
    "huggingface_hub[hf_xet]" \
    sentencepiece \
    protobuf

COPY download_models.py server.py entrypoint.sh ./
RUN chmod +x entrypoint.sh

EXPOSE 7860

HEALTHCHECK --interval=30s --timeout=10s --retries=3 --start-period=1800s \
    CMD curl -f http://localhost:7860/ || exit 1

ENTRYPOINT ["./entrypoint.sh"]
