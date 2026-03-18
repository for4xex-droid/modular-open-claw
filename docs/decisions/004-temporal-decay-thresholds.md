# ADR-004: Temporal Decay 閾値の設計（RS-3）

**Status**: Accepted  
**Date**: 2026-03-19  
**Deciders**: motivationstudio

## Context

Soul Engine は経験を蓄積し続けるが、古い記憶や防衛が永遠に残ると：
- メモリが肥大化する
- 過去の弱い防衛が Rebirth 時に次世代に無駄に継承される
- 感情マーカーが飽和して直感バイアスが不安定になる

## Decision

3つの閾値を設定:

| パラメータ | 値 | 意味 |
|---|---|---|
| `decay_rate` | 0.995 | 毎 Experience 処理時に intensity を 0.5% 減衰 |
| `death_threshold` | 0.2 | この閾値を下回った Defense/SomaticMarker は永久削除 |
| `rebirth_inheritance` | 0.5 | Rebirth 時にこの閾値以上の Defense のみ次世代に継承 |

### 数値の根拠

- **0.995**: 約140回の Experience で intensity が半減（十分な記憶保持期間）
- **0.2**: 元の強度の20%まで下がった記憶は「忘れてよい」
- **0.5**: 0.7（旧値）では強い防衛しか残らず、中程度の学びが失われていた。0.5 に下げることで「それなりに重要」な防衛も次世代に継承

## Consequences

- **Good**: 無限 Rebirth サイクルでもメモリが飽和しない
- **Good**: 0.2 と 0.5 の間の「忘却ゾーン」が自然な記憶の消失を再現
- **Risk**: 短期間に大量の Experience があると重要な防衛も減衰しすぎる可能性
