pub async fn test() {
    // 実際には sqlx::query!() はコンパイル時にマクロ展開される
    let _ = sqlx::query("SELECT 1"); // 関数としての呼び出し
}
