import { describe, it, expect } from 'vitest';

/** sRGB relative luminance per WCAG 2.1 */
function relativeLuminance(r: number, g: number, b: number): number {
  const toLinear = (c: number) => {
    const s = c / 255;
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * toLinear(r) + 0.7152 * toLinear(g) + 0.0722 * toLinear(b);
}

/** Contrast ratio between two sRGB colors */
function contrastRatio(
  r1: number, g1: number, b1: number,
  r2: number, g2: number, b2: number,
): number {
  const l1 = relativeLuminance(r1, g1, b1);
  const l2 = relativeLuminance(r2, g2, b2);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}

/** Parse a hex color like #1a2b3c or #abc */
function parseHex(hex: string): [number, number, number] {
  let h = hex.replace('#', '');
  if (h.length === 3) h = h[0]+h[0]+h[1]+h[1]+h[2]+h[2];
  return [
    parseInt(h.slice(0, 2), 16),
    parseInt(h.slice(2, 4), 16),
    parseInt(h.slice(4, 6), 16),
  ];
}

const BG_MAIN = '#FFFDF8';      // --color-bg-main
const TEXT_PRIMARY = '#2D2A26'; // --color-text-primary
const TEXT_MUTED = '#8C8880';   // --color-text-muted
const COLOR_DANGER = '#C44A3F'; // --color-danger
const COLOR_ACCENT = '#3F865E'; // --color-accent
const COLOR_INFO = '#4A7BAF';   // --color-info
const COLOR_SEV_5 = '#C44A3F';  // severity-5 (danger)
const COLOR_SEV_1 = '#8C8880';  // severity-1 (info)

describe('WCAG AA color contrast', () => {
  it('primary text on main background >= 4.5:1', () => {
    const ratio = contrastRatio(
      ...parseHex(TEXT_PRIMARY), ...parseHex(BG_MAIN),
    );
    expect(ratio).toBeGreaterThanOrEqual(4.5);
  });

  it('muted text on main background >= 4.5:1', () => {
    const ratio = contrastRatio(
      ...parseHex(TEXT_MUTED), ...parseHex(BG_MAIN),
    );
    expect(ratio).toBeGreaterThanOrEqual(4.5);
  });

  it('danger text on main background >= 4.5:1', () => {
    const ratio = contrastRatio(
      ...parseHex(COLOR_DANGER), ...parseHex(BG_MAIN),
    );
    expect(ratio).toBeGreaterThanOrEqual(4.5);
  });

  it('accent text on main background >= 4.5:1', () => {
    const ratio = contrastRatio(
      ...parseHex(COLOR_ACCENT), ...parseHex(BG_MAIN),
    );
    expect(ratio).toBeGreaterThanOrEqual(4.5);
  });

  it('info text on main background >= 4.5:1', () => {
    const ratio = contrastRatio(
      ...parseHex(COLOR_INFO), ...parseHex(BG_MAIN),
    );
    expect(ratio).toBeGreaterThanOrEqual(4.5);
  });

  it('severity-5 text on main background >= 4.5:1', () => {
    const ratio = contrastRatio(
      ...parseHex(COLOR_SEV_5), ...parseHex(BG_MAIN),
    );
    expect(ratio).toBeGreaterThanOrEqual(4.5);
  });

  it('severity-1 text on main background >= 4.5:1', () => {
    const ratio = contrastRatio(
      ...parseHex(COLOR_SEV_1), ...parseHex(BG_MAIN),
    );
    expect(ratio).toBeGreaterThanOrEqual(4.5);
  });
});
