import { expect, test } from '@playwright/test';

test('the inspector seeds a world, steps it, and shows what came out of it', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto('./?seed=e2e-1');
  await expect(page.locator('#status')).toContainText('period 0');
  await page.locator('#step-13').click();
  await expect(page.locator('#status')).toContainText('period 13', { timeout: 30_000 });
  const audit = page.locator('#audit table tbody tr');
  await expect(audit.first()).toBeVisible();
  await expect(page.locator('#audit tr.red')).toHaveCount(0);
  await expect(page.locator('#audit tr.unbuilt')).toHaveCount(2);
  // Sovereign D3: a curve with a point per line, each saying whether it traded or was carried.
  const curve = page.locator('section.curve').first();
  await expect(curve).toBeVisible();
  await expect(curve.locator('tbody tr')).not.toHaveCount(0);
  // Observer A1.a: a mark that did not trade this period is visibly stale.
  await expect(page.locator('#prints tr.stale').first()).toBeVisible();
  // Sovereign C1.a, C4: what the issuer announced and what the auction did.
  await expect(page.locator('#sovereign')).toContainText('programme');
  await expect(page.locator('#sovereign tbody tr').first()).toBeVisible();
  expect(errors).toEqual([]);
});
