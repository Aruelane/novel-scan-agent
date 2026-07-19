import { describe, it, expect } from 'vitest';

/** sRGB relative luminance per WCAG 2.1 */
function relativeLuminance(r: number, g: number, b: number): number {
  const toLinear = (c: number) => {
    const s = c / 255;
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * toLinear(r) + 0.7152 * toLinear(g) + 0.0722 * toLinear(b);
}

function contrastRatio(
  r1: number, g1: number, b1: number,
  r2: number, g2: number, b2: number,
): number {
  const l1 = relativeLuminance(r1, g1, b1);
  const l2 = relativeLuminance(r2, g2, b2);
  return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
}

function parseHex(hex: string): [number, number, number] {
  let h = hex.replace('#', '');
  if (h.length === 3) h = h[0]+h[0]+h[1]+h[1]+h[2]+h[2];
  return [parseInt(h.slice(0,2),16), parseInt(h.slice(2,4),16), parseInt(h.slice(4,6),16)];
}

// Actual CSS custom property values from index.css
const BG = '#fdf8f3';
const pairs: [string, string, string][] = [
  ['primary text', '#3b332b', BG],
  ['muted text', '#705e4c', BG],
  ['danger text', '#a33d30', BG],
  ['accent text', '#3d7040', BG],
  ['info text', '#3a6080', BG],
  ['severity-5 text', '#a33d30', BG],
  ['severity-1 text', '#705e4c', BG],
];

describe('WCAG AA color contrast', () => {
  for (const [label, fg, bg] of pairs) {
    it(`${label} on background >= 4.5:1`, () => {
      const ratio = contrastRatio(...parseHex(fg), ...parseHex(bg));
      expect(ratio).toBeGreaterThanOrEqual(4.5);
    });
  }
});
