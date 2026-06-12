#!/usr/bin/env python3
"""
Aiome Custom MCP Server Boilerplate
Model Context Protocol (MCP) 準拠のツールサーバー実装例。
"""

import sys
import json
from typing import List, Dict, Any

# mcp SDK のインポートをエミュレート、または標準入出力での簡易JSON-RPCハンドラを記述し、
# 最小依存で動作するポータブルな MCP サーバーのボイラープレートを提供します。
class MCPServer:
    def __init__(self, name: str, version: str):
        self.name = name
        self.version = version
        self.tools = {}

    def register_tool(self, name: str, description: str, input_schema: Dict[str, Any]):
        def decorator(func):
            self.tools[name] = {
                "func": func,
                "description": description,
                "input_schema": input_schema
            }
            return func
        return decorator

    def run(self):
        """標準入出力から JSON-RPC メッセージを処理する簡易ループ"""
        print(f"DEBUG: Starting Custom MCP Server '{self.name}' v{self.version}", file=sys.stderr)
        
        while True:
            try:
                line = sys.stdin.readline()
                if not line:
                    break
                
                request = json.loads(line)
                method = request.get("method")
                req_id = request.get("id")
                params = request.get("params", {})

                if method == "initialize":
                    response = {
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "result": {
                            "protocolVersion": "2024-11-05",
                            "capabilities": {
                                "tools": {}
                            },
                            "serverInfo": {
                                "name": self.name,
                                "version": self.version
                            }
                        }
                    }
                    self._send(response)

                elif method == "tools/list":
                    response = {
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "result": {
                            "tools": [
                                {
                                    "name": name,
                                    "description": info["description"],
                                    "inputSchema": info["input_schema"]
                                }
                                for name, info in self.tools.items()
                            ]
                        }
                    }
                    self._send(response)

                elif method == "tools/call":
                    tool_name = params.get("name")
                    tool_args = params.get("arguments", {})
                    
                    if tool_name in self.tools:
                        try:
                            result = self.tools[tool_name]["func"](**tool_args)
                            result_content = [
                                {
                                    "type": "text",
                                    "text": str(result)
                                }
                            ]
                            response = {
                                "jsonrpc": "2.0",
                                "id": req_id,
                                "result": {
                                    "content": result_content
                                }
                            }
                        except Exception as e:
                            response = {
                                "jsonrpc": "2.0",
                                "id": req_id,
                                "error": {
                                    "code": -32603,
                                    "message": f"Internal error during tool call: {str(e)}"
                                }
                            }
                    else:
                        response = {
                            "jsonrpc": "2.0",
                            "id": req_id,
                            "error": {
                                      "code": -32601,
                                      "message": f"Tool '{tool_name}' not found"
                            }
                        }
                    self._send(response)

            except json.JSONDecodeError:
                pass
            except Exception as e:
                print(f"ERROR: {str(e)}", file=sys.stderr)

    def _send(self, message: Dict[str, Any]):
        sys.stdout.write(json.dumps(message) + "\n")
        sys.stdout.flush()

# ──────────────────────────────────────
# MCP サーバーの定義とツールの登録
# ──────────────────────────────────────
server = MCPServer("custom-calculator", "1.0.0")

@server.register_tool(
    name="calculate_agent_karma",
    description="エージェントの活動ファクターとインシデント履歴からカルマ寄与度を計算します。",
    input_schema={
        "type": "object",
        "properties": {
            "resolved_incidents": {
                "type": "integer",
                "description": "解決されたインシデントチケット数"
            },
            "autonomous_actions": {
                "type": "integer",
                "description": "自律活動の回数"
            }
        },
        "required": ["resolved_incidents", "autonomous_actions"]
    }
)
def calculate_agent_karma(resolved_incidents: int, autonomous_actions: int) -> str:
    # カルマ計算の模擬ロジック
    karma_contrib = (resolved_incidents * 15) + (autonomous_actions * 2)
    return json.dumps({
        "status": "success",
        "karma_contribution": karma_contrib,
        "assessment": "High Positive Contribution" if karma_contrib > 100 else "Standard Active Agent"
    })

if __name__ == "__main__":
    server.run()
