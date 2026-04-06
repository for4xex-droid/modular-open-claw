# ADR-025: Agent-Native Document Discovery via File System Projection

- Status: Accepted
- Created: 2026-04-06
- Context: Duke University Research — Coding agents + file system navigation > million-token context windows

## Problem

Cortex Knowledge Base は DB + embedding ベースのセマンティック検索（RAG）に依存している。
Duke大学の研究（2026年）により、コーディングエージェントが `grep`/`sed`/`cat` を用いた
ファイルシステム探索でドキュメントを処理する方が、100万トークンコンテキストウィンドウや
RAG embeddings を使用するよりも平均 **+17.3%** 高い精度を達成することが実証された。

### Key Findings

1. **ファイルシステム = 座標系**: ディレクトリ階層にドキュメントを配置すると +6pt の精度向上
2. **Retriever は天井になる**: BM25/dense embedding を追加するとエージェント固有の探索行動が抑制され性能が低下し得る
3. **創発的マルチホップ推論**: エージェントは指示なしに6ホップの反復クエリ精錬チェーンを自律構築

## Decision

1. **CortexFileProjector** を新設し、DB上のWiki記事をファイルシステム階層として物理投影する
2. **DreamState** の科学的仮説検証タスクにおいて、投影されたファイルインデックスをプロンプトに注入する（Agent-Native Discovery モード）
3. **CortexCompiler** のコンパイルサイクル後に自動投影を行う
4. 既存の embedding-based RAG は人間向けクエリ用として温存する（デュアルモード戦略）

## Consequences

- エージェントの自律タスク（DreamState Scientific Experiment）の精度が向上する
- ファイルシステムI/Oが発生するが、差分投影（content_hash ベース）により最小化
- 将来的に MCP ツール (`cortex_fs_search`) として外部エージェントに公開可能

## References

- Duke University, "Coding Agents are Better Document Processors", 2026
- BrowseComp-Plus benchmark: 88.5% vs 80.0% (+11% relative)
- Oolong-Real benchmark: 37.46 vs 24.09 (+56% relative)
- Natural Questions (3T tokens): 56.0% vs 50.9% (+10% relative)
