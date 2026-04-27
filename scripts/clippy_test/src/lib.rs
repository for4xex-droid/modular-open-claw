pub async fn test() {
    // 実際には sqlx::query!() はコンパイル時にマクロ展開される
    let _ = sqlx::query::<sqlx::Sqlite>("SELECT 1"); // 関数としての呼び出し
}
