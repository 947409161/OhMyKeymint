import { expect, test } from "@playwright/test";

test.describe("design gallery", () => {
	test("renders the component inventory", async ({ page }) => {
		await page.goto("/?gallery");
		await expect(
			page.getByRole("heading", { name: "Design gallery" }),
		).toBeVisible();
		await expect(page.getByRole("heading", { name: "Buttons" })).toBeVisible();
		await expect(
			page.getByRole("heading", { name: "Status tones" }),
		).toBeVisible();
		await expect(
			page.getByRole("heading", { name: "Navigation" }),
		).toBeVisible();
	});

	test("renders the monospace role in an actual monospace face", async ({
		page,
	}) => {
		await page.goto("/?gallery");
		const mono = page.locator(".text-omk-mono").first();
		const family = await mono.evaluate(
			(node) => getComputedStyle(node).fontFamily,
		);
		expect(family.toLowerCase()).toContain("mono");
	});

	test("wraps long status values instead of clipping them", async ({
		page,
	}) => {
		await page.goto("/?gallery");
		const value = page.getByText("Google hardware root certificate", {
			exact: true,
		});
		const box = await value.boundingBox();
		const style = await value.evaluate((node) => ({
			overflow: getComputedStyle(node).overflow,
			whiteSpace: getComputedStyle(node).whiteSpace,
		}));
		expect(box).not.toBeNull();
		expect(style.overflow).not.toBe("hidden");
	});
});
