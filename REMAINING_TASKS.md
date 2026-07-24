# 📋 プロジェクト全体 残存タスク集約レポート (REMAINING_TASKS.md)

> [!IMPORTANT]
> **Aiome 側タスクは [OPEN.md](OPEN.md) が正本。**  
> Nurture / commercial 残の **実行正本** は [`docs/roadmaps/nurture_remaining_ledger_plan.md`](docs/roadmaps/nurture_remaining_ledger_plan.md) **v1.3**。  
> 本 §3 は Disposition 付きスナップショット（2026-07-25 同期）。

**最終更新日**: 2026-07-25  
**対象プロジェクト**: `aiome` (OSS) & `commercial/`（旧 Project-Nurture 相当。スタンドアロン `Project-Nurture/` はミラー）

---

## 1–2. Aiome 側

Aiome 側の未解決・解決は **OPEN.md** を参照（本ファイル §1–2 の旧リストは OPEN へ移入済みで陳腐化）。

---

## 3. Project-Nurture / commercial 残（Disposition）

正本ドキュメント: [`commercial/docs/UNCERTAINTY_BREAKTHROUGH.md`](commercial/docs/UNCERTAINTY_BREAKTHROUGH.md)（superseded 注記あり） / [`commercial/`](commercial/)  
計画: [`docs/roadmaps/nurture_remaining_ledger_plan.md`](docs/roadmaps/nurture_remaining_ledger_plan.md)

| ID | 旧項目 | Disposition | 備考 |
|---|---|---|---|
| NR-01 | TLA+ `NurtureEconomyProtocol` 策定＋TLC | **DONE（仕様）/ CI 配線済** | `commercial/specs/`。Wave A ✅（TLC 実行は formal-verify） |
| NR-02 | VRM 15fps / LLM VRAM セマフォ | **DEFER** | Phase E（親計画未作成）。`VramArbiter` はサイドカー用で別物 |
| NR-03 | On-memory DRM `tauri://vrm/` | **DEFER** | Phase E E4。サーバ `DrmEngine` は既存 |
| NR-04 | Saga `Compensable` / `CompensationLog` | **FREEZE** | 型状態 + `nurture_saga_logs` + `rollback` で運用可 |
| NR-05 | ZKP / CoinQuantum / 経済 CRDT | **FREEZE** | Automerge CRDT は経済外で完了 |
| NR-06 | CSAM BoneChecker 実機調整 | **HUMAN** | コードは 1/5.5・fail-closed 済。コーパスのみ |
| NR-07 | `MAX_TOTAL_OUTSTANDING_COINS` | **LEGAL GATE** | 無償 KC 中は非該当（`KC_LEGAL_POSITION.md`） |
| NR-08 | 特商法表記ページ | **DONE** | LP `/tokushoho` |
| NR-09 | `PurchasePolicy` 3 モード | **PRODUCT GATE（既定凍結）** | 実効=MCP whitelist 未解禁 + HTTP eKYC/Pro。新 enum 禁止 |
| NR-10 | CP→ギフトカード / Tremendous | **OUT / DONE** | ADR-052。`convert-points` 済 |
| NR-11 | 公式素体コールドスタート | **HUMAN + Phase E** | `.vrm` 実体なし |
| NR-12 | BiomeBackground 目視 | **DONE** | OP-002 |
| NR-13 | 他 TLA の CI 未配線 | **OUT OF PLAN** | 別 OP |
| NR-14 | `/commerce/withdraw` alias | **DONE** | Wave 0c ✅ 2026-07-25（sunset 前倒し） |

---

## 4. 目視検証・その他

* **BiomeBackground + alpha:false** — **DONE**（OP-002 / 2026-07-13）
