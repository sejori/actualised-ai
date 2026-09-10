import { test, expect } from '@playwright/test';

test('has title and setup page', async ({ page }) => {
  await page.goto('./');
  
  // Expect a title "to contain" a substring.
  await expect(page).toHaveTitle(/Actualised/i);

  // Check that the Setup layout appears
  const heading = page.locator('h1').first();
  await expect(heading).toBeVisible();

  // Optionally run a visual comparison
  // await expect(page).toHaveScreenshot('setup-page.png');
});

test('loads the canvas workspace on new project', async ({ page }) => {
  await page.goto('./');
  
  // Click "New Graph" or start button
  const startButton = page.getByRole('button', { name: 'Start' });
  if (await startButton.isVisible()) {
    await startButton.click();
    
    // Expect canvas to be visible
    const canvas = page.locator('.canvas-shell').first();
    await expect(canvas).toBeVisible();
  }
});

test('creates an agent from the keyboard without halting reactivity', async ({ page }) => {
  const pageErrors: Error[] = [];
  page.on('pageerror', (error) => pageErrors.push(error));

  await page.goto('./');
  await page.getByRole('button', { name: 'Initialise company' }).click();

  const addAgent = page.getByRole('button', { name: 'Add Agent' });
  await page.locator('body').focus();
  await page.keyboard.press('Tab');
  await expect(addAgent).toBeFocused();
  await page.keyboard.press('Enter');

  await expect(page.getByText('8 agents')).toBeVisible();
  await expect(page.getByRole('complementary', { name: 'New Agent details' })).toBeVisible();

  await page.getByRole('button', { name: 'Cancel' }).click();
  await page.getByRole('button', { name: 'Close details' }).click();
  const toolLibrary = page.getByRole('button', { name: 'Tool Library', exact: true });
  await toolLibrary.focus();
  await page.keyboard.press('Enter');

  await expect(page.getByRole('complementary', { name: 'Tool Library' })).toBeVisible();
  expect(pageErrors).toEqual([]);
});
