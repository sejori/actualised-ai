import { test, expect } from '@playwright/test';

test.describe('Cloud Auth Flows', () => {
  test.skip(() => process.env.TEST_MODE !== 'cloud', 'Skipping auth tests in non-cloud mode');

  test('signs up a new user, creates a company, and loads canvas', async ({ page }) => {
    await page.goto('./');

    // 1. Initial Setup Page
    await expect(page).toHaveTitle(/Actualised/i);
    await expect(page.getByRole('heading', { name: 'Build the company that builds the product.' })).toBeVisible();

    // 2. Start creating a company
    const nameInput = page.getByLabel('Company name');
    await expect(nameInput).toBeVisible();
    await nameInput.fill('Test Company');
    await page.getByRole('button', { name: 'Found Venture' }).click();

    // 3. Redirects to Auth Page to create an account for this company
    await expect(page).toHaveURL(/.*company=Test/);
    await expect(page.getByRole('heading', { name: 'Create an account to found Test Company' })).toBeVisible();

    // 4. Sign up
    const emailInput = page.getByLabel('Email', { exact: true });
    const passInput = page.getByLabel('Password', { exact: true });
    
    // Create random user to avoid conflicts
    const testEmail = `test_${Date.now()}@actualised.ai`;
    await emailInput.fill(testEmail);
    await passInput.fill('password123');
    
    await page.getByRole('button', { name: 'Sign Up' }).click();

    // 5. Verify canvas loaded
    await expect(page.locator('.canvas-shell')).toBeVisible({ timeout: 15000 });
    await expect(page.getByText('8 agents')).toBeVisible();
  });
});
