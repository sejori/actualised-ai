import { test, expect } from '@playwright/test';

test('has title and setup page', async ({ page }) => {
  await page.goto('./');
  
  // Expect a title "to contain" a substring.
  await expect(page).toHaveTitle(/Actualised/i);

  // Check that the Setup layout appears
  const heading = page.locator('h1').first();
  await expect(heading).toBeVisible();
  await expect(page.getByRole('button', { name: 'Continue building Pawsome' })).toBeVisible();

  // Optionally run a visual comparison
  // await expect(page).toHaveScreenshot('setup-page.png');
});

test('loads the canvas workspace on new project', async ({ page }) => {
  await page.goto('./');
  await page.getByRole('button', { name: 'Continue building Pawsome' }).click();

  await expect(page.locator('.canvas-shell')).toBeVisible();
  await expect(page.getByText('8 agents')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Pawsome' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Project Manager — Company Orchestrator' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Engineering Lead — Lead Engineer' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Product Lead — Product Manager' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Growth Lead — Marketing Director' })).toBeVisible();
});

test('creates an agent from the keyboard without halting reactivity', async ({ page }) => {
  const pageErrors: Error[] = [];
  page.on('pageerror', (error) => pageErrors.push(error));

  await page.goto('./');
  await page.getByRole('button', { name: 'Continue building Pawsome' }).click();

  const addAgent = page.getByRole('button', { name: 'Add Agent' });
  await page.locator('body').focus();
  await page.keyboard.press('Tab');
  await expect(addAgent).toBeFocused();
  await page.keyboard.press('Enter');

  await expect(page.getByText('9 agents')).toBeVisible();
  await expect(page.getByRole('complementary', { name: 'New Agent details' })).toBeVisible();

  await page.getByRole('button', { name: 'Cancel' }).click();
  await page.getByRole('button', { name: 'Close details' }).click();
  const toolLibrary = page.getByRole('button', { name: 'Tool Library', exact: true });
  await toolLibrary.focus();
  await page.keyboard.press('Enter');

  await expect(page.getByRole('complementary', { name: 'Tool Library' })).toBeVisible();
  expect(pageErrors).toEqual([]);
});
