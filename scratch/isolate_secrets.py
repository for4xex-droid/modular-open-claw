import os
import sys
import stat

SECRET_KEYS = {
    "API_SERVER_SECRET", "NURTURE_INTERNAL_SECRET", "FEDERATION_SECRET",
    "JWT_PRIVATE_KEY_B64", "FAL_KEY", "GEMINI_API_KEY", "OPENAI_API_KEY",
    "TTS_OPENAI_API_KEY", "STRIPE_API_KEY", "STRIPE_WEBHOOK_SECRET",
    "STRIPE_PRICE_SUBSCRIPTION_MONTHLY", "TREMENDOUS_API_KEY", "WP_API_TOKEN",
    "SEARCH_API_KEY", "FIRECRAWL_API_KEY", "EXA_API_KEY", "BRIGHTDATA_API_KEY",
    "MLIT_REINFOLIB_API_KEY", "MLIT_DPF_API_KEY", "X_BEARER_TOKEN",
    "VAULT_MASTER_PASSWORD", "POLAR_API_KEY", "POLAR_WEBHOOK_SECRET",
    "A2A_AUTH_TOKEN", "A2A_NODE_TOKEN", "TIMESFM_AUTH_TOKEN", "DISCORD_TOKEN",
    "TELEGRAM_TOKEN", "DISCORD_WEBHOOK_URL"
}

def main():
    workspace_dir = "/Users/motista/Desktop/antigravity/aiome"
    env_path = os.path.join(workspace_dir, ".env")
    secret_path = os.path.join(workspace_dir, ".env.secret")

    if not os.path.exists(env_path):
        print(f"Error: .env not found at {env_path}")
        sys.exit(1)

    with open(env_path, "r", encoding="utf-8") as f:
        env_lines = f.readlines()

    new_env_lines = []
    secret_lines = []

    for line in env_lines:
        trimmed = line.strip()
        # コメント行や空行はそのまま維持
        if not trimmed or trimmed.startswith("#"):
            new_env_lines.append(line)
            continue

        if "=" in trimmed:
            key, val = trimmed.split("=", 1)
            key = key.strip()
            # プレースホルダーで値がない場合はシークレットに移行せず .env に残すか空にする
            # プレースホルダー文字列の判定: "<YOUR_KEY_HERE>" など
            val_clean = val.strip().strip('"').strip("'")
            
            if key in SECRET_KEYS:
                if val_clean == "<YOUR_KEY_HERE>" or not val_clean:
                    # 空またはプレースホルダーなら .env に残す
                    new_env_lines.append(line)
                else:
                    # 有効なシークレット値がある場合は .env.secret へ移動
                    secret_lines.append(line)
                    # 元の .env ではプレースホルダーに置き換える
                    new_env_lines.append(f'{key}="<YOUR_KEY_HERE>"\n')
            else:
                new_env_lines.append(line)
        else:
            new_env_lines.append(line)

    # .env.secret の書き込み (追記/マージ)
    existing_secrets = {}
    if os.path.exists(secret_path):
        with open(secret_path, "r", encoding="utf-8") as f:
            for s_line in f:
                s_trimmed = s_line.strip()
                if "=" in s_trimmed and not s_trimmed.startswith("#"):
                    s_key, s_val = s_trimmed.split("=", 1)
                    existing_secrets[s_key.strip()] = s_line

    for line in secret_lines:
        if "=" in line:
            key, _ = line.strip().split("=", 1)
            existing_secrets[key.strip()] = line

    # 最終的な .env.secret 書き出し
    with open(secret_path, "w", encoding="utf-8") as f:
        f.write("# Isolated Secrets for Aiome\n# Strictly ignored in git\n\n")
        for key in sorted(existing_secrets.keys()):
            f.write(existing_secrets[key])

    # パーミッションを 600 (所有者のみ読み書き可) に設定
    os.chmod(secret_path, stat.S_IRUSR | stat.S_IWUSR)
    print(f"Successfully isolated {len(secret_lines)} secrets to {secret_path}")

    # .env の書き換え
    with open(env_path, "w", encoding="utf-8") as f:
        f.writelines(new_env_lines)
    print(f"Successfully updated {env_path}")

if __name__ == "__main__":
    main()
