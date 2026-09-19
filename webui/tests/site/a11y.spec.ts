import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const THEMES = ["light", "dark", "amoled"] as const;

// Shirone's a11y rule: assert the page really is in the scanned theme, so a
// scan cannot pass simply because the override never applied.
for (const theme of THEMES) {
	test(`has no axe violations in the ${theme} theme`, async ({ page }) => {
		await page.goto("/");
		await page.getByTestId(`omk-theme-${theme}`).click();
		await expect(page.locator("html")).toHaveAttribute("data-theme", theme);

		const results = await new AxeBuilder({ page })
			.withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
			.analyze();
		expect(results.violations).toEqual([]);
	});
}
