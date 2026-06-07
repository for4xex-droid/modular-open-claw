import os
import re

# 正規表現で LlmResponse { ... } をマッチング
# 括弧のネストは考慮しない単純なマッチ（通常、MockのLlmResponseはネストしない）
pattern = re.compile(r'(LlmResponse\s*\{)(.*?)(^\s*\})', re.MULTILINE | re.DOTALL)

def process_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    def replacer(match):
        prefix = match.group(1)
        body = match.group(2)
        suffix = match.group(3)

        if '..Default::default()' in body:
            return match.group(0)

        # reasoning: None, metadata: None, を削除
        body = re.sub(r'^\s*reasoning:\s*None,?\s*$', '', body, flags=re.MULTILINE)
        body = re.sub(r'^\s*metadata:\s*None,?\s*$', '', body, flags=re.MULTILINE)
        
        # AiomeError::LlmResponse { ... } のようなエラーEnumとの誤爆を防ぐ
        if 'content:' not in body and 'stop_reason:' not in body:
            return match.group(0)

        body = body.rstrip()
        if body and not body.endswith(','):
            body += ','
        
        # suffixのインデントに合わせる
        indent = match.group(3).replace('}', '')
        body += f'\n{indent}    ..Default::default()\n'

        return f"{prefix}{body}{suffix}"

    new_content = pattern.sub(replacer, content)

    # fallback_router.rs の特別処理 (return Ok(resp))
    if "fallback_router.rs" in filepath:
        # 特別な置換
        pass

    if new_content != content:
        with open(filepath, 'w') as f:
            f.write(new_content)
        print(f"Updated {filepath}")

def main():
    from pathlib import Path
    base_dir = Path(__file__).resolve().parents[1]
    dirs = [
        str(base_dir / "libs"),
        str(base_dir / "apps/api-server/src"),
    ]
    for d in dirs:
        for root, _, files in os.walk(d):
            for file in files:
                if file.endswith('.rs'):
                    process_file(os.path.join(root, file))

if __name__ == '__main__':
    main()
