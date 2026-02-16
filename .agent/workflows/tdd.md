---
description: テスト駆動開発を実行。テストを先に書き、実装、リファクタリングの順で進める。
---

# /tdd - テスト駆動開発コマンド

Red-Green-Refactorサイクルでテスト駆動開発を実行します。

## TDDサイクル

```
┌─────────────────────────────────────────┐
│  1. RED: 失敗するテストを書く           │
│     ↓                                   │
│  2. GREEN: テストを通す最小限のコード   │
│     ↓                                   │
│  3. REFACTOR: コードを改善              │
│     ↓                                   │
│  (繰り返し)                            │
└─────────────────────────────────────────┘
```

## 実行手順

### ステップ1: 要件の理解
1. 実装する機能を明確に理解
2. 期待される入力と出力を特定
3. エッジケースを列挙

### ステップ2: テストファイル作成
```typescript
// example.test.ts
describe('機能名', () => {
  it('正常系: 期待される動作', () => {
    // Arrange
    const input = '入力値';
    
    // Act
    const result = functionUnderTest(input);
    
    // Assert
    expect(result).toBe('期待値');
  });

  it('異常系: エラーハンドリング', () => {
    // 無効な入力でエラーをスロー
    expect(() => functionUnderTest(null)).toThrow();
  });

  it('エッジケース: 空入力', () => {
    const result = functionUnderTest('');
    expect(result).toBe('');
  });
});
```

### ステップ3: テスト実行（RED）
// turbo
```powershell
npm test -- --watch
```

テストが失敗することを確認。

### ステップ4: 最小限の実装（GREEN）
テストを通す最小限のコードを書く。
- 余計な機能を追加しない
- まずテストを通すことに集中

### ステップ5: テスト再実行
// turbo
```powershell
npm test
```

すべてのテストがパスすることを確認。

### ステップ6: リファクタリング（REFACTOR）
- コードの重複を除去
- 命名を改善
- パフォーマンスを最適化
- テストは常にグリーンを維持

## カバレッジ要件

**目標: 80%以上のカバレッジ**

カバレッジ確認コマンド：
// turbo
```powershell
npm test -- --coverage
```

## テストのベストプラクティス

### 1. AAAパターン
- **Arrange**: テストデータの準備
- **Act**: テスト対象の実行
- **Assert**: 結果の検証

### 2. 1テスト1アサーション
```typescript
// ✅ Good
it('should return user name', () => {
  expect(user.getName()).toBe('John');
});

it('should return user age', () => {
  expect(user.getAge()).toBe(30);
});

// ❌ Bad
it('should return user data', () => {
  expect(user.getName()).toBe('John');
  expect(user.getAge()).toBe(30);
  expect(user.getEmail()).toBe('john@example.com');
});
```

### 3. 意味のあるテスト名
```typescript
// ✅ Good
it('should throw ValidationError when email is invalid', () => {});

// ❌ Bad
it('test1', () => {});
```

### 4. モックの適切な使用
```typescript
// 外部依存のモック化
jest.mock('./database', () => ({
  query: jest.fn().mockResolvedValue([{ id: 1 }])
}));
```

## テストの種類

| 種類 | 目的 | カバレッジ目標 |
|------|------|----------------|
| ユニット | 個別関数/クラス | 80%+ |
| 統合 | コンポーネント連携 | 60%+ |
| E2E | ユーザーフロー | クリティカルパスのみ |

## 関連ワークフロー

- `/plan` - 実装計画
- `/test-coverage` - カバレッジ分析
- `/code-review` - コードレビュー
