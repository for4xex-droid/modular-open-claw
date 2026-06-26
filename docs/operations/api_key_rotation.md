# APIキーおよびシークレットのローテーション（更新）運用手順

本ドキュメントは、`aiome` のシークレット隔離機構（`key-proxy` ↔ 各サービス間連携）において、APIキーやシークレット情報を安全に更新・ローテーションするための運用手順を定義します。

---

## 1. シークレットの分類と格納先

`aiome` では、セキュリティレベルと用途に応じて、シークレットの格納先が2箇所に分かれています。

| 分類 | 対象キー例 | 格納先（macOS / 開発機） | 格納先（本番 / Linux） |
| :--- | :--- | :--- | :--- |
| **ブートストラップ・キー**<br>(基本となる最重要シークレット) | `VAULT_MASTER_PASSWORD`<br>`VAULT_SECRET`<br>`GEMINI_API_KEY` | **macOS Keychain**<br>(セキュアストレージ) | **環境変数**<br>(起動プロセスへの直接指定) |
| **一般シークレット**<br>(サードパーティAPIキーなど) | `STRIPE_SECRET_KEY`<br>その他のAPIキーなど | **AbyssVault (SQLite DB)**<br>(XChaCha20-Poly1305暗号化) | **AbyssVault (SQLite DB)**<br>(XChaCha20-Poly1305暗号化) |

---

## 2. APIキーの更新（ローテーション）手順

シークレットの管理には、プロジェクトに同梱されている管理コマンド `abyss-vault` CLI を使用します。

### A. ブートストラップ・キーを更新する場合 (macOS)
macOS の Keychain に保存されている `VAULT_SECRET` や `GEMINI_API_KEY` などを更新します。

```bash
# 例: Gemini APIキーのみを更新する場合
cargo run --bin abyss-vault -- bootstrap --gemini-api-key <新しいAPIキー>

# 例: マスターパスワードやVaultシークレットを更新する場合
cargo run --bin abyss-vault -- bootstrap --master-password <新しいパスワード> --vault-secret <新しいトークン>
```

> **Note:**
> 本番環境（LinuxなどKeychainが使えない環境）では、これらのブートストラップ・キーは各プロセスの起動時の環境変数を通じて直接指定します。環境変数を更新してコンテナやプロセスを再起動してください。

---

### B. 管理コンソール（GUI）から更新する場合
非技術者のユーザーや簡易的な管理を行いたい場合は、管理コンソール（GUI）から安全にシークレットを管理することができます。

1. **管理コンソールを開く**:
   「設定」ページ（SettingsPage）を開きます。
2. **Abyss Vault Secrets Manager セクション**:
   画面の下部にある「Abyss Vault シークレットマネージャ」セクションに移動します（このセクションは `viewMode` が `beginner` であっても常時表示されます）。
3. **キーの設定・更新**:
   設定したいシークレット（例: `GEMINI_API_KEY`）の右側にある「設定」ボタンをクリックします。
   ポップアップモーダルが表示されるので、新しいAPIキーの値を入力し、「保存」ボタンをクリックします。
4. **キーの削除**:
   すでに設定されているシークレットについては、右側の「ゴミ箱（削除）」ボタンをクリックし、確認ダイアログで「OK」を選択することで Vault から完全に削除することができます。

---

### C. 一般シークレットを個別更新・追加する場合 (AbyssVault CLI)
`Stripe` のシークレットキーなど、AbyssVault (SQLite) 内に暗号化保存されているキーを更新します。

```bash
# 例: GEMINI_API_KEY を対話形式（入力が画面に非表示）で安全に更新する場合
cargo run --bin abyss-vault -- set GEMINI_API_KEY

# 例: 直接値を指定して更新する場合
cargo run --bin abyss-vault -- set GEMINI_API_KEY <新しいAPIキー>
```
* `abyss-vault set` コマンドは内部的に `UPSERT` 処理を行うため、既存のキーがあれば新しい値で上書きされ、存在しない場合は新規登録されます。
* 値を指定せずに `set <KEY>` を実行した場合、入力値がシェル履歴に残らないよう非表示のプロンプトが表示されます。

---

### D. 設定済みのシークレット一覧やステータスを確認する場合 (AbyssVault CLI)
現在どのシークレットが登録されているか、一覧や詳細を確認できます。

```bash
# ホワイトリストにある18のシークレットの設定状況（✅ / ❌）を一覧表示
cargo run --bin abyss-vault -- status

# 現在登録されているキー名のみを一覧表示（値は非表示）
cargo run --bin abyss-vault -- list

# 特定のシークレットキーの値を復号して表示
cargo run --bin abyss-vault -- get GEMINI_API_KEY

# 特定のシークレットキーを削除（確認プロンプトが表示されます）
cargo run --bin abyss-vault -- delete GEMINI_API_KEY

# 未設定のシークレットキーを対話的に順次セットアップする
cargo run --bin abyss-vault -- setup
```

---

### E. 複数シークレットを一括インポート・更新する場合 (AbyssVault)
`.env.secret` などのファイルからシークレットを一括で読み込んで更新します。

1. **テンプレートの用意**:
   プロジェクトルートにあるテンプレートファイル [.env.secret.example](file:///Users/motista/Desktop/antigravity/aiome/.env.secret.example) をコピーして `.env.secret` を作成します。
   ```bash
   cp .env.secret.example .env.secret
   ```
2. **値の変更**:
   作成した `.env.secret` 内の、設定したい各キーの右辺の値を実際のシークレット情報に書き換えます。
3. **インポートコマンドの実行**:
   ```bash
   cargo run --bin abyss-vault -- import --file .env.secret
   ```
4. **平文ファイルの削除**:
   インポート完了後、不要になった平文の `.env.secret` ファイルはセキュリティ確保のため必ず物理削除してください。
   ```bash
   rm .env.secret
   ```

---

### F. Stripe Webhook シークレットの複数指定とローテーション
Stripe Webhook v2 thin event への移行やシークレットのローテーション（更新）に伴い、新旧のシークレットを同時に有効化して移行期間中のダウンタイムをゼロにするため、`STRIPE_WEBHOOK_SECRET` は**カンマ区切りによる複数シークレットの指定**に対応しています。

1. **新旧シークレットの並列設定**:
   Stripe 管理画面で新しく Webhook 宛先を作成、またはシークレットを再生成した際、新旧両方のシークレットをカンマ区切りで Vault または `.env.secret` に設定します。
   ```bash
   # 例: 2つの Webhook シークレットを登録する場合
   cargo run --bin abyss-vault -- set STRIPE_WEBHOOK_SECRET "YOUR_OLD_SECRET,YOUR_NEW_SECRET"
   ```
2. **反映と検証**:
   シークレット設定後、サービス（`api-server`）を再起動します。署名検証（`verify_signature`）時にカンマで分割された各シークレットがループで順次試行され、いずれか1つで検証が成功すれば Webhook リクエストが承認されます。
3. **旧シークレットのクリーンアップ**:
   Stripe 側でのトラフィック移行が完全に完了し、旧 Webhook 宛先を削除した後は、安全のため旧シークレットを `STRIPE_WEBHOOK_SECRET` から除外して新シークレットのみに更新し、再度再起動します。

---

## 3. 更新内容のシステムへの反映フロー（重要）

`abyss-vault` コマンドでデータベースや Keychain の値を更新しただけでは、現在実行中の `api-server` や `samsara-hub` などのサービスには**即時反映されません**。

### 反映の仕組みと手順
1. **ブートストラップ (起動) 時の注入**:
   各サービス (`api-server` や `samsara-hub` など) は、起動時にのみ `shared::security::fetch_and_inject_secrets()` を実行し、`key-proxy` から許可されたシークレット（ホワイトリスト検証済み）を動的取得してメモリ（環境変数等）に注入します。
2. **ローリング再起動の実行**:
   シークレットを更新した後は、稼働中のプロセスを**再起動**（またはコンテナの再デプロイ）する必要があります。
   - 例: `kill` シグナルによる安全なシャットダウンと再起動、またはコンテナオーケストレータにおけるローリングアップデートを実行します。
3. **注入の確認**:
   起動時のログに以下のようなデバッグログが出力され、新しいシークレットが正常に注入されたことを確認します。
   ```
   [DEBUG] Successfully fetched and injected secrets from key-proxy
   ```

---

## 4. 運用のベストプラクティスとセキュリティ上の注意点

1. **環境変数スクラビングの徹底**:
   `aiome` には起動後に環境変数からシークレットをメモリ上からクリアするスクラビング機能 (`scrub_env`) が備わっていますが、ファイル自体（`.env`）に平文のAPIキーを書き残したまま放置しないようにしてください。
2. **ホワイトリスト制限 (`ALLOWED_VAULT_SECRETS`)**:
   `key-proxy` が配信を許可するキーは `libs/shared/src/security.rs` のホワイトリストに登録されているものに限定されます。新しい種類のエコシステムAPIキーを追加する場合は、事前にソースコード側のホワイトリストにキー名を追加する必要があります。
