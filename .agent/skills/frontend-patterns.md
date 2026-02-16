# フロントエンドパターン

## React パターン

### コンポーネント設計

```tsx
// ✅ 関数コンポーネント
interface UserCardProps {
  user: User;
  onEdit?: (id: string) => void;
}

export function UserCard({ user, onEdit }: UserCardProps) {
  return (
    <div className="user-card">
      <h3>{user.name}</h3>
      <p>{user.email}</p>
      {onEdit && (
        <button onClick={() => onEdit(user.id)}>Edit</button>
      )}
    </div>
  );
}
```

### カスタムフック

```tsx
// データフェッチフック
function useUser(id: string) {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    async function fetchUser() {
      try {
        setLoading(true);
        const data = await api.getUser(id);
        setUser(data);
      } catch (e) {
        setError(e as Error);
      } finally {
        setLoading(false);
      }
    }
    fetchUser();
  }, [id]);

  return { user, loading, error };
}

// 使用例
function UserProfile({ id }: { id: string }) {
  const { user, loading, error } = useUser(id);
  
  if (loading) return <Spinner />;
  if (error) return <ErrorMessage error={error} />;
  if (!user) return <NotFound />;
  
  return <UserCard user={user} />;
}
```

### 状態管理

```tsx
// Context + useReducer
interface AppState {
  user: User | null;
  theme: 'light' | 'dark';
}

type Action =
  | { type: 'SET_USER'; payload: User }
  | { type: 'LOGOUT' }
  | { type: 'TOGGLE_THEME' };

function appReducer(state: AppState, action: Action): AppState {
  switch (action.type) {
    case 'SET_USER':
      return { ...state, user: action.payload };
    case 'LOGOUT':
      return { ...state, user: null };
    case 'TOGGLE_THEME':
      return { ...state, theme: state.theme === 'light' ? 'dark' : 'light' };
    default:
      return state;
  }
}

const AppContext = createContext<{
  state: AppState;
  dispatch: React.Dispatch<Action>;
} | null>(null);

export function AppProvider({ children }: { children: React.ReactNode }) {
  const [state, dispatch] = useReducer(appReducer, initialState);
  return (
    <AppContext.Provider value={{ state, dispatch }}>
      {children}
    </AppContext.Provider>
  );
}
```

## Next.js パターン

### App Router

```tsx
// app/users/[id]/page.tsx
export default async function UserPage({ params }: { params: { id: string } }) {
  const user = await getUser(params.id);
  
  if (!user) {
    notFound();
  }
  
  return <UserProfile user={user} />;
}

// メタデータ
export async function generateMetadata({ params }: { params: { id: string } }) {
  const user = await getUser(params.id);
  return {
    title: user?.name ?? 'User Not Found',
    description: `Profile of ${user?.name}`
  };
}
```

### Server Actions

```tsx
// app/actions.ts
'use server';

import { revalidatePath } from 'next/cache';

export async function createUser(formData: FormData) {
  const name = formData.get('name') as string;
  const email = formData.get('email') as string;
  
  await db.users.create({ name, email });
  
  revalidatePath('/users');
}

// 使用例
export function CreateUserForm() {
  return (
    <form action={createUser}>
      <input name="name" required />
      <input name="email" type="email" required />
      <button type="submit">Create</button>
    </form>
  );
}
```

### データフェッチ

```tsx
// キャッシュ戦略
// 静的（ビルド時に取得）
const data = await fetch('https://api.example.com/data');

// 動的（リクエスト毎）
const data = await fetch('https://api.example.com/data', { cache: 'no-store' });

// 再検証（60秒毎）
const data = await fetch('https://api.example.com/data', { next: { revalidate: 60 } });
```

## フォーム処理

### React Hook Form + Zod

```tsx
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';

const schema = z.object({
  email: z.string().email('Invalid email'),
  password: z.string().min(8, 'Password must be at least 8 characters'),
});

type FormData = z.infer<typeof schema>;

export function LoginForm() {
  const { register, handleSubmit, formState: { errors } } = useForm<FormData>({
    resolver: zodResolver(schema),
  });

  const onSubmit = (data: FormData) => {
    console.log(data);
  };

  return (
    <form onSubmit={handleSubmit(onSubmit)}>
      <input {...register('email')} placeholder="Email" />
      {errors.email && <span>{errors.email.message}</span>}
      
      <input {...register('password')} type="password" placeholder="Password" />
      {errors.password && <span>{errors.password.message}</span>}
      
      <button type="submit">Login</button>
    </form>
  );
}
```

## スタイリング

### CSS Modules

```tsx
// UserCard.module.css
.card {
  padding: 1rem;
  border-radius: 8px;
  background: var(--bg-secondary);
}

.title {
  font-size: 1.25rem;
  font-weight: 600;
}

// UserCard.tsx
import styles from './UserCard.module.css';

export function UserCard({ user }: { user: User }) {
  return (
    <div className={styles.card}>
      <h3 className={styles.title}>{user.name}</h3>
    </div>
  );
}
```

### CSS変数によるテーマ

```css
:root {
  --color-primary: #3b82f6;
  --color-secondary: #64748b;
  --bg-primary: #ffffff;
  --bg-secondary: #f8fafc;
  --text-primary: #1e293b;
  --radius: 8px;
}

[data-theme="dark"] {
  --bg-primary: #0f172a;
  --bg-secondary: #1e293b;
  --text-primary: #f8fafc;
}
```

## アクセシビリティ

```tsx
// 適切なセマンティクス
<button onClick={handleClick}>Submit</button>  // ✅
<div onClick={handleClick}>Submit</div>  // ❌

// ラベル
<label htmlFor="email">Email</label>
<input id="email" type="email" aria-describedby="email-hint" />
<p id="email-hint">We'll never share your email.</p>

// キーボードナビゲーション
<button onKeyDown={(e) => e.key === 'Enter' && handleClick()}>
  Submit
</button>
```
