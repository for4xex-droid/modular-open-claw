FROM python:3.11-slim

WORKDIR /app

# Install uv for fast resolution
RUN pip install uv

# Create a virtual environment and install dependencies
RUN uv venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"

# Install FastAPI server and the skill
RUN uv pip install fastapi uvicorn pydantic "geo-optimizer-skill>=1.0.0"

COPY main.py .

EXPOSE 8080

CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8080"]
