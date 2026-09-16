import { test, expect } from '@playwright/test';

test('has title and setup page', async ({ page }) => {
  await page.goto('./');
  await expect(page).toHaveTitle(/Actualised/i);
  const heading = page.locator('h1').first();
  await expect(heading).toBeVisible();
  await expect(page.getByRole('button', { name: 'Continue building Pawsome' })).toBeVisible();
});

test('loads the canvas workspace on new project', async ({ page }) => {
  await page.goto('./');
  await page.getByRole('button', { name: 'Continue building Pawsome' }).click();

  await expect(page.locator('.canvas-shell')).toBeVisible();
  await expect(page.getByText('8 agents')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Pawsome' })).toBeVisible();
  
  // Just use regex to avoid em-dash/en-dash encoding issues
  await expect(page.getByRole('button', { name: /Project Manager.*Company Orchestrator/ })).toBeVisible();
});

test('creates an agent from the keyboard without halting reactivity', async ({ page }) => {
  const pageErrors: Error[] = [];
  page.on('pageerror', (error) => pageErrors.push(error));

  await page.goto('./');
  await page.getByRole('button', { name: 'Continue building Pawsome' }).click();

  const addAgent = page.getByRole('button', { name: 'Add Agent' });
  await addAgent.focus();
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

test('edits an existing agent and updates the canvas', async ({ page }) => {
  await page.goto('./');
  await page.getByRole('button', { name: 'Continue building Pawsome' }).click();

  await page.getByRole('button', { name: /Project Manager.*Company Orchestrator/ }).evaluate(b => (b as HTMLElement).click());
  await expect(page.getByRole('heading', { name: 'Project Manager', exact: true })).toBeVisible();
  
  await page.getByRole('button', { name: 'Edit', exact: true }).click();
  
  const nameInput = page.getByLabel('Name');
  await expect(nameInput).toBeVisible();
  
  await nameInput.fill('Senior Project Manager');
  await nameInput.press('Enter');
  
  await expect(page.getByRole('button', { name: /Senior Project Manager.*Company Orchestrator/ })).toBeVisible();
});

test('can queue a message for an agent', async ({ page }) => {
  await page.goto('./');
  await page.getByRole('button', { name: 'Continue building Pawsome' }).click();

  await page.getByRole('button', { name: /Project Manager.*Company Orchestrator/ }).evaluate(b => (b as HTMLElement).click());
  await expect(page.getByRole('heading', { name: 'Project Manager', exact: true })).toBeVisible();

  const messageInput = page.getByLabel("Send a message into this agent's next turn");
  await messageInput.fill('Hello from E2E test!');
  await page.getByRole('button', { name: 'Queue message' }).click();

  await expect(page.locator('.pending-list')).toContainText('Hello from E2E test!');
});
