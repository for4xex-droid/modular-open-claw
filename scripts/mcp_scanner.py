import urllib.request
import json
import concurrent.futures

URLS = [
    "https://api.github.com/repos/upstash/context7/readme",
    "https://api.github.com/repos/eyaltoledano/claude-task-master/readme",
    "https://api.github.com/repos/github/github-mcp-server/readme",
    "https://api.github.com/repos/awslabs/mcp/readme",
    "https://api.github.com/repos/microsoft/playwright-mcp/readme",
    "https://api.github.com/repos/googleanalytics/google-analytics-mcp/readme",
    "https://api.github.com/repos/zcaceres/markdownify-mcp/readme",
    "https://api.github.com/repos/jlowin/fastmcp/readme",
    "https://api.github.com/repos/mcp-use/mcphub/readme",
    "https://api.github.com/repos/modelcontextprotocol/servers/readme",
]

def fetch_readme(url):
    try:
        req = urllib.request.Request(url, headers={"Accept": "application/vnd.github.v3.raw", "User-Agent": "AiomeScanner"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            if resp.status == 200:
                text = resp.read().decode('utf-8')
                return url.split('repos/')[1].split('/readme')[0], text[:1000]
            else:
                return url, f"Failed: {resp.status}"
    except Exception as e:
        return url, f"Error: {e}"

with concurrent.futures.ThreadPoolExecutor(max_workers=5) as executor:
    results = executor.map(fetch_readme, URLS)
    for name, text in results:
        print(f"--- {name} ---")
        print(text)
        print()
