/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { LegalLayout } from './LegalLayout';
import { useTranslation } from 'react-i18next';

// -------------------------------------------------------------
// Privacy Policy Page
// -------------------------------------------------------------
export function PrivacyPage() {
  const { t } = useTranslation();

  return (
    <LegalLayout title={t('footer.privacy', 'Privacy Policy')} lastUpdated="2026-07-14">
      <section className="space-y-4">
        <p className="text-gray-400">
          本プライバシーポリシー（以下「本ポリシー」）は、セルフホスト型 AI オペレーティングシステム「Aiome」（以下「本ソフトウェア」）が取り扱うデータの性質、本ソフトウェアを稼働させる利用者（以下「運営者」または「ユーザー」）のデータ保護方針、およびモチベーションスタジオ合同会社（以下「当社」）が有償プランの提供に伴い取得する情報の取り扱いについて定めます。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">1. 原則（The Principle of Local-First）</h2>
        <p>
          Aiome は、<strong>プライバシー・バイ・デザイン（Privacy by Design）</strong> および <strong>ローカルファースト（Local-First）</strong> の設計原則に基づいて構築されています。
          本ソフトウェアによって生成、収集、または保存されるデータ（チャットログ・アセット・監査ログ等）は、原則として<strong>本ソフトウェアが稼働しているインフラ環境内（ローカルホストまたは所有者のサーバー）で完結し、当社へは送信されません。</strong> 当社が取得する情報は、第 4 条に定める有償プランの契約管理に必要な情報に限られます。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">2. 本ソフトウェアがローカルに保存するデータ</h2>
        <p>本ソフトウェアは以下のデータをシステム内でローカルに保存（SQLite 等）します。</p>
        <ul className="list-disc pl-6 space-y-2">
          <li><strong>アカウント情報</strong>: エージェントID、認証キー、管理者メールアドレス、規約同意記録（同意した版・日時）</li>
          <li><strong>通信記録（チャットログ）</strong>: ユーザーと AI エンティティ（Samsara / Cortex）とのチャット履歴、思考プロセスログ (<code>trajectory_store</code> 等)</li>
          <li><strong>アセットデータ</strong>: アップロードされた音声バインダー (.aivoice), アバターモデル (.inx), ドキュメント知識</li>
          <li><strong>監査ログ・システムのヘルスデータ</strong>: エラーログ、コスト消費履歴 (<code>resource_usage_logs</code>)</li>
        </ul>
        <p>
          これらのデータはすべて、運営者が指定するローカルの永続化ボリューム（通常 <code>./data/</code> 配下）に保存されます。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">3. 外部サービスとの通信におけるデータの取り扱い</h2>
        <p>
          Aiome は、LLM 推論や外部リソースの取得のため、運営者が設定した場合にのみ以下の外部 API と通信します。
        </p>
        <ul className="list-disc pl-6 space-y-2">
          <li>
            <strong>LLM プロバイダー (Claude, OpenAI, Gemini, Fal.ai 等)</strong>:
            <ul className="list-disc pl-6 mt-2 space-y-1">
              <li>送信内容: 推論に必要なプロンプト、システムプロンプト、および必要な対話の文脈（チャット履歴）。</li>
              <li>取り扱い: 各プロバイダーの利用規約に依存します。本ソフトウェアを実行する運営者は、各プロバイダーに送信されるデータの内容に責任を持ちます。</li>
            </ul>
          </li>
          <li><strong>コマースおよび連携 (Stripe, Tremendous 等)</strong>: 決済情報やギフト送付処理のためのパラメーター通信。</li>
          <li><strong>P2P Federation (Samsara Hub)</strong>: 公開設定された情報のみが送信されます。</li>
        </ul>
        <p>
          運営者は、機密情報（個人情報 PII、パスワードなど）を含むデータをプロンプトに含めないよう、システム側のマスキング設定（<code>guardrails</code> の PII マスキングなど）等を通じて適宜コントロールする責任を有します。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">4. 当社が取得する情報（有償プラン契約者）</h2>
        <p>
          Pro プラン（有償サブスクリプション）をご契約いただく場合、当社は決済代行事業者 <strong>Stripe, Inc.</strong> を通じて、以下の情報を取得・保有します。
        </p>
        <table className="w-full text-left border-collapse border border-white/10 rounded-2xl overflow-hidden bg-brand-surface/20">
          <thead>
            <tr className="border-b border-white/5">
              <th className="p-4 bg-white/5 text-white font-semibold">情報</th>
              <th className="p-4 bg-white/5 text-white font-semibold">内容</th>
              <th className="p-4 bg-white/5 text-white font-semibold">備考</th>
            </tr>
          </thead>
          <tbody>
            <tr className="border-b border-white/5">
              <td className="p-4">決済関連情報</td>
              <td className="p-4">メールアドレス、カードブランド・下 4 桁・有効期限、決済履歴、Stripe 顧客 ID</td>
              <td className="p-4"><strong>クレジットカード番号そのものは当社に送信されず、当社は保持しません</strong>（Stripe が PCI DSS に準拠して処理します）</td>
            </tr>
            <tr className="border-b border-white/5">
              <td className="p-4">契約管理情報</td>
              <td className="p-4">サブスクリプションの契約状態（有効・停止・解約）</td>
              <td className="p-4">Pro 機能の有効化/停止の自動制御に使用</td>
            </tr>
            <tr>
              <td className="p-4">お問い合わせ情報</td>
              <td className="p-4">メールでのお問い合わせに含まれる氏名・メールアドレス・内容</td>
              <td className="p-4">サポート対応に使用</td>
            </tr>
          </tbody>
        </table>
        <p>
          <strong>利用目的</strong>: (1) 料金の請求・決済および契約の管理、(2) Pro 機能の有効化・停止の制御、(3) お問い合わせへの対応、(4) 法令に基づく帳簿等の保存義務の履行。
        </p>
        <p>
          <strong>第三者提供・委託</strong>: 当社は、決済処理を Stripe, Inc.（米国）に委託しています。Stripe のプライバシーポリシーは{' '}
          <a href="https://stripe.com/privacy" target="_blank" rel="noopener noreferrer" className="text-brand-cyan hover:underline">https://stripe.com/privacy</a>{' '}
          をご確認ください。当社は、法令に基づく場合を除き、取得した情報を第三者に提供しません。
        </p>
        <p>
          <strong>保存期間</strong>: 決済関連情報は、法令上の帳簿保存義務（税法上原則 7 年間）に従って保存し、期間経過後に削除します。
        </p>
        <p>
          <strong>開示・訂正・削除等の請求</strong>: ご本人からの保有個人データの開示・訂正・利用停止等のご請求は <code>project.aiome@gmail.com</code> にて受け付けます。ご本人確認の上、法令に従い遅滞なく対応します。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">5. Cookie・アクセス解析</h2>
        <p>
          公式サイト（aiome.dev）は、現時点でトラッキング Cookie を使用していません。将来アクセス解析（Plausible 等の Cookie レス解析を予定）を導入する場合は、本ポリシーを改定し周知します。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">6. データの削除（忘れられる権利に対応した設計）</h2>
        <p>
          Aiome は、全データがデータベース内の単一または特定のテーブルに関連付けられているため、環境の初期化（<code>FactoryReset</code>）または該当リソースの削除 API の実行を通じて、<strong>利用者が自らの意志でサーバー内からデータを完全に物理削除（CASCADE DELETE またはディレクトリ破棄）することが可能な設計</strong>となっています。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">7. 個人特定情報 (PII) のマスキング方針</h2>
        <p>
          本ソフトウェアは、アプリケーションのログ（<code>tracing</code> イベント経由）において、クレジットカード番号等をマスクするフィルター機能を出荷時に実装・提供していますが、すべての PII の漏洩を防ぐことを保証するものではありません。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">8. 免責事項</h2>
        <p>
          当社は、<strong>運営者が管理する本ソフトウェアのインスタンス内</strong>で処理・保存されたデータの漏洩、消失、改ざんなどのインシデントについて、当社の故意または重過失による場合を除き、責任を負いません。バックアップ・リストア運用、データベースアクセスへの適切なアクセス権限の設定は、運営者の責任において実施してください。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">9. 変更について</h2>
        <p>
          本ポリシーは、システムの機能追加等に伴い改定される場合があります。重要な変更を行う場合は、公式サイトまたはアプリケーション内で周知します。最新版はリポジトリまたは公式サイトにてご確認ください。
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
    <LegalLayout title={t('footer.terms', 'Terms of Service')} lastUpdated="2026-07-15">
      <section className="space-y-4">
        <p className="text-gray-400">
          本利用規約（以下「本規約」）は、モチベーションスタジオ合同会社（motivationstudio, LLC。以下「当社」）が提供するセルフホスト型 AI オペレーティングシステム「Aiome」（以下「本ソフトウェア」）および当社が有償で提供するサブスクリプションサービス（以下「Pro プラン」）の利用条件を定めるものです。本ソフトウェアの利用者（以下「ユーザー」）は、本規約に同意した上で本ソフトウェアを利用するものとします。
        </p>
        <p className="text-sm text-gray-500">
          <strong>適用開始日</strong>: 2026-07-15（本版の公開日）
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">1. 早期アクセス (First Penguin Edition) に関する留意事項</h2>
        <p>
          本ソフトウェアは現在、早期アクセスおよびプレリリース段階（First Penguin Edition）として提供されています。当社は、本ソフトウェアを「現状有姿（AS IS）」で提供し、その動作の完全性、安定性、無謬性、セキュリティについて、<strong>法令上許容される最大限の範囲で</strong>保証を行いません。ユーザーは、予期しないシステム破損やデータ消失が発生する可能性があることを理解し、重要なデータのバックアップ等の自衛措置を講じた上で、自己の責任において本ソフトウェアを稼働させるものとします。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">2. ライセンスおよび商用利用許諾 (BSL-1.1)</h2>
        <p>
          本ソフトウェアは、<strong>Business Source License 1.1 (BSL-1.1)</strong> に基づきデュアルライセンス供与されています。
        </p>
        <ul className="list-disc pl-6 space-y-2">
          <li>非商用目的、個人的な検証目的、または BSL-1.1 の許諾範囲内での利用は無料です。</li>
          <li>BSL-1.1 で制限されている本番環境および特定の商用環境での運用については、当社が別途発行する商用利用ライセンスの取得が必要です。詳細はライセンスファイル（<code>LICENSE</code>）をご参照ください。</li>
        </ul>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">3. Pro プラン（有償サブスクリプション）</h2>

        <h3 className="text-lg font-semibold text-white">3.1 内容と料金</h3>
        <ul className="list-disc pl-6 space-y-2">
          <li>Pro プランは、本ソフトウェアの追加機能（ストリーミング音声合成、LoRA 学習等。最新の提供機能一覧はアップグレード画面および公式サイトに表示）を月額制で提供するサービスです。</li>
          <li>料金は <strong>月額 19.99 米ドル（USD）</strong> です。日本円でのお支払額は、決済時の為替レートおよびカード会社の手数料により変動します。</li>
          <li>一部の Pro 機能は段階的に提供（順次解禁）されます。各機能の提供状況は購入手続き画面の表示を正とします。</li>
        </ul>

        <h3 className="text-lg font-semibold text-white">3.2 契約の成立と自動更新</h3>
        <ul className="list-disc pl-6 space-y-2">
          <li>Pro プランの利用契約は、当社の決済代行事業者（Stripe, Inc. 以下「Stripe」）の決済画面において決済手続が完了した時点で成立します。</li>
          <li>契約期間は 1 か月間とし、ユーザーが解約手続を行わない限り、<strong>同一条件で 1 か月ごとに自動更新され、更新日に翌期間分の料金が登録されたお支払方法へ自動的に請求されます。</strong></li>
        </ul>

        <h3 className="text-lg font-semibold text-white">3.3 無料トライアル</h3>
        <p>
          <strong>現時点では無料トライアルは提供していません。</strong> 将来当社が無料トライアルを設定する場合、その期間・条件は購入手続き画面に表示します。トライアル期間中に解約した場合、料金は発生しません。トライアル期間の満了後は、解約手続が行われない限り自動的に有償契約へ移行します。
        </p>

        <h3 className="text-lg font-semibold text-white">3.4 解約</h3>
        <ul className="list-disc pl-6 space-y-2">
          <li>ユーザーは、いつでもアプリケーション内の「お支払い管理」（Stripe カスタマーポータル）から解約手続を行うことができます。</li>
          <li>解約手続後も、<strong>支払済みの契約期間の末日までは Pro 機能を利用できます。</strong> 契約期間の途中での解約による日割り返金は行いません。</li>
          <li>
            解約手続の詳細は「
            <a href="/cancellation" className="text-brand-cyan hover:underline">解約・返金ポリシー</a>
            」に定めます。
          </li>
        </ul>

        <h3 className="text-lg font-semibold text-white">3.5 支払不履行</h3>
        <p>
          料金の決済が完了しなかった場合、当社は Pro 機能の提供を一時停止することがあります。決済が正常に完了した時点で、提供は自動的に再開されます。
        </p>

        <h3 className="text-lg font-semibold text-white">3.6 価格改定</h3>
        <p>
          当社は、料金を改定する場合、<strong>効力発生日の 30 日前まで</strong>に公式サイトまたはアプリケーション内で周知します。改定後の料金に同意しないユーザーは、効力発生日までに解約することができます。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">4. Karma Coins (KC) — アプリ内ポイント</h2>
        <ul className="list-disc pl-6 space-y-2">
          <li>Karma Coins（以下「KC」）は、本ソフトウェア内の機能を通じて<strong>無償で付与される</strong>アプリケーション内ポイントであり、<strong>当社は KC を有償で販売しません。</strong></li>
          <li>KC は、現金その他の財産的価値と交換することはできず、払い戻しの対象にもなりません。KC は資金決済に関する法律上の前払式支払手段に該当しません。</li>
          <li>当社が将来 KC の有償販売を開始する場合は、法令上必要な手続（資金決済法上の対応を含む）を完了した上で、本規約を改定し事前に周知します。</li>
        </ul>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">5. 返金</h2>
        <ul className="list-disc pl-6 space-y-2">
          <li>
            当社は、次の場合を除き、支払済みの利用料金の返金を行いません。
            <ol className="list-decimal pl-6 mt-2 space-y-1">
              <li>法令に基づき返金が義務付けられる場合</li>
              <li>当社の責めに帰すべき事由により本サービスが提供されなかった場合</li>
              <li>決済から 24 時間以内かつ当該課金周期において Pro 機能を実質的に未使用と当社が合理的に判断できる誤購入（善意対応。詳細は「解約・返金ポリシー」）</li>
            </ol>
          </li>
          <li>Pro プランは通信販売により提供されるデジタルサービスであり、特定商取引法上のクーリング・オフ制度の適用はありません。契約内容（自動更新・解約方法を含む）は購入手続き画面および「特定商取引法に基づく表記」で決済前にご確認いただけます。</li>
        </ul>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">6. AI 自律実行に伴う責任の制限</h2>
        <p>
          本ソフトウェアは、AI によるコードの自動生成、デバッグ、自己修復（Self-Healing）、および外部 API への自動的な接続要求やタスクの自律的実行を行います。
        </p>
        <ol className="list-decimal pl-6 space-y-4">
          <li>
            当社は、<strong>当社の故意または重過失による場合を除き</strong>、本ソフトウェアの自律的な動作（自律的なコード書き換えを含む）に起因して発生した以下の事象について責任を負いません。
            <ul className="list-disc pl-6 mt-2 space-y-1">
              <li>稼働サーバー内のデータ消失、ファイルの破損、データベースの不整合</li>
              <li>自律的な外部 API 呼び出しに伴う、サードパーティサービス（LLM、TTS 等）の想定外の課金コストの発生</li>
              <li>自動生成されたコードまたはアセットが第三者の知的財産権（著作権、特許権等）を侵害した場合のトラブル</li>
              <li>その他、AI の意思決定および行動の結果生じたシステム障害や機会損失</li>
            </ul>
          </li>
          <li>
            当社が損害賠償責任を負う場合であっても、その賠償額は、<strong>当該ユーザーが損害発生の直近 12 か月間に当社へ支払った利用料金の総額を上限</strong>とします。ただし、当社の故意または重過失による場合、および消費者契約法その他の法令によりこの制限が許されない場合はこの限りではありません。
          </li>
          <li>ユーザーは、外部 API の利用上限（コストサーキットブレーカー・支出上限設定等）を自己の責任で適切に設定するものとします。</li>
        </ol>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">7. 禁止事項</h2>
        <p>ユーザーは、本ソフトウェアの利用にあたり、以下の行為を行ってはなりません。</p>
        <ul className="list-disc pl-6 space-y-2">
          <li>児童ポルノ（CSAM）の生成、極度に有害または毒性のある表現の自律拡散</li>
          <li>不正アクセス、他のセル環境への意図的なサイバー攻撃</li>
          <li>本ソフトウェアを利用した詐欺行為または他者へのなりすまし</li>
          <li>法令または公序良俗に違反する行為</li>
        </ul>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">8. 本規約の変更</h2>
        <p>
          本規約は、民法第 548 条の 4（定型約款の変更）に基づき、ユーザーの一般の利益に適合する場合、または変更が合理的なものである場合に変更されることがあります。変更する場合、当社は効力発生日の <strong>14 日前まで</strong>（ユーザーに不利益な変更の場合は 30 日前まで）に、変更内容と効力発生日を公式サイトまたはアプリケーション内で周知します。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">9. 準拠法・管轄裁判所</h2>
        <p>
          本規約は<strong>日本法</strong>に準拠します。本ソフトウェアおよび Pro プランに関する一切の紛争については、<strong>京都地方裁判所</strong>を第一審の専属的合意管轄裁判所とします。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">10. 正文</h2>
        <p>
          本規約は日本語を正文とします。参考のために英語その他の言語への翻訳が作成された場合でも、日本語の正文のみが効力を有します。
        </p>
      </section>

      <section className="space-y-4 border-t border-white/10 pt-6">
        <blockquote className="border-l-4 border-brand-cyan/50 pl-4 text-gray-400 italic">
          <strong>合意確認</strong>: ユーザーが本ソフトウェアの初期設定（セットアップウィザード）において本規約への同意を選択し、初期設定を完了した時点で、本規約のすべての条項に同意したものとみなされます。同意時には、同意した規約の版（v2.0）と日時がシステム設定に記録されます。
        </blockquote>
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
    <LegalLayout title={t('footer.tokushoho', '特定商取引法に基づく表記')} lastUpdated="2026-07-15">
      <section className="space-y-4 mb-6">
        <p className="text-gray-400">特定商取引法に基づき、以下の通り表示いたします。</p>
      </section>

      <table className="w-full text-left border-collapse border border-white/10 rounded-2xl overflow-hidden bg-brand-surface/20">
        <tbody>
          <tr className="border-b border-white/5">
            <th className="p-4 w-1/3 bg-white/5 text-white font-semibold">販売業者</th>
            <td className="p-4">motivationstudio, LLC（モチベーションスタジオ合同会社）</td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">運営責任者</th>
            <td className="p-4">モチベーションスタジオ合同会社 代表社員</td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">所在地</th>
            <td className="p-4">京都府京都市出雲路俵町35（モチベーションスタジオ合同会社 本店所在地）</td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">電話番号</th>
            <td className="p-4">080-3804-0184（受付: 原則 3 営業日以内に折り返し。日常的なお問い合わせは下記メールアドレスを推奨）</td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">問い合わせ先メールアドレス</th>
            <td className="p-4">project.aiome@gmail.com</td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">ホームページ</th>
            <td className="p-4">
              <a href="https://aiome.dev" target="_blank" rel="noopener noreferrer" className="text-brand-cyan hover:underline">https://aiome.dev</a>
            </td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">販売する役務の内容・分量</th>
            <td className="p-4">
              <strong>Aiome Pro（月額サブスクリプション）</strong> — 各課金周期につき <strong>1 か月分</strong>の Pro 機能利用権を提供します。提供機能の詳細は購入手続き画面および公式サイトの価格説明をご確認ください（一部機能は順次解禁）。
            </td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">販売価格</th>
            <td className="p-4">
              <strong>月額 19.99 米ドル（USD）</strong>（税務上の取り扱いはお客様の居住地・カード会社の表示に従います）
              <ul className="list-disc pl-6 mt-2 space-y-1">
                <li>日本円でのお支払額は、決済時の為替レートおよびカード会社の換算手数料により変動します。</li>
                <li>販売価格は、購入手続き画面（Stripe Checkout）にも表示されます。</li>
              </ul>
            </td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">販売価格以外でお客様に発生する費用</th>
            <td className="p-4">
              <ul className="list-disc pl-6 space-y-1">
                <li>インターネット接続料金・通信料金（お客様のご負担）</li>
                <li>本ソフトウェアから外部 AI サービス（LLM API 等）を利用する場合の当該サービスの利用料金（お客様が各事業者と直接契約し、負担するもの）</li>
              </ul>
            </td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">支払方法</th>
            <td className="p-4">クレジットカード決済（決済代行: Stripe, Inc.）</td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">支払時期</th>
            <td className="p-4">
              <ul className="list-disc pl-6 space-y-1">
                <li><strong>初回</strong>: 購入手続き完了時にお支払いが確定します。</li>
                <li><strong>2 回目以降</strong>: 契約は 1 か月ごとに自動更新され、各更新日に翌 1 か月分の料金が自動的に請求されます。</li>
                <li>無料トライアルを当社が設定した場合に限り、その期間・条件は購入手続き画面に表示し、満了後に初回課金が発生します（<strong>現時点では無料トライアルは提供していません</strong>）。</li>
              </ul>
            </td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">サービスの提供時期</th>
            <td className="p-4">決済完了後、ただちに（システムによる自動処理で）Pro 機能が有効化されます。</td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">解約（自動更新の停止）</th>
            <td className="p-4">
              <ul className="list-disc pl-6 space-y-1">
                <li>アプリケーション内の「お支払い管理」から <strong>Stripe カスタマーポータル</strong>にアクセスし、いつでも解約手続を行うことができます（申込みと同程度に簡便な Web 手続）。</li>
                <li>解約手続後も、支払済みの契約期間の末日までは Pro 機能をご利用いただけます。次回更新日以降の請求は発生しません。</li>
                <li>
                  詳細な手順は「
                  <a href="/cancellation" className="text-brand-cyan hover:underline">解約・返金ポリシー</a>
                  」をご確認ください。
                </li>
              </ul>
            </td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">返品・キャンセル（返金ポリシー）</th>
            <td className="p-4">
              <ul className="list-disc pl-6 space-y-1">
                <li>デジタル役務の性質上、決済完了後の返金は原則として行いません。契約期間途中の解約による日割り返金も行いません。</li>
                <li>ただし、(1) 法令に基づき返金が義務付けられる場合、(2) 当社の責めに帰すべき事由によりサービスが提供されなかった場合、(3) <strong>決済から 24 時間以内かつ Pro 機能を実質的に未使用の誤購入</strong>について当社が認めた場合は、返金に応じることがあります。</li>
                <li>通信販売のため、クーリング・オフ制度の適用はありません。</li>
              </ul>
            </td>
          </tr>
          <tr className="border-b border-white/5">
            <th className="p-4 bg-white/5 text-white font-semibold">動作環境</th>
            <td className="p-4">Docker が動作する環境（macOS / Linux 等）。詳細は公式サイトおよび README をご確認ください。</td>
          </tr>
          <tr>
            <th className="p-4 bg-white/5 text-white font-semibold">その他</th>
            <td className="p-4">
              <ul className="list-disc pl-6 space-y-1">
                <li>本表記は Aiome Pro（月額サブスクリプション）を対象としています。</li>
                <li>アプリケーション内ポイント「Karma Coins (KC)」は無償で付与されるものであり、<strong>販売・換金の対象ではありません</strong>。</li>
              </ul>
            </td>
          </tr>
        </tbody>
      </table>
    </LegalLayout>
  );
}

// -------------------------------------------------------------
// Customer Support Page（Stripe Business「顧客サポート URL」用）
// -------------------------------------------------------------
export function SupportPage() {
  const { t } = useTranslation();

  return (
    <LegalLayout title={t('footer.support', 'カスタマーサポート')} lastUpdated="2026-07-16">
      <section className="space-y-4">
        <p className="text-gray-400">
          Aiome（運営: motivationstudio, LLC / モチベーションスタジオ合同会社）のカスタマーサポート窓口です。
          課金・解約・不具合・法務文書に関するお問い合わせは、以下までご連絡ください。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">連絡先</h2>
        <ul className="list-disc pl-6 space-y-2">
          <li>
            メール:{' '}
            <a href="mailto:project.aiome@gmail.com" className="text-brand-cyan hover:underline">
              project.aiome@gmail.com
            </a>
            （推奨）
          </li>
          <li>電話: 080-3804-0184（受付: 原則 3 営業日以内に折り返し）</li>
          <li>
            公式サイト:{' '}
            <a href="https://aiome.dev" className="text-brand-cyan hover:underline">
              https://aiome.dev
            </a>
          </li>
        </ul>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">関連ページ</h2>
        <ul className="list-disc pl-6 space-y-2">
          <li>
            <a href="/privacy" className="text-brand-cyan hover:underline">プライバシーポリシー</a>
          </li>
          <li>
            <a href="/terms" className="text-brand-cyan hover:underline">利用規約</a>
          </li>
          <li>
            <a href="/tokushoho" className="text-brand-cyan hover:underline">特定商取引法に基づく表記</a>
          </li>
          <li>
            <a href="/cancellation" className="text-brand-cyan hover:underline">解約・返金ポリシー</a>
          </li>
        </ul>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">解約について</h2>
        <p>
          Pro サブスクリプションの解約は、管理コンソールの Stripe カスタマーポータルからいつでも行えます。
          手順の詳細は「
          <a href="/cancellation" className="text-brand-cyan hover:underline">解約・返金ポリシー</a>
          」をご確認ください。
        </p>
      </section>
    </LegalLayout>
  );
}

// -------------------------------------------------------------
// Cancellation & Refund Policy Page
// -------------------------------------------------------------
export function CancellationPage() {
  const { t } = useTranslation();

  return (
    <LegalLayout title={t('footer.cancellation', '解約・返金ポリシー')} lastUpdated="2026-07-15">
      <section className="space-y-4">
        <p className="text-gray-400">
          本ポリシーは、Aiome Pro（月額サブスクリプション）の解約手続と返金の取り扱いを定めるものです。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">1. 解約の手順</h2>
        <ol className="list-decimal pl-6 space-y-2">
          <li>Aiome 管理コンソールにログインします。</li>
          <li>コマース画面等の「お支払い管理」または「サブスク管理」から <strong>Stripe カスタマーポータル</strong> を開きます。</li>
          <li>ポータル内の「プランをキャンセル」を選択し、画面の案内に従って解約を確定します。</li>
        </ol>
        <p>解約はいつでも可能で、違約金は発生しません。電話や書面のみの解約手続は要求しません。</p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">2. 解約の効果</h2>
        <ul className="list-disc pl-6 space-y-2">
          <li>解約手続の完了後も、<strong>支払済みの契約期間の末日までは Pro 機能をご利用いただけます。</strong></li>
          <li>契約期間の末日をもって Pro 機能は無効化され、以降の請求は発生しません。</li>
          <li>契約期間の途中で解約された場合でも、<strong>日割りによる返金は行いません。</strong></li>
        </ul>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">3. 無料トライアル</h2>
        <p>
          <strong>現時点では無料トライアルは提供していません。</strong> 将来提供する場合は、購入手続き画面に期間・条件を表示し、トライアル中の解約では料金が発生しません。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">4. 返金</h2>
        <p>原則として、決済完了後の返金は行いません。ただし次の場合は例外とします。</p>
        <ol className="list-decimal pl-6 space-y-2">
          <li><strong>法令に基づき返金が義務付けられる場合</strong></li>
          <li><strong>当社の責めに帰すべき事由</strong>により、決済完了後も Pro 機能が提供されなかった場合</li>
          <li>
            <strong>善意対応（誤購入）</strong>: 決済から <strong>24 時間以内</strong>であり、かつ当該課金周期において Pro 機能を<strong>実質的に利用していない</strong>と当社が合理的に判断できる場合（類似プロダクトで一般的な限定的返金窓口）
          </li>
        </ol>
        <p>
          返金をご希望の場合は、決済日・登録メールアドレス・理由を添えて <code>project.aiome@gmail.com</code> までご連絡ください。当社にて事実関係を確認の上、対応いたします（返金が認められた場合、原支払方法への返金には数営業日かかることがあります）。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">5. 支払失敗によるサービス停止</h2>
        <p>
          登録されたお支払方法での決済が失敗した場合、Pro 機能は一時停止されることがあります。お支払方法を更新し決済が完了すると、自動的に再開されます。
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-xl font-bold text-white">6. お問い合わせ</h2>
        <p>本ポリシーに関するお問い合わせ: <code>project.aiome@gmail.com</code></p>
      </section>
    </LegalLayout>
  );
}
