import { test, expect } from '@playwright/test';

const VIEWPORTS = [
  { width: 390, height: 844, label: 'mobile' },
  { width: 800, height: 900, label: 'tablet' },
  { width: 1440, height: 900, label: 'desktop' },
];

for (const vp of VIEWPORTS) {
  test.describe(`${vp.label} ${vp.width}x${vp.height}`, () => {
    test.use({ viewport: { width: vp.width, height: vp.height } });

    test('no horizontal overflow', async ({ page }) => {
      await page.goto('/');
      const scrollWidth = await page.evaluate(() => document.documentElement.scrollWidth);
      const clientWidth = await page.evaluate(() => document.documentElement.clientWidth);
      expect(scrollWidth).toBeLessThanOrEqual(clientWidth + 1);
    });

    test('main layout regions exist', async ({ page }) => {
      await page.goto('/');
      await expect(page.locator('[aria-label="书架"]')).toBeVisible();
      await expect(page.locator('[aria-label="工作区"]')).toBeVisible();
    });

    if (vp.width <= 800) {
      test('single panel + bottom nav in narrow viewport', async ({ page }) => {
        await page.goto('/');
        await expect(page.locator('.bottom-nav')).toBeVisible();
        // Only one mobile-visible panel at a time
        const visiblePanels = await page.locator('.mobile-visible').count();
        expect(visiblePanels).toBe(1);
      });
    } else {
      test('three-column layout in wide viewport', async ({ page }) => {
        await page.goto('/');
        await expect(page.locator('.bottom-nav')).not.toBeVisible();
        // Sidebar and evidence panel visible
        await expect(page.locator('[aria-label="书架"]')).toBeVisible();
        await expect(page.locator('[aria-label="命中证据面板"]')).toBeVisible();
      });
    }
  });
}
