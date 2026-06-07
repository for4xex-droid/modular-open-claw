import { LegalLayout } from './LegalLayout';
import { useTranslation } from 'react-i18next';

// -------------------------------------------------------------
// Privacy Policy Page
// -------------------------------------------------------------
export function PrivacyPage() {
  const { t } = useTranslation();
  
  return (
    <LegalLayout title={t('footer.privacy', 'Privacy Policy')} lastUpdated="2026-04-12">
      <section className="space-y-4">
        <p className="text-gray-400">
          本プライバシーポリシー（以下「本ポリシー」）は、セルフホスト型 AI オペレーティングシステム「Aiome」（以下「本ソフトウェア」）が取り扱うデータの性質、および利用者が本ソフトウェアを稼働させる際（以下「運営者」または「ユーザー」）のデータ保護方針について定めます。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">1. 原則（The Principle of Local-First）</h2>
        <p>
          Aiome は、<strong>プライバシー・バイ・デザイン（Privacy by Design）</strong> および <strong>ローカルファースト（Local-First）</strong> の設計原則に基づいて構築されています。
          本ソフトウェアによって生成、収集、または保存されるデータは、原則として<strong>本ソフトウェアが稼働しているインフラ環境内（ローカルホストまたは所有者のサーバー）で完結し、開発者（モチベーションスタジオ LLC）へは一切送信されません。</strong>
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">2. 収集されるデータの種類と保存場所</h2>
        <p>
          本ソフトウェアは以下のデータをシステム内でローカルに保存（SQLite）します。
        </p>
        <ul className="list-disc pl-6 space-y-2">
          <li><strong>アカウント情報</strong>: エージェントID、認証キー</li>
          <li><strong>通信記録（チャットログ）</strong>: ユーザーと AI エンティティ（Samsara / Cortex）とのチャット履歴、思考プロセスログ</li>
          <li><strong>アセットデータ</strong>: アップロードされた音声バインダー (.aivoice), アバターモデル (.inx), ドキュメント知識</li>
          <li><strong>監査ログ・システムのヘルスデータ</strong>: エラーログ、コスト消費履歴</li>
        </ul>
        <p>
          これらのデータはすべて、運営者が指定するローカルの永続化ボリューム（通常 `./data/` 配下）に保存されます。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">3. 外部サービスとの通信におけるデータの取り扱い</h2>
        <p>
          Aiome は、LLM 推論や外部リソースの取得のため、運営者が設定した場合にのみ外部 API（Claude, OpenAI, Gemini, Stripe, Samsara Hub 等）と通信します。
          運営者は、機密情報（個人情報 PII、パスワードなど）を含むデータをプロンプトに含めないよう、システム側のマスキング設定等を通じて適宜コントロールする責任を有します。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">4. データの削除（忘れられる権利 / GDPR準拠）</h2>
        <p>
          Aiome は、全データがデータベース内の単一または特定のテーブルに関連付けられているため、環境の初期化（`FactoryReset`）または該当リソースの削除 API の実行を通じて、<strong>利用者が自らの意志でサーバー内からデータを完全に物理削除（CASCADE DELETE またはディレクトリ破棄）することが可能です（忘れられる権利）</strong>。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">5. 免責事項</h2>
        <p>
          モチベーションスタジオ LLC（Motivation Studio LLC）は、<strong>運営者が管理する本ソフトウェアのインスタンス内で処理・保存されたデータの漏洩、消失、改ざんなどのインシデントについて、一切の免責（責任を負わないこと）</strong>とします。
        </p>
      </section>
    </LegalLayout>
  );
}

// -------------------------------------------------------------
// Terms of Service Page
// -------------------------------------------------------------
export function TermsPage() {
  const { t } = useTranslation();

  return (
    <LegalLayout title={t('footer.terms', 'Terms of Service')} lastUpdated="2026-05-31">
      <section className="space-y-4">
        <p className="text-gray-400">
          本利用規約（以下「本規約」）は、モチベーションスタジオ LLC（以下「当社」）が提供、または運営するセルフホスト型 AI オペレーティングシステム「Aiome」（以下「本ソフトウェア」）の利用条件を定めるものです。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">1. 早期アクセス免責</h2>
        <p>
          本ソフトウェアは現在、早期アクセスおよびプレリリース段階として提供されています。当社は、動作の完全性、安定性、無謬性、セキュリティについて一切の保証を行わず、「現状有姿（AS IS）」で提供します。予期しないシステム破損やデータ消失が発生する可能性があることを理解し、自己責任で稼働させるものとします。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">2. ライセンスおよび商用利用許諾 (BSL-1.1)</h2>
        <p>
          本ソフトウェアは、<strong>Business Source License 1.1 (BSL-1.1)</strong> に基づきデュアルライセンス供与されています。
          非商用目的、個人的な検証目的、または BSL-1.1 の許諾範囲内での利用は無料です。特定の商用環境での運用については、商用利用ライセンスの取得が必要です。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">3. コマース経済圏および Karma Coins (KC) 決済規約</h2>
        <ul className="list-disc pl-6 space-y-2">
          <li><strong>返金不可の方針</strong>: <strong>Stripe 決済を介して購入された Karma Coins (KC) のチャージ代金、および月額サブスクリプション料金については、理由の如何を問わず一切の返金を行いません（返金不可ポリシー）。</strong></li>
          <li><strong>プラットフォーム手数料</strong>: アプリケーション内での取引において、手数料として取引額の 15% が控除され、残る 85% がクリエイターに分配されます。</li>
          <li><strong>eKYC（本人確認）の義務化</strong>: 出金や有償サービス提供にあたっては、Stripe Identity などを通じた本人確認（eKYC）の完了を必須条件とします。</li>
        </ul>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">4. 自己修復 (Self-Healing) および AI 自律実行に伴う完全免責</h2>
        <p>
          本ソフトウェアは、AI によるコードの自動生成、デバッグ、自己修復（Self-Healing）、および外部 API への自動的な接続要求やタスクの自律的実行を行います。
          当社は、本ソフトウェアの自律的な動作（自律的なコード書き換えを含む）に起因して発生した、稼働サーバー内のデータ消失、想定外の API 課金コストの発生、第三者の知的財産権の侵害について<strong>一切の免責（法的責任および金銭的賠償責任を負わないこと）</strong>とします。
        </p>
      </section>
    </LegalLayout>
  );
}

// -------------------------------------------------------------
// Act on Specified Commercial Transactions Page (特定商取引法)
// -------------------------------------------------------------
export function TokushohoPage() {
  const { t } = useTranslation();

  return (
    <LegalLayout title={t('footer.tokushoho', '特定商取引法に基づく表記')} lastUpdated="2026-06-07">
      <table className="w-full text-left border-collapse border border-white/10 rounded-2xl overflow-hidden bg-brand-surface/20">
        <tbody>
          <tr className="border-b border-white/5">
            <th className="p-4 w-1/3 bg-white/5 text-white font-semibold">特定商取引法に基づく表記</th>
            <td className="p-4">以下の通り開示いたします。</td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">販売業者</th>
            <td className="p-4">motivationstudio, LLC（モチベーションスタジオ合同会社）</td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">運営責任者</th>
            <td className="p-4">モチベーションスタジオ合同会社 代表社員</td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">所在地</th>
            <td className="p-4">京都府京都市（※詳細な住所はご請求に応じて遅滞なく電子メール等で提供いたします）</td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">メールアドレス</th>
            <td className="p-4">project.aiome@gmail.com（問い合わせ先メールアドレス）</td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">販売価格</th>
            <td className="p-4">商品購入ページ（Karma Coins 購入画面など）に表示される対価（販売価格）をご確認ください。</td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">支払方法</th>
            <td className="p-4">クレジットカード決済（Stripe Commerce Engine 経由）によるお支払方法（支払方法）となります。</td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">引渡時期</th>
            <td className="p-4">お支払方法による決済完了後、即時（システムの遅滞なき稼働による Karma Coins の自動付与）に商品の引き渡しを行います（引渡時期）。</td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">返品・キャンセル</th>
            <td className="p-4">デジタルコンテンツおよびバーチャル通貨（Karma Coins）の性質上、購入手続き完了後の返品・キャンセル、返金には応じられません（返品ポリシー）。</td>
          </tr>
        </tbody>
      </table>
    </LegalLayout>
  );
}
