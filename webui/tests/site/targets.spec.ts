import { expect, test } from "@playwright/test";

const nav = (page: import("@playwright/test").Page) =>
	page.getByRole("navigation", { name: "Main" });

const main = (page: import("@playwright/test").Page) => page.locator("main");

const openTargets = async (page: import("@playwright/test").Page) => {
	await page.goto("/");
	await main(page)
		.getByRole("button", { name: /apps selected/ })
		.click();
	await expect(page.getByTestId("omk-targets")).toBeVisible();
};

const row = (page: import("@playwright/test").Page, name: string) =>
	page.getByRole("checkbox", { name });

test.describe("scoped apps", () => {
	test("opens from the home row", async ({ page }) => {
		await openTargets(page);
		await expect(page.getByRole("checkbox").first()).toBeVisible();
	});

	test("opens from the tools row", async ({ page }) => {
		await page.goto("/");
		await nav(page).getByRole("button", { name: "Tools", exact: true }).click();
		await main(page)
			.getByRole("button", { name: /Add package names/ })
			.click();
		await expect(page.getByTestId("omk-targets")).toBeVisible();
	});

	// Regression: the row was a <label> that both wrapped the control and
	// pointed at it with htmlFor, so the activation was forwarded twice and the
	// value returned to where it started.
	test("toggles a selection exactly once per click", async ({ page }) => {
		await openTargets(page);
		const target = row(page, "Google Play services");
		await expect(target).toHaveAttribute("aria-checked", "true");

		await target.click();
		await expect(target).toHaveAttribute("aria-checked", "false");
		await target.click();
		await expect(target).toHaveAttribute("aria-checked", "true");
	});

	test("keeps the save count in step with the selection", async ({ page }) => {
		await openTargets(page);
		const save = page.getByRole("button", { name: /^Save/ });
		await expect(save).toHaveText(/Save \(2\)/);

		await row(page, "Google Play services").click();
		await expect(save).toHaveText(/Save \(1\)/);
	});

	test("requests each app icon through the host scheme", async ({ page }) => {
		await openTargets(page);
		const icons = page.locator('img[src^="ksu://icon/"]');
		await expect(icons).toHaveCount(6);
		await expect(icons.first()).toHaveAttribute(
			"src",
			"ksu://icon/com.google.android.gms",
		);
	});

	// Outside a WebView ksu:// never resolves. Every frame must still paint, so
	// the fallback is what the browser and the specs always exercise.
	test("falls back to a glyph when the host cannot serve the icon", async ({
		page,
	}) => {
		await openTargets(page);
		const hasGlyph = await page
			.locator('img[src^="ksu://icon/"]')
			.first()
			.evaluate((img) => img.parentElement?.querySelector("svg") !== null);
		expect(hasGlyph).toBe(true);
	});

	// Regression: the image was hidden with `display: none` until it loaded,
	// and a browser does not fetch an image that is not being rendered — so the
	// icon never arrived, its load event never fired, and the glyph was the
	// only thing the frame could ever show.
	test("keeps the icon image rendered while it loads", async ({ page }) => {
		await openTargets(page);
		await expect(page.locator('img[src^="ksu://icon/"]').first()).toBeVisible();
	});

	test("keeps the icon frame square so the row cannot shift", async ({
		page,
	}) => {
		await openTargets(page);
		const frame = await page
			.locator('img[src^="ksu://icon/"]')
			.first()
			.evaluate((img) => {
				const box = img.parentElement?.getBoundingClientRect();
				return {
					width: Math.round(box?.width ?? 0),
					height: Math.round(box?.height ?? 0),
				};
			});
		expect(frame.width).toBeGreaterThan(0);
		expect(frame.width).toBe(frame.height);
	});

	test("filters by search and by selection", async ({ page }) => {
		await openTargets(page);

		await page.getByRole("searchbox").fill("Key Attestation");
		await expect(page.getByRole("checkbox")).toHaveCount(1);

		await page.getByRole("searchbox").fill("");
		await page.getByRole("button", { name: "Not selected" }).click();
		await expect(page.getByRole("checkbox")).toHaveCount(4);
		await expect(row(page, "Banking App")).toBeVisible();
	});

	test("selects and clears every visible app", async ({ page }) => {
		await openTargets(page);
		await page.getByRole("button", { name: "Select all", exact: true }).click();
		await expect(page.getByRole("button", { name: /^Save/ })).toHaveText(
			/Save \(6\)/,
		);

		await page.getByRole("button", { name: "Deselect all" }).click();
		await expect(page.getByRole("button", { name: /^Save/ })).toHaveText(
			/Save \(0\)/,
		);
	});

	test("saves and reports the result", async ({ page }) => {
		await openTargets(page);
		await row(page, "Banking App").click();
		await page.getByRole("button", { name: /^Save/ }).click();

		await expect(page.getByText("Config saved")).toBeVisible();
		await expect(page.getByTestId("omk-targets")).toBeHidden();
	});
});

test.describe("back gestures", () => {
	// The host intercepts the Android back gesture and hands it to the page,
	// so overlays live on the browser history stack.
	test("the back gesture closes the scoped apps view", async ({ page }) => {
		await openTargets(page);
		await page.goBack();
		await expect(page.getByTestId("omk-targets")).toBeHidden();
		await expect(
			main(page).getByRole("button", { name: /apps selected/ }),
		).toBeVisible();
	});

	test("Escape closes the scoped apps view", async ({ page }) => {
		await openTargets(page);
		await page.keyboard.press("Escape");
		await expect(page.getByTestId("omk-targets")).toBeHidden();
	});

	test("Escape closes one layer at a time", async ({ page }) => {
		await openTargets(page);
		await page.getByRole("button", { name: /Add System App/ }).click();
		await expect(page.getByRole("dialog")).toBeVisible();

		// Headless UI owns this Escape; the scoped apps view must stay open.
		await page.keyboard.press("Escape");
		await expect(page.getByRole("dialog")).toBeHidden();
		await expect(page.getByTestId("omk-targets")).toBeVisible();

		await page.keyboard.press("Escape");
		await expect(page.getByTestId("omk-targets")).toBeHidden();
	});

	test("the back gesture closes a dialog one layer at a time", async ({
		page,
	}) => {
		await page.goto("/");
		await nav(page).getByRole("button", { name: "Tools", exact: true }).click();
		await main(page)
			.getByRole("button", { name: /ADB Disabler/ })
			.click();
		await expect(page.getByRole("dialog")).toBeVisible();

		await page.goBack();
		await expect(page.getByRole("dialog")).toBeHidden();
		await expect(
			main(page).getByRole("button", { name: /ADB Disabler/ }),
		).toBeVisible();
	});
});
