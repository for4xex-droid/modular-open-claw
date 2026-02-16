# バックエンドパターン

## API設計

### RESTful API

```
GET    /users          # 一覧取得
GET    /users/:id      # 詳細取得
POST   /users          # 作成
PUT    /users/:id      # 更新（全体）
PATCH  /users/:id      # 更新（部分）
DELETE /users/:id      # 削除
```

### レスポンス形式

```typescript
// 成功レスポンス
interface SuccessResponse<T> {
  success: true;
  data: T;
  meta?: {
    page: number;
    total: number;
    limit: number;
  };
}

// エラーレスポンス
interface ErrorResponse {
  success: false;
  error: {
    code: string;
    message: string;
    details?: Record<string, string>;
  };
}
```

### HTTPステータスコード

| コード | 用途 |
|--------|------|
| 200 | 成功 |
| 201 | 作成成功 |
| 204 | 成功（レスポンスボディなし） |
| 400 | 不正なリクエスト |
| 401 | 認証エラー |
| 403 | 認可エラー |
| 404 | リソースなし |
| 422 | バリデーションエラー |
| 500 | サーバーエラー |

## データベースパターン

### Repository パターン

```typescript
interface UserRepository {
  findById(id: string): Promise<User | null>;
  findAll(filter?: UserFilter): Promise<User[]>;
  create(data: CreateUserInput): Promise<User>;
  update(id: string, data: UpdateUserInput): Promise<User>;
  delete(id: string): Promise<void>;
}

class PostgresUserRepository implements UserRepository {
  constructor(private db: Database) {}
  
  async findById(id: string): Promise<User | null> {
    return this.db.query('SELECT * FROM users WHERE id = $1', [id]);
  }
}
```

### トランザクション

```typescript
async function transferMoney(fromId: string, toId: string, amount: number) {
  await db.transaction(async (tx) => {
    await tx.query('UPDATE accounts SET balance = balance - $1 WHERE id = $2', [amount, fromId]);
    await tx.query('UPDATE accounts SET balance = balance + $1 WHERE id = $2', [amount, toId]);
  });
}
```

### マイグレーション

```sql
-- migrations/001_create_users.sql
CREATE TABLE users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  email VARCHAR(255) UNIQUE NOT NULL,
  name VARCHAR(255) NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_users_email ON users(email);
```

## キャッシュ戦略

### キャッシュパターン

```typescript
async function getUserWithCache(id: string): Promise<User | null> {
  // キャッシュ確認
  const cached = await cache.get(`user:${id}`);
  if (cached) return JSON.parse(cached);
  
  // DBから取得
  const user = await db.users.findById(id);
  if (!user) return null;
  
  // キャッシュに保存（TTL: 1時間）
  await cache.set(`user:${id}`, JSON.stringify(user), 'EX', 3600);
  
  return user;
}
```

### キャッシュ無効化

```typescript
async function updateUser(id: string, data: UpdateUserInput): Promise<User> {
  const user = await db.users.update(id, data);
  
  // キャッシュを無効化
  await cache.del(`user:${id}`);
  
  return user;
}
```

## 認証・認可

### JWT認証

```typescript
// トークン生成
function generateToken(user: User): string {
  return jwt.sign(
    { sub: user.id, email: user.email },
    process.env.JWT_SECRET!,
    { expiresIn: '24h' }
  );
}

// トークン検証ミドルウェア
async function authMiddleware(req: Request, res: Response, next: NextFunction) {
  const token = req.headers.authorization?.replace('Bearer ', '');
  if (!token) {
    return res.status(401).json({ error: 'Unauthorized' });
  }
  
  try {
    const decoded = jwt.verify(token, process.env.JWT_SECRET!);
    req.user = decoded;
    next();
  } catch {
    return res.status(401).json({ error: 'Invalid token' });
  }
}
```

### RBAC（ロールベースアクセス制御）

```typescript
const permissions = {
  admin: ['read', 'write', 'delete', 'manage'],
  editor: ['read', 'write'],
  viewer: ['read']
};

function checkPermission(user: User, required: string): boolean {
  return permissions[user.role]?.includes(required) ?? false;
}
```

## エラーハンドリング

### カスタムエラークラス

```typescript
class AppError extends Error {
  constructor(
    message: string,
    public code: string,
    public statusCode: number = 500
  ) {
    super(message);
  }
}

class NotFoundError extends AppError {
  constructor(resource: string) {
    super(`${resource} not found`, 'NOT_FOUND', 404);
  }
}

class ValidationError extends AppError {
  constructor(
    message: string,
    public details: Record<string, string>
  ) {
    super(message, 'VALIDATION_ERROR', 422);
  }
}
```

### グローバルエラーハンドラ

```typescript
function errorHandler(err: Error, req: Request, res: Response, next: NextFunction) {
  console.error('Error:', err);
  
  if (err instanceof AppError) {
    return res.status(err.statusCode).json({
      success: false,
      error: {
        code: err.code,
        message: err.message
      }
    });
  }
  
  // 予期しないエラー
  res.status(500).json({
    success: false,
    error: {
      code: 'INTERNAL_ERROR',
      message: 'An unexpected error occurred'
    }
  });
}
```

## ロギング

```typescript
import pino from 'pino';

const logger = pino({
  level: process.env.LOG_LEVEL || 'info',
  transport: {
    target: 'pino-pretty',
    options: { colorize: true }
  }
});

// 使用例
logger.info({ userId: user.id }, 'User created');
logger.error({ err, requestId }, 'Request failed');
```
