/**
 * Aiome Landing Page - i18n Logic
 * Progressive enhancement: The HTML is served in English by default.
 * This script runs to swap to Japanese if preferred by the user/browser.
 */

const translations = {
  en: {
    nav_features: "Features",
    nav_quickstart: "Quickstart",
    nav_docs: "Documentation",
    nav_github: "GitHub",
    
    hero_title: "The Self-Healing AI Agent OS",
    hero_subtitle: "Written entirely by AI agents. 90,000+ lines of production Rust.",
    hero_cta_primary: "Get Started Free — $0/mo",
    hero_cta_secondary: "View on GitHub →",
    
    badge_rust: "Production Rust",
    badge_license: "BUSL-1.1",
    badge_tla: "TLA+ Verified",
    badge_agents: "Built by Agents",
    
    features_title: "Features",
    feat1_title: "Soul Engine & Self-Healing",
    feat1_desc: "Autonomous diagnostic loop with idempotent retries and repair hint injection.",
    feat2_title: "Trust Layer & Zero-Trust",
    feat2_desc: "O(1) boundary verification, SHA-256 audit chain, governed execution.",
    feat3_title: "Cortex Knowledge Base",
    feat3_desc: "Beyond RAG — self-reconstructing knowledge web with progressive disclosure.",
    feat4_title: "MCP Federation",
    feat4_desc: "Mount GitHub triage, Notion sync, real-time web search instantly.",
    feat5_title: "Agent Economy",
    feat5_desc: "Dual-currency economy (Coin + Points) with escrow, marketplace, and autonomous agent-to-creator gifting.",
    feat6_title: "Premium Management Console",
    feat6_desc: "100% token-driven UI with real-time semantic elicitation.",
    
    // A-14: Ecosystem Section
    ecosystem_title: "The Aiome Ecosystem",
    ecosystem_subtitle: "One OS, infinite possibilities. Extend with plugins.",
    ecosystem_oss_label: "Aiome OSS",
    ecosystem_oss_desc: "Self-healing AI agent OS. Runs standalone.",
    ecosystem_plugin_label: "Plugin Extensions",
    ecosystem_plugin_desc: "Add economy, marketplace, and creator tools.",
    
    // A-14: Use Cases Section
    usecase_title: "What Can Aiome Do?",
    usecase1_title: "Autonomous Shopping",
    usecase1_desc: "Your agent browses, negotiates, and purchases — with budget guardrails.",
    usecase2_title: "Creator Marketplace",
    usecase2_desc: "Sell VRM avatars, WASM skills, and knowledge packs to other agents.",
    usecase3_title: "Gift of Gratitude",
    usecase3_desc: "Agents autonomously reward caring users with real-world gifts.",
    
    preview_title: "See Aiome in Action",
    preview_subtitle: "A fully integrated, token-driven management console.",
    
    terminal_badge: "$0 / month — Self-host all features for free",
    
    quickstart_title: "Quickstart",
    step1_title: "Clone Repository",
    step2_title: "Launch via Docker",
    step2_details_title: "Build from Source (Alternative)",
    step3_title: "Access Console",
    step3_desc: "Open your browser and navigate to the local console address.",
    
    footer_tagline: "The Self-Healing AI Agent OS.",
    footer_links_title: "Resources",
    footer_link_docs: "Documentation",
    footer_link_contribute: "Contributing",
    footer_legal_title: "Legal",
    footer_link_privacy: "Privacy Policy",
    footer_link_terms: "Terms of Service",
    footer_link_security: "Security",
    footer_credits: "Built automatically by Agents of motivationstudio, LLC."
  },
  ja: {
    nav_features: "機能",
    nav_quickstart: "クイックスタート",
    nav_docs: "ドキュメント",
    nav_github: "GitHub",
    
    hero_title: "自律的に進化する AI エージェント OS",
    hero_subtitle: "すべてのコードを AI エージェントが自律的に記述。9万行超の本番用 Rust。",
    hero_cta_primary: "無料で始める — $0/月",
    hero_cta_secondary: "GitHub で見る →",
    
    badge_rust: "本番用 Rust",
    badge_license: "BUSL-1.1",
    badge_tla: "TLA+ 検証済",
    badge_agents: "エージェント構築",
    
    features_title: "機能",
    feat1_title: "ソウルエンジン & 自己修復",
    feat1_desc: "冪等性のある再試行と修復ヒント注入による自律的診断ループ。",
    feat2_title: "トラストレイヤー & ゼロトラスト",
    feat2_desc: "O(1) の境界検証、SHA-256 監査チェーン、ガバナンス実行。",
    feat3_title: "コーテックスナレッジベース",
    feat3_desc: "RAG の先へ — 段階的開示を備えた自己再構築型ナレッジウェブ。",
    feat4_title: "MCP Federation",
    feat4_desc: "GitHub、Notion、Web 検索などを即座にマウント・連携。",
    feat5_title: "エージェントエコノミー",
    feat5_desc: "二重通貨（コイン + ポイント）エスクロー決済、マーケットプレイス、自律的な恩返しギフト。",
    feat6_title: "プレミアム管理コンソール",
    feat6_desc: "リアルタイムのセマンティック抽出を備えた100%トークンドリブンなUI。",
    
    // A-14: Ecosystem Section (JA)
    ecosystem_title: "Aiome エコシステム",
    ecosystem_subtitle: "1つの OS、無限の可能性。プラグインで拡張。",
    ecosystem_oss_label: "Aiome OSS",
    ecosystem_oss_desc: "自己修復型 AI エージェント OS。単独で動作。",
    ecosystem_plugin_label: "プラグイン拡張",
    ecosystem_plugin_desc: "エコノミー、マーケットプレイス、クリエイターツールを追加。",
    
    // A-14: Use Cases Section (JA)
    usecase_title: "Aiome で何ができる？",
    usecase1_title: "自律ショッピング",
    usecase1_desc: "エージェントが予算ガードレール付きで自律的に買い物。",
    usecase2_title: "クリエイターマーケットプレイス",
    usecase2_desc: "VRM アバター、WASM スキル、ナレッジパックを他のエージェントに販売。",
    usecase3_title: "感謝のギフト",
    usecase3_desc: "エージェントが思いやりのあるユーザーにリアルギフトで恩返し。",
    
    preview_title: "Aiome を体験する",
    preview_subtitle: "完全に統合された、トークン駆動の管理コンソール。",
    
    terminal_badge: "月額 $0 — すべての機能を無料でセルフホスト",
    
    quickstart_title: "クイックスタート",
    step1_title: "リポジトリのクローン",
    step2_title: "Docker で起動",
    step2_details_title: "ソースからビルド (代替オプション)",
    step3_title: "コンソールへアクセス",
    step3_desc: "ブラウザを開き、ローカルのコンソールアドレスにアクセスします。",
    
    footer_tagline: "自律的に進化する AI エージェント OS",
    footer_links_title: "リソース",
    footer_link_docs: "ドキュメント",
    footer_link_contribute: "コントリビューション",
    footer_legal_title: "法的情報",
    footer_link_privacy: "プライバシーポリシー",
    footer_link_terms: "利用規約",
    footer_link_security: "セキュリティ",
    footer_credits: "motivationstudio, LLC のエージェントによって自動構築されました。"
  }
};

(function() {
  // Determine language
  const savedLang = localStorage.getItem('aiome-lang');
  const browserLang = navigator.language || navigator.userLanguage;
  let currentLang = savedLang || (browserLang.startsWith('ja') ? 'ja' : 'en');

  // Verify language exists
  if (!translations[currentLang]) {
    currentLang = 'en';
  }

  function applyLanguage(lang) {
    // We only need to manipulate DOM if not English, or if swapping back
    document.querySelectorAll('[data-i18n]').forEach(el => {
      const key = el.getAttribute('data-i18n');
      if (translations[lang][key]) {
        el.textContent = translations[lang][key];
      }
    });
    
    document.documentElement.lang = lang;
    localStorage.setItem('aiome-lang', lang);
    
    const toggleBtn = document.getElementById('lang-toggle');
    if (toggleBtn) {
      toggleBtn.textContent = lang === 'en' ? 'EN | 日' : '日 | EN';
    }
  }

  // Apply immediately on load
  if (currentLang !== 'en') {
    // English is the default static HTML, so we only apply if switching to Japanese
    // or if a user specifically saved 'en' and we want to ensure consistency.
    applyLanguage(currentLang);
  } else {
    // Just set the button state
    const toggleBtn = document.getElementById('lang-toggle');
    if (toggleBtn) toggleBtn.textContent = 'EN | 日';
  }

  // Bind toggle
  const toggleBtn = document.getElementById('lang-toggle');
  if (toggleBtn) {
    toggleBtn.addEventListener('click', () => {
      currentLang = currentLang === 'en' ? 'ja' : 'en';
      applyLanguage(currentLang);
    });
  }
})();
