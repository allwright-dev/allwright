import { expect, test as base } from '../dist/index.js';
import { shutdown } from '@allwright.dev/core';
import { afterAll } from 'vitest';

const test = base.extend({
  allwright: async ({}, use) => {
    await use({ browser: process.env.ALLWRIGHT_TEST_BROWSER === 'firefox' ? 'firefox' : 'chromium' });
  },
});
afterAll(() => shutdown());

test.skipIf(!process.env.ALLWRIGHT_TEST_BROWSER)('semantic locators and filters work through lazy fixtures and the engine', { timeout: 30_000 }, async ({ page }) => {
  // Construct before navigation to exercise the lazy page/locator wrappers.
  const beta = page.getByRole('listitem').filter({ has: page.getByRole('heading', { name: /^Beta$/ }) });
  const purchase = beta.getByRole('button', { name: 'Buy', exact: true });
  await page.goto('data:text/html,' + encodeURIComponent('<ul><li><h2>Alpha</h2><button>Buy</button></li><li><h2>Beta</h2><button onclick="this.textContent=\'Done\'">Buy</button></li></ul><label>Email<input placeholder="Email address"></label><img alt="Company logo"><span title="Greeting" data-testid="welcome">Hello world</span>'));
  await expect(beta).toHaveCount(1);
  await expect(page.getByRole('listitem').not(beta)).toHaveCount(1);
  await expect(page.getByRole('listitem').not(beta).getByRole('heading')).toHaveText('Alpha');
  await expect(beta).not().toHaveText('AlphaBuy');
  await expect(beta).not.toHaveCount(0);
  await expect(page.getByTestId('absent')).not().toBeVisible({ timeoutMs: 1000 });
  await expect(page.locator('li', { hasNot: page.getByText('Alpha', { exact: true }) })).toHaveCount(1);
  await expect(page.getByRole('listitem').filter({ hasText: /beta/i, hasNotText: 'Alpha', visible: true })).toHaveCount(1);
  await expect(page.getByRole('listitem').first().getByRole('heading')).toHaveText('Alpha');
  await expect(page.getByRole('listitem').last().locator('xpath=//h2')).toHaveText('Beta');
  await purchase.click();
  await expect(beta.getByText('Done')).toHaveCount(1);
  await page.getByLabel('Email', { exact: true }).fill('test@example.com');
  await expect(page.getByPlaceholder('Email address')).toHaveCount(1);
  await expect(page.getByAltText(/company/i)).toHaveCount(1);
  await expect(page.getByTitle('Greeting')).toHaveCount(1);
  await expect(page.getByTestId('welcome')).toHaveText('Hello world');
});

test.skipIf(!process.env.ALLWRIGHT_TEST_BROWSER)('negative assertions retry visibility and removal changes', { timeout: 30_000 }, async ({ page }) => {
  await page.goto('data:text/html,' + encodeURIComponent('<div id="busy">Working</div><div id="notice">Notice</div><button onclick="setTimeout(() => { document.querySelector(\'#busy\').hidden = true; document.querySelector(\'#notice\').remove(); }, 150)">Dismiss</button>'));
  await expect(page.locator('#busy')).toBeVisible();
  await page.getByRole('button', { name: 'Dismiss' }).click();
  await expect(page.locator('#busy')).not().toBeVisible({ timeoutMs: 3000, intervalMs: 20 });
  await expect(page).not.toHaveCount('#notice', 1, { timeoutMs: 3000, intervalMs: 20 });
  await expect(page).not().toBeVisible('#notice', { timeoutMs: 1000 });
});
