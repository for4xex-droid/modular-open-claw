FROM python:3.11-slim

WORKDIR /app

# RT3-8 FIX: Remove git after timesfm installation to minimize attack surface.
RUN apt-get update && apt-get install -y --no-install-recommends git && rm -rf /var/lib/apt/lists/*

COPY requirements.txt .

RUN pip install --no-cache-dir torch>=2.0.0
RUN pip install --no-cache-dir fastapi==0.115.0 uvicorn==0.30.6 pydantic==2.8.2
RUN pip install --no-cache-dir git+https://github.com/google-research/timesfm.git#egg=timesfm

# RT3-8 FIX: Remove git and build tools after installation
RUN apt-get purge -y git && apt-get autoremove -y && rm -rf /var/lib/apt/lists/*

# RT3-9 FIX: Create non-root user
RUN groupadd -r sidecar && useradd -r -g sidecar -d /app -s /sbin/nologin sidecar
COPY main.py .
RUN chown -R sidecar:sidecar /app

USER sidecar

# RT3-10: Limit workers to 1 to prevent unbounded process spawning.
# TimesFM model is large; multiple workers would exceed the 4G memory limit.
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "3020", "--workers", "1", "--limit-concurrency", "4"]
