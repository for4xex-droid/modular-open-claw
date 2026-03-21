# ADR 012: マルチソース・トレンド集約システムの導入

## ステータス
提案中 (Proposed)

## コンテキスト
プロジェクト NURTURE の「フェーズ 12.5 (Dream State Refinement)」において、AIが自律的にトレンドを収集し、新しいジョブ（幻のジョブ）を生成する機能の強化が必要となった。
従来の `ExternalTrendSonar` は単一の Web Search API キーに依存しており、以下の課題があった：
1. **データソースの固定**: Web検索以外の情報源（RSSフィード、SNS、ニュースAPI等）を柔軟に追加できない。
2. **テスト可能性の低さ**: 外部APIに強く結合しており、モック化や集約ロジックの検証が困難。
3. **レジリエンス不足**: 単一のAPIが失敗した場合にトレンド収集全体が停止する。

## 意思決定
アダプターパターンを採用し、`TrendAdapter` トレイトを導入することで、トレンド収集ロジックを抽象化する。

1. **`TrendAdapter` トレイトの定義**:
   - `fetch(query)` および `name()` メソッドを持つ非同期トレイトを定義。
2. **`ExternalTrendSonar` の再定義**:
   - `Vec<Arc<dyn TrendAdapter>>` を保持し、複数のアダプターを管理するように変更。
   - `get_trends` メソッドは登録された全アダプターからデータを並行/順次収集し、結果を集約する。
3. **アダプターの実装**:
   - `WebSearchAdapter`: 以前の Web Search API 機能を移行。Snippet のサニタイズ機能を内蔵。
   - `RssCollector`: 既存の RSS コレクターを `TrendAdapter` として統合。
4. **インスタンスの共通化**:
   - `api-server` の起動時に `TrendSonar` を一括初期化し、`AppState` または `BackgroundWorker` のローカル変数として共有。

## 影響範囲
- `libs/infrastructure/src/trend_sonar.rs`: コアロジックの変更。
- `libs/infrastructure/src/rss_collector.rs`: アダプター実装の追加。
- `apps/api-server/src/main.rs`: 初期化フローと `BackgroundWorker` 内の呼び出し箇所の修正。
- `libs/infrastructure/src/dream_state.rs`: テストコードの修正。

## メリット
- **拡張性**: 新しいトレンドソース（例：GitHub Trending, Twitter Search）を `TrendAdapter` を実装するだけで簡単に追加できる。
- **堅牢性**: 一方のアダプターが失敗しても、他のアダプターからのデータ収集を継続できる。
- **保守性**: 各データソースのロジックが独立し、ユニットテストが容易になる。
- **データ品質**: 出力データのサニタイズがアダプターレベルで強制され、システム全体の入力品質が向上する。

## デメリット
- **複雑性の微増**: 初期化フローで複数のアダプターを生成・登録する手順が必要。
- **メモリ消費**: 複数のアダプター（特に `RssCollector` などの重いもの）を保持するため、微小ながらメモリ使用量が増加する。
