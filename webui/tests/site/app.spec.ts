import { expect, test } from "@playwright/test";

const nav = (page: import("@playwright/test").Page) =>
	page.getByRole("navigation", { name: "Main" });

const main = (page: import("@playwright/test").Page) => page.locator("main");

const gotoPage = async (
	page: import("@playwright/test").Page,
	label: string,
) => {
	await nav(page).getByRole("button", { name: label, exact: true }).click();
};

test.describe("home", () => {
	test("names the module in the app bar", async ({ page }) => {
		await page.goto("/");
		// The app bar names the module, not the page: the navigation bar
		// already marks which page is current.
		await expect(
			page.locator("header").getByRole("heading", { name: "Oh My Keymint" }),
		).toBeVisible();
	});

	test("renders the identity surface from the bridge", async ({ page }) => {
		await page.goto("/");
		for (const value of [
			"Google hardware root certificate",
			"StrongBox",
			"Not revoked",
			"Normal",
			"2026-09-05",
		]) {
			await expect(main(page).getByText(value, { exact: true })).toBeVisible();
		}
	});

	test("substitutes the scoped app count into the row title", async ({
		page,
	}) => {
		await page.goto("/");
		// The regression this pins: the parameterised key was used as a plain
		// label, so "%s apps selected" reached the screen verbatim.
		await expect(
			main(page).getByRole("button", { name: /\d+ apps selected/ }),
		).toBeVisible();
		await expect(page.getByText("%s")).toHaveCount(0);
	});

	test("lists the recorded activity", async ({ page }) => {
		await page.goto("/");
		await expect(main(page).getByText("Recent activity")).toBeVisible();
		await expect(main(page).getByText("Sync security patch")).toBeVisible();
	});

	test("clears the activity log and reports it", async ({ page }) => {
		await page.goto("/");
		await main(page).getByRole("button", { name: "Clear activity" }).click();
		await expect(page.getByText("Activity cleared")).toBeVisible();
	});
});

test.describe("navigation", () => {
	test("switches between the three pages", async ({ page }) => {
		await page.goto("/");
		await gotoPage(page, "Tools");
		await expect(main(page).getByText("App management")).toBeVisible();
		await gotoPage(page, "Settings");
		await expect(main(page).getByText("Language")).toBeVisible();
		await gotoPage(page, "Home");
		await expect(
			main(page).getByRole("button", { name: /apps selected/ }),
		).toBeVisible();
	});

	test("marks the current page for assistive technology", async ({ page }) => {
		await page.goto("/");
		await expect(
			nav(page).getByRole("button", { name: "Home", exact: true }),
		).toHaveAttribute("aria-current", "page");
		await gotoPage(page, "Tools");
		await expect(
			nav(page).getByRole("button", { name: "Tools", exact: true }),
		).toHaveAttribute("aria-current", "page");
	});
});

test.describe("tools", () => {
	test("offers every tool row", async ({ page }) => {
		await page.goto("/");
		await gotoPage(page, "Tools");
		for (const name of [
			"Add package names",
			"Change Keybox",
			"Soter spoofing",
			"Sync security patch",
			"Restore default security patch",
			"ADB Disabler",
		]) {
			await expect(
				main(page).getByRole("button", { name: new RegExp(name) }),
			).toBeVisible();
		}
	});

	test("applies the ADB disabler through the dialog", async ({ page }) => {
		await page.goto("/");
		await gotoPage(page, "Tools");
		await main(page)
			.getByRole("button", { name: /ADB Disabler/ })
			.click();

		const dialog = page.getByRole("dialog");
		await expect(dialog).toBeVisible();
		await dialog.getByRole("switch", { name: "Enable ADB Disabler" }).click();
		await dialog.getByRole("button", { name: "Apply" }).click();
		await expect(dialog).toBeHidden();
		await expect(page.getByText(/ADB Disabler settings applied/)).toBeVisible();
	});

	test("reads and writes the Soter spoof state", async ({ page }) => {
		await page.goto("/");
		await gotoPage(page, "Tools");
		await main(page)
			.getByRole("button", { name: /Soter spoofing/ })
			.click();

		const dialog = page.getByRole("dialog");
		await expect(dialog).toBeVisible();
		const toggle = dialog.getByRole("switch");
		await expect(toggle).toBeVisible();
		await toggle.click();
		await expect(
			page.getByText("Soter spoofing enabled. Reboot to apply."),
		).toBeVisible();
	});
});

test.describe("settings", () => {
	test("changes the theme from the appearance picker", async ({ page }) => {
		await page.goto("/");
		await gotoPage(page, "Settings");
		await main(page)
			.getByRole("button", { name: /^Appearance/ })
			.click();

		const dialog = page.getByRole("dialog");
		await dialog.getByRole("button", { name: "Pure black" }).click();
		await expect(page.locator("html")).toHaveAttribute("data-theme", "amoled");
	});

	test("offers a language picker", async ({ page }) => {
		await page.goto("/");
		await gotoPage(page, "Settings");
		await main(page)
			.getByRole("button", { name: /^Language/ })
			.click();
		const dialog = page.getByRole("dialog");
		await expect(
			dialog.getByRole("button", { name: "Follow system" }),
		).toBeVisible();
		await expect(dialog.getByRole("button", { name: "日本語" })).toBeVisible();
	});

	// Regression: refactoring SettingRow into a text-only subcomponent dropped
	// the trailing slot, so the slider silently disappeared from the row. A
	// prop that exists but is never rendered is invisible to the type checker.
	test("renders the trailing slot of a setting row", async ({ page }) => {
		await page.goto("/");
		await gotoPage(page, "Settings");
		const slider = main(page).getByRole("slider", { name: "Interface scale" });
		await expect(slider).toBeVisible();
		const width = await slider.evaluate((thumb) =>
			Math.round(
				thumb.parentElement?.parentElement?.getBoundingClientRect().width ?? 0,
			),
		);
		expect(width).toBeGreaterThan(100);
	});

	test("adjusts the interface scale", async ({ page }) => {
		await page.goto("/");
		await gotoPage(page, "Settings");
		const slider = main(page).getByRole("slider", { name: "Interface scale" });
		const before = Number(await slider.getAttribute("aria-valuenow"));
		await slider.focus();
		await page.keyboard.press("ArrowLeft");
		await expect
			.poll(async () => Number(await slider.getAttribute("aria-valuenow")))
			.toBeLessThan(before);
	});
});
