#!/usr/bin/env bash
# =============================================================================
# MLIT MCP Server Setup Script
# 国土交通省 MCP サーバーのセットアップスクリプト
# =============================================================================
# Usage: bash tools/setup-mlit-mcp.sh
#
# Prerequisites:
#   - Python 3.10+
#   - API keys (set in .env or pass as arguments)
#     MLIT_REINFOLIB_API_KEY: 不動産情報ライブラリ
#     MLIT_DPF_API_KEY:      国土交通データプラットフォーム
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MLIT_DIR="$PROJECT_ROOT/tools/mlit-mcp"

echo "🗾 MLIT MCP Server Setup"
echo "========================"

# --- Check Python ---
if ! command -v python3 &> /dev/null; then
    echo "❌ Python 3 is not installed. Please install Python 3.10+."
    exit 1
fi

PYTHON_VERSION=$(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')
echo "✅ Python $PYTHON_VERSION detected"

# --- Clone / Update Repos ---
mkdir -p "$MLIT_DIR"

echo ""
echo "📦 [1/2] Setting up mlit-geospatial-mcp (不動産情報ライブラリ)..."
if [ -d "$MLIT_DIR/mlit-geospatial-mcp" ]; then
    echo "   → Updating existing clone..."
    cd "$MLIT_DIR/mlit-geospatial-mcp" && git pull --ff-only 2>/dev/null || true
else
    git clone https://github.com/chirikuuka/mlit-geospatial-mcp.git "$MLIT_DIR/mlit-geospatial-mcp"
fi

cd "$MLIT_DIR/mlit-geospatial-mcp"
if [ ! -d ".venv" ]; then
    python3 -m venv .venv
fi
source .venv/bin/activate
pip install -q -r requirements.txt
deactivate
echo "   ✅ mlit-geospatial-mcp ready"

echo ""
echo "📦 [2/2] Setting up mlit-dpf-mcp (国土交通データプラットフォーム)..."
if [ -d "$MLIT_DIR/mlit-dpf-mcp" ]; then
    echo "   → Updating existing clone..."
    cd "$MLIT_DIR/mlit-dpf-mcp" && git pull --ff-only 2>/dev/null || true
else
    git clone https://github.com/MLIT-DATA-PLATFORM/mlit-dpf-mcp.git "$MLIT_DIR/mlit-dpf-mcp"
fi

cd "$MLIT_DIR/mlit-dpf-mcp"
if [ ! -d ".venv" ]; then
    python3 -m venv .venv
fi
source .venv/bin/activate
pip install -q -e .
pip install -q aiohttp pydantic tenacity python-json-logger mcp python-dotenv
deactivate
echo "   ✅ mlit-dpf-mcp ready"

# --- Generate mcp_servers.json ---
echo ""
echo "🔧 Generating ~/.aiome/mcp_servers.json..."

GEOSPATIAL_PYTHON="$MLIT_DIR/mlit-geospatial-mcp/.venv/bin/python"
GEOSPATIAL_SERVER="$MLIT_DIR/mlit-geospatial-mcp/src/server.py"
DPF_PYTHON="$MLIT_DIR/mlit-dpf-mcp/.venv/bin/python"
DPF_SERVER="$MLIT_DIR/mlit-dpf-mcp/src/server.py"

mkdir -p "$HOME/.aiome"

# Check if mcp_servers.json already exists
if [ -f "$HOME/.aiome/mcp_servers.json" ]; then
    echo "   ⚠️  ~/.aiome/mcp_servers.json already exists."
    echo "   → Add the following entries manually if needed:"
    echo ""
    echo "   \"mlit-geospatial\": {"
    echo "     \"command\": \"$GEOSPATIAL_PYTHON\","
    echo "     \"args\": [\"$GEOSPATIAL_SERVER\"],"
    echo "     \"env\": { \"LIBRARY_API_KEY\": \"\$MLIT_REINFOLIB_API_KEY\" }"
    echo "   }"
    echo ""
    echo "   \"mlit-dpf\": {"
    echo "     \"command\": \"$DPF_PYTHON\","
    echo "     \"args\": [\"$DPF_SERVER\"],"
    echo "     \"env\": { \"MLIT_API_KEY\": \"\$MLIT_DPF_API_KEY\", \"MLIT_BASE_URL\": \"https://data-platform.mlit.go.jp/api/v1/\" }"
    echo "   }"
else
    cat > "$HOME/.aiome/mcp_servers.json" << EOF
{
  "mcp_servers": {
    "mlit-geospatial": {
      "command": "$GEOSPATIAL_PYTHON",
      "args": ["$GEOSPATIAL_SERVER"],
      "env": {
        "LIBRARY_API_KEY": "\$MLIT_REINFOLIB_API_KEY",
        "PYTHONUNBUFFERED": "1",
        "LOG_LEVEL": "WARNING"
      }
    },
    "mlit-dpf": {
      "command": "$DPF_PYTHON",
      "args": ["$DPF_SERVER"],
      "env": {
        "MLIT_API_KEY": "\$MLIT_DPF_API_KEY",
        "MLIT_BASE_URL": "https://data-platform.mlit.go.jp/api/v1/",
        "PYTHONUNBUFFERED": "1",
        "LOG_LEVEL": "WARNING"
      }
    }
  }
}
EOF
    echo "   ✅ ~/.aiome/mcp_servers.json created"
fi

echo ""
echo "🎉 Setup complete!"
echo ""
echo "Next steps:"
echo "  1. Set API keys in .env:"
echo "     MLIT_REINFOLIB_API_KEY=\"your_key_here\""
echo "     MLIT_DPF_API_KEY=\"your_key_here\""
echo "  2. Restart Aiome — MCP Discovery will auto-connect the servers."
echo ""
echo "  Apply for keys:"
echo "    不動産情報ライブラリ: https://www.reinfolib.mlit.go.jp/api/request/"
echo "    国土交通データPF:   https://data-platform.mlit.go.jp/"
