#!/bin/bash

# ComfyUI を「檻 (Jail)」の中で起動するスクリプト
# 期待される出力先を Rust 側のガードレールと物理的に同期させる。

# プロジェクトのルートディレクトリを取得
PROJECT_ROOT=$(cd $(dirname $0)/..; pwd)
WORKSPACE_DIR="$PROJECT_ROOT/workspace/shorts_factory"
COMFY_OUT_DIR="$WORKSPACE_DIR/comfy_out"

# ディレクトリの作成
mkdir -p "$COMFY_OUT_DIR"

echo "🔒 Starting ComfyUI with Synchronized Jail..."
echo "📂 Jail Root: $WORKSPACE_DIR"
echo "📁 Output Dir: $COMFY_OUT_DIR"

# ComfyUI の実行 (既存のインストールパスを想定、必要に応じて変更)
# --output-directory オプションで Rust 側の監視対象フォルダを強制指定する
if [ -d "ComfyUI" ]; then
    cd ComfyUI
    python3 main.py --output-directory "$COMFY_OUT_DIR" "$@"
else
    echo "⚠️  ComfyUI directory not found in project root."
    echo "Please ensure ComfyUI is installed at: $PROJECT_ROOT/ComfyUI"
    echo "Or run manually with: --output-directory $COMFY_OUT_DIR"
fi
