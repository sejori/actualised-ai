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
