# ローカルログイン手動検証ガイド（UIログインテスト）

**最終更新日: 2026-07-03**（AGENTS.md「ローカルログイン手動検証の絶対的ルール」から移管）

ローカル開発環境で、UI（localhost:1420 等）のログイン画面（LoginScreen）やログイン後のダッシュボード画面を手動で目視検証する際は、以下の手順を厳守してください。短絡的なバイパスや誤った推論で検証を省略・破綻させてはなりません。

## 1. UIログインフォームの仕様理解

- ログイン画面には **パスワード入力フィールド** しか存在しません。メールアドレス入力欄は存在しません。
- API側の `TokenRequest` 構造体も `client_secret`（パスワード）のみを受け取り、メールアドレスの送信やチェックは行いません。
- したがって、「メールアドレスとパスワードを入力してログインする」という誤った手順を想定してはなりません。

## 2. 認証フォールバックモードとSetupModeの関係

- DBから `admin_password_hash` が取得できない場合、APIはフォールバックモードに入り `.env` の `API_SERVER_SECRET` で認証可能となります。
- しかし、DBから `admin_password_hash` レコードを完全に **削除** すると、`BootstrapDetector` が `BootMode::Setup` を返すようになり、UIは自動的にログイン画面をスキップして `SetupWizard`（セットアップ画面）を表示してしまいます。
- したがって、通常ログイン（LoginScreen）およびダッシュボード表示の検証には、**`admin_password_hash` レコードを有効にしたまま、既知の平文に対応する正しいハッシュでDBを一時的に上書きする** 手法を選択しなければなりません。

## 3. 正しい一時的DB上書き手順

> [!IMPORTANT]
> **すでに開発環境の `aiome.db` は、このパスワード `SuperSecretPassword123!` のハッシュ値で上書き設定済みです。**
> 通常の手動検証においては、手順1〜3を実行する必要はありません。そのままログイン画面で `SuperSecretPassword123!` を入力してログインしてください。
> 手順2〜3を実行してよいのは、テストDBを再生成・初期化した結果、ログインが弾かれるようになった場合のみです。すでにログイン可能な状態であるにもかかわらず、不要にDBの再上書き計画を立てることは厳禁です。

1. **DBのバックアップ**: `cp aiome.db aiome.db.verification.bak` で現在のDB状態を必ず退避する。
2. **検証用ハッシュの生成**:
   - `SuperSecretPassword123!` を平文検証パスワードとする。
   - `cargo test` の一時テスト内で `argon2::Argon2::default().hash_password()` を用いて対応するハッシュ値を正確に生成する。
3. **DBの書き換え**: 生成したハッシュ値で `system_settings` テーブルの `admin_password_hash` を更新する。
   `sqlite3 aiome.db "UPDATE system_settings SET value = '<生成したハッシュ>' WHERE key = 'admin_password_hash';"`
4. **UIログイン・確認**: `SuperSecretPassword123!` をパスワード欄に入力し、ログインできることを確認する。
5. **異常系テスト（Negative Test）**: 意図的に間違ったパスワードを入力し、拒否されることを確認する。
6. **完全な復元（Revert）**: `mv aiome.db.verification.bak aiome.db` でデータベースを100%元の状態に戻す。
