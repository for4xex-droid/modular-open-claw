# コーディング規約

言語別のベストプラクティスとコーディング規約をまとめます。

## TypeScript

### 型定義

```typescript
// 明示的な型定義
interface User {
  id: string;
  name: string;
  email: string;
  createdAt: Date;
}

// ユニオン型による網羅的なケース処理
type Status = 'pending' | 'active' | 'completed';

// 型ガード
function isUser(obj: unknown): obj is User {
  return typeof obj === 'object' && obj !== null && 'id' in obj;
}
```

### any の回避

```typescript
// ❌ any
function process(data: any) { }

// ✅ unknown + 型ガード
function process(data: unknown) {
  if (typeof data === 'string') {
    // data は string
  }
}

// ✅ ジェネリクス
function process<T>(data: T): T { }
```

### Null チェック

```typescript
// オプショナルチェーン
const name = user?.profile?.name;

// Nullish coalescing
const value = input ?? 'default';

// 型ガードによる早期リターン
if (!user) return null;
```

## Rust

### エラーハンドリング

```rust
// Result型の活用
fn read_file(path: &str) -> Result<String, io::Error> {
    fs::read_to_string(path)
}

// ?演算子でエラー伝播
fn process() -> Result<(), Error> {
    let content = read_file("data.txt")?;
    Ok(())
}

// カスタムエラー型
#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Validation failed: {0}")]
    Validation(String),
}
```

### 所有権

```rust
// 借用で不要なクローンを避ける
fn process(data: &str) { }

// 可変借用は1つだけ
fn update(data: &mut Vec<i32>) {
    data.push(42);
}

// ライフタイム明示
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}
```

### イディオム

```rust
// イテレータ活用
let sum: i32 = numbers.iter().filter(|&x| x > &0).sum();

// パターンマッチング
match result {
    Ok(value) => println!("{}", value),
    Err(e) => eprintln!("Error: {}", e),
}

// Option の処理
let value = option.unwrap_or_default();
let value = option.map(|x| x * 2);
```

## Python

### 型ヒント

```python
from typing import Optional, List, Dict

def process_users(users: List[Dict[str, str]]) -> Optional[str]:
    if not users:
        return None
    return users[0].get('name')
```

### リスト内包表記

```python
# ✅ Good
squares = [x**2 for x in range(10) if x % 2 == 0]

# ❌ Bad (複雑すぎ)
result = [f(x, y) for x in range(10) for y in range(10) if x != y and g(x, y)]
```

### コンテキストマネージャ

```python
# ファイル操作
with open('file.txt', 'r') as f:
    content = f.read()

# カスタムコンテキストマネージャ
@contextmanager
def timer():
    start = time.time()
    yield
    print(f"Elapsed: {time.time() - start:.2f}s")
```

## Go

### エラーハンドリング

```go
// 明示的なエラーチェック
result, err := doSomething()
if err != nil {
    return fmt.Errorf("failed to do something: %w", err)
}

// カスタムエラー
type ValidationError struct {
    Field   string
    Message string
}

func (e *ValidationError) Error() string {
    return fmt.Sprintf("%s: %s", e.Field, e.Message)
}
```

### 構造体とインターフェース

```go
// インターフェースは小さく
type Reader interface {
    Read(p []byte) (n int, err error)
}

// 構造体にはメソッドを持たせる
type User struct {
    ID   string
    Name string
}

func (u *User) FullName() string {
    return u.Name
}
```

## 共通ルール

### ドキュメント

```typescript
/**
 * ユーザーをIDで取得する
 * 
 * @param id - ユーザーID
 * @returns ユーザーオブジェクト、見つからない場合はnull
 * @throws {DatabaseError} データベース接続エラー時
 */
async function getUser(id: string): Promise<User | null> { }
```

### テスト可能性

- 依存性注入を活用
- 純粋関数を優先
- 副作用を分離
