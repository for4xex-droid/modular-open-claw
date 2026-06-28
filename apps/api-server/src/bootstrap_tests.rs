/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#[cfg(test)]
mod tests {
    use crate::bootstrap;
    use infrastructure::html_report::HtmlReportBuilder;

    #[tokio::test]
    async fn test_app_state_provides_html_report_builder() {
        // 1. ボートシーケンスを（一部モックして）実行
        // 2. AppState を取得
        // 3. AppState.html_report_builder() が有効なレポートを生成できるか確認
        
        // 現時点ではコンパイルエラーになるため、これが RED ステップです。
    }
}
