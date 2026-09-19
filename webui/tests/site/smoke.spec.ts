import { expect, test } from "@playwright/test";

const theme = (page: import("@playwright/test").Page, value: string) =>
	page.getByRole("button", { name: `Theme: ${value}`, exact: true });

const radius = (page: import("@playwright/test").Page, value: string) =>
	page.getByRole("button", { name: `Radius: ${value}`, exact: true });

test.describe("design gallery", () => {
	test("renders the component inventory", async ({ page }) => {
		await page.goto("/");
		await expect(
			page.getByRole("heading", { name: "Design gallery" }),
		).toBeVisible();
		await expect(page.getByRole("heading", { name: "Buttons" })).toBeVisible();
		await expect(
			page.getByRole("heading", { name: "Status tones" }),
		).toBeVisible();
	});

	test("switches the theme attribute", async ({ page }) => {
		await page.goto("/");
		for (const value of ["light", "dark", "amoled"]) {
			await theme(page, value).click();
			await expect(page.locator("html")).toHaveAttribute("data-theme", value);
		}
	});

	test("marks the active theme for assistive technology", async ({ page }) => {
		await page.goto("/");
		await theme(page, "dark").click();
		await expect(theme(page, "dark")).toHaveAttribute("aria-pressed", "true");
		await expect(theme(page, "light")).toHaveAttribute("aria-pressed", "false");
	});

	test("switches the radius character", async ({ page }) => {
		await page.goto("/");
		await radius(page, "tight").click();
		await expect(page.locator("html")).toHaveAttribute("data-radius", "tight");
		await radius(page, "soft").click();
		await expect(page.locator("html")).toHaveAttribute("data-radius", "soft");
	});

	test("radius characters resolve to different computed radii", async ({
		page,
	}) => {
		await page.goto("/");
		const card = page.locator(".rounded-omk-lg").first();
		await radius(page, "soft").click();
		const soft = await card.evaluate(
			(node) => getComputedStyle(node).borderTopLeftRadius,
		);
		await radius(page, "tight").click();
		const tight = await card.evaluate(
			(node) => getComputedStyle(node).borderTopLeftRadius,
		);
		expect(soft).not.toEqual(tight);
	});

	test("persists the appearance preference across reloads", async ({
		page,
	}) => {
		await page.goto("/");
		await theme(page, "amoled").click();
		await page.reload();
		await expect(page.locator("html")).toHaveAttribute("data-theme", "amoled");
	});

	test("paints a monochrome surface", async ({ page }) => {
		await page.goto("/");
		await theme(page, "light").click();
		const background = await page
			.locator("html")
			.evaluate((node) => getComputedStyle(node).backgroundColor);
		const channels = background.match(/\d+/g)?.map(Number) ?? [];
		expect(channels.length).toBeGreaterThanOrEqual(3);
		expect(channels[0]).toBe(channels[1]);
		expect(channels[1]).toBe(channels[2]);
	});
});
