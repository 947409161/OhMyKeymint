import { expect, test } from "@playwright/test";

test.describe("Switch", () => {
	test("exposes switch semantics and toggles with the keyboard", async ({
		page,
	}) => {
		await page.goto("/");
		const control = page.getByRole("switch", { name: "Raw switch" });
		await expect(control).toBeVisible();
		const before = await control.getAttribute("aria-checked");
		await control.focus();
		await page.keyboard.press("Space");
		await expect(control).not.toHaveAttribute("aria-checked", before ?? "");
	});

	test("takes its accessible name from the setting row", async ({ page }) => {
		await page.goto("/");
		await expect(
			page.getByRole("switch", { name: "Switch row" }),
		).toBeVisible();
	});
});

test.describe("Checkbox", () => {
	test("exposes checkbox semantics and toggles with the keyboard", async ({
		page,
	}) => {
		await page.goto("/");
		const control = page.getByRole("checkbox", { name: "Raw checkbox" });
		await control.focus();
		await page.keyboard.press("Space");
		await expect(control).toHaveAttribute("aria-checked", "true");
	});

	test("takes its accessible name from the setting row", async ({ page }) => {
		await page.goto("/");
		await expect(
			page.getByRole("checkbox", { name: "Checkbox row" }),
		).toBeVisible();
	});
});

test.describe("Slider", () => {
	test("steps with the arrow keys", async ({ page }) => {
		await page.goto("/");
		const slider = page.getByRole("slider", { name: "Raw slider" });
		const before = Number(await slider.getAttribute("aria-valuenow"));
		await slider.focus();
		await page.keyboard.press("ArrowRight");
		await expect
			.poll(async () => Number(await slider.getAttribute("aria-valuenow")))
			.toBeGreaterThan(before);
	});
});

test.describe("SettingRow", () => {
	test("renders a navigate row as a real button", async ({ page }) => {
		await page.goto("/");
		await expect(
			page.getByRole("button", { name: /Navigate row/ }),
		).toBeVisible();
	});

	test("keeps a disabled row inert", async ({ page }) => {
		await page.goto("/");
		await expect(
			page.getByRole("switch", { name: "Disabled row" }),
		).toBeDisabled();
	});

	test("gives every icon-only control a name", async ({ page }) => {
		await page.goto("/");
		await expect(page.getByRole("button", { name: "Browse" })).toBeVisible();
		await expect(
			page
				.getByRole("banner")
				.getByRole("button", { name: "Settings", exact: true }),
		).toBeVisible();
	});
});

test.describe("NavigationBar", () => {
	test("marks the selected item with aria-current", async ({ page }) => {
		await page.goto("/");
		const bar = page.getByRole("navigation", { name: "Main" });
		await expect(bar.getByRole("button", { name: "Home" })).toHaveAttribute(
			"aria-current",
			"page",
		);
		await bar.getByRole("button", { name: "Tools" }).click();
		await expect(bar.getByRole("button", { name: "Tools" })).toHaveAttribute(
			"aria-current",
			"page",
		);
		await expect(bar.getByRole("button", { name: "Home" })).not.toHaveAttribute(
			"aria-current",
			"page",
		);
	});
});

test.describe("Snackbar", () => {
	test("announces a transient message", async ({ page }) => {
		await page.goto("/");
		await page.getByRole("button", { name: "Show snackbar" }).click();
		await expect(page.getByText("Keybox installed.")).toBeVisible();
	});

	test("carries an error tone without hue", async ({ page }) => {
		await page.goto("/");
		await page.getByRole("button", { name: "Show error snackbar" }).click();
		const item = page.getByText("Keybox validation failed.");
		await expect(item).toBeVisible();
		const background = await item
			.locator("..")
			.evaluate((node) => getComputedStyle(node).backgroundColor);
		const channels = background.match(/\d+/g)?.map(Number) ?? [];
		expect(channels[0]).toBe(channels[1]);
		expect(channels[1]).toBe(channels[2]);
	});
});

test.describe("StatusField", () => {
	test("states the condition in text, not only in tone", async ({ page }) => {
		await page.goto("/");
		await expect(page.getByText("Revoked", { exact: true })).toBeVisible();
		await expect(
			page.getByText("Google hardware root certificate", { exact: true }),
		).toBeVisible();
		await expect(
			page.getByText("Not installed", { exact: true }),
		).toBeVisible();
	});
});
