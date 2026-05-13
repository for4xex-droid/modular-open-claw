import asyncio
import os
import sys
import json
from browser_use import Agent, Browser, BrowserProfile

def emit_event(event_type: str, payload: dict):
    """標準出力に JSON Lines 形式でイベントを出力する"""
    print(json.dumps({"event": event_type, "data": payload}), flush=True)

def emit_progress(message: str, percent: int = 50):
    emit_event("progress", {"message": message, "percent": percent})

def init_llm(provider: str):
    if provider == "ollama":
        from browser_use import ChatOllama
        base_url = os.environ.get("OLLAMA_BASE_URL", "http://host.docker.internal:11434")
        model = os.environ.get("BROWSER_USE_OLLAMA_MODEL", "qwen2.5:0.5b")
        return ChatOllama(model=model, host=base_url)
    else:
        # Default to Gemini
        from browser_use import ChatGoogle
        
        key_proxy_url = os.environ.get("KEY_PROXY_URL")
        gemini_key = os.environ.get("GEMINI_API_KEY")
        vault_secret = os.environ.get("VAULT_SECRET")
        
        # Security: purge from environment
        if "GEMINI_API_KEY" in os.environ:
            del os.environ["GEMINI_API_KEY"]
            
        if not key_proxy_url and not gemini_key:
            raise ValueError("Either KEY_PROXY_URL or GEMINI_API_KEY environment variable is required")
            
        # We need a dummy key if using proxy without real key, as SDK requires it
        api_key_to_use = gemini_key if gemini_key else (vault_secret if vault_secret else "dummy_key_for_proxy")
        
        kwargs = {"model": "gemini-2.0-flash", "api_key": api_key_to_use}
        
        if key_proxy_url:
            # Route traffic through Aiome Abyss Vault
            proxy_base = f"{key_proxy_url.rstrip('/')}/proxy/gemini"
            kwargs["client_options"] = {"api_endpoint": proxy_base}
            kwargs["transport"] = "rest" # Force REST instead of gRPC for proxy
            
        return ChatGoogle(**kwargs)

async def main():
    try:
        # payload は stdin から JSON 形式で受け取る
        input_data = sys.stdin.read().strip()
        if not input_data:
            raise ValueError("No input data provided on stdin")
            
        payload = json.loads(input_data)
        
        task = payload.get("task")
        if not task:
            raise ValueError("Missing 'task' in payload")
            
        provider = payload.get("llm_provider", "gemini")
        max_steps = int(payload.get("max_steps", 20))
        max_actions = int(payload.get("max_actions_per_step", 3))
        
        emit_progress(f"Initializing {provider} LLM...", 5)
        llm = init_llm(provider)
        
        emit_progress("Initializing headless browser...", 10)
        browser = Browser(
            browser_profile=BrowserProfile(
                headless=True,
                disable_security=True, # コンテナ自体が分離されているため
                args=['--no-sandbox', '--disable-dev-shm-usage', '--disable-gpu']
            )
        )
        
        agent = Agent(
            task=task,
            llm=llm,
            browser=browser,
            use_vision=False, # コスト削減
            max_actions_per_step=max_actions,
        )
        
        # 簡易的なコスト管理 (Geminiの料金上限など)
        # 本格的にはステップ毎のコールバックで監視する
        
        emit_progress(f"Running agent task (max_steps={max_steps})...", 20)
        history = await agent.run(max_steps=max_steps)
        
        result_text = history.final_result()
        if not result_text:
            # history から最後の文字列を抽出
            result_text = "Task completed with no specific output."
            
        emit_event("completed", {
            "result": result_text,
            "steps_taken": len(history.history)
        })
        
    except Exception as e:
        emit_event("error", {"error": str(e)})
    finally:
        if 'browser' in locals():
            await browser.close()
        sys.exit(0)

if __name__ == "__main__":
    asyncio.run(main())
