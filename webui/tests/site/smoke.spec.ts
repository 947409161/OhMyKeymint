import { expect, test } from "@playwright/test";

test.describe("WebUI shell", () => {
	test("renders the application shell", async ({ page }) => {
		await page.goto("/");
		await expect(page.getByTestId("omk-app")).toBeVisible();
		await expect(page.getByTestId("omk-shell-status")).toHaveText(
			/React toolchain online/,
		);
	});

	test("applies the selected theme to the document element", async ({ page }) => {
		await page.goto("/");
		for (const mode of ["light", "dark", "amoled"] as const) {
			await page.getByTestId(`omk-theme-${mode}`).click();
			await expect(page.locator("html")).toHaveAttribute("data-theme", mode);
		}
	});

	test("marks the active theme for assistive technology", async ({ page }) => {
		await page.goto("/");
		await page.getByTestId("omk-theme-dark").click();
		await expect(page.getByTestId("omk-theme-dark")).toHaveAttribute(
			"aria-pressed",
			"true",
		);
		await expect(page.getByTestId("omk-theme-light")).toHaveAttribute(
			"aria-pressed",
			"false",
		);
	});

	test("persists the appearance preference across reloads", async ({ page }) => {
		await page.goto("/");
		await page.getByTestId("omk-theme-amoled").click();
		await page.reload();
		await expect(page.locator("html")).toHaveAttribute("data-theme", "amoled");
	});

	test("paints a monochrome surface", async ({ page }) => {
		await page.goto("/");
		await page.getByTestId("omk-theme-light").click();
		const background = await page
			.locator("html")
			.evaluate((node) => getComputedStyle(node).backgroundColor);
		const channels = background.match(/\d+/g)?.map(Number) ?? [];
		expect(channels.length).toBeGreaterThanOrEqual(3);
		expect(channels[0]).toBe(channels[1]);
		expect(channels[1]).toBe(channels[2]);
	});
});
