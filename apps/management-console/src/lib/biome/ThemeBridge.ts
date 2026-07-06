import * as THREE from 'three';
import { cssVar } from '../../utils/cssVar';

class ThemeBridge {
  private static colorCache: Map<string, THREE.Color> = new Map();
  private static stringCache: Map<string, string> = new Map();
  private static observer: MutationObserver | null = null;

  static {
    // ユーザー環境がブラウザで document が存在する場合にのみ監視を開始する
    if (typeof document !== 'undefined') {
      this.initObserver();
    }
  }

  private static initObserver() {
    this.observer = new MutationObserver(() => {
      // テーマ変更など、DOM要素の属性が変わったらキャッシュをクリア
      this.colorCache.clear();
      this.stringCache.clear();
    });

    this.observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class', 'data-theme', 'style']
    });
  }

  /**
   * CSS変数から値を取得する (キャッシュあり)
   */
  private static getCssVariable(varName: string, fallback: string): string {
    if (typeof document === 'undefined') return fallback;
    
    const cached = this.stringCache.get(varName);
    if (cached) return cached;

    const value = getComputedStyle(document.documentElement)
      .getPropertyValue(varName)
      .trim();

    const result = value || fallback;
    this.stringCache.set(varName, result);
    return result;
  }

  /**
   * 3D WebGL 描画用 (THREE.Color) の色を取得する
   */
  public static getElementColor(element: string): THREE.Color {
    const el = element.toLowerCase();
    const cached = this.colorCache.get(el);
    if (cached) return cached;

    const tokenName = `--biome-element-${el}`;
    const fallback = el === 'fallback'
      ? cssVar('--biome-element-fallback')
      : cssVar('--white-100');
    const hex = this.getCssVariable(tokenName, fallback);

    const color = new THREE.Color(hex);
    this.colorCache.set(el, color);
    return color;
  }

  /**
   * UI/DOM 描画用の色 (文字列のHEX) を取得する
   */
  public static getUiElementColor(element: string): string {
    const el = element.toLowerCase();
    const tokenName = `--biome-ui-element-${el}`;
    const fallback = cssVar(`--biome-ui-element-${el}`, cssVar('--accent-cyan'));

    return this.getCssVariable(tokenName, fallback);
  }
}

export default ThemeBridge;
