import { expect, test } from '@playwright/test';

test('the inspector seeds a world, steps it, and shows a green audit with stale prints', async ({
  page,
}) => {
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
  await expect(page.locator('#prints tr.stale')).toHaveCount(1);
  expect(errors).toEqual([]);
});
