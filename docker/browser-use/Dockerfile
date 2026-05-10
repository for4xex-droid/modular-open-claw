FROM mcr.microsoft.com/playwright/python:v1.49.0-noble

WORKDIR /app

# Install dependencies
RUN pip install --no-cache-dir \
    browser-use \
    langchain-google-genai \
    langchain-ollama

COPY entrypoint.py /app/entrypoint.py

ENTRYPOINT ["python", "/app/entrypoint.py"]
