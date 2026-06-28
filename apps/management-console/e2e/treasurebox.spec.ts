/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { test, expect } from '@playwright/test';

test.describe('TreasureBox Component (TDD)', () => {
    test('should not contain Tailwind utility classes', async ({ page }) => {
        // We will mock or navigate to a page that renders TreasureBox.
        // It should be rendered on the HomePage or via a direct component test.
        // For E2E, we go to the home page if it has TreasureBox, or we can check the component directly.
        await page.goto('/');
        
        // Wait for TreasureBox to be visible or check if it's there
        // Actually TreasureBox is used in HomePage.
        const treasureBox = page.locator('.artemis-treasure-box').first();
        
        // This test will fail initially because we haven't added the .artemis-treasure-box class yet!
        await expect(treasureBox).toBeVisible({ timeout: 5000 });
        
        // Check that old Tailwind classes are gone
        const oldTailwindElement = page.locator('.bg-white\\/5').first();
        await expect(oldTailwindElement).toHaveCount(0);
    });

    test('should use artemis-heading for the title', async ({ page }) => {
        await page.goto('/');
        
        const heading = page.locator('.artemis-treasure-box .artemis-heading').first();
        await expect(heading).toBeVisible({ timeout: 5000 });
        await expect(heading).toHaveText('AI Workspace');
    });
});
