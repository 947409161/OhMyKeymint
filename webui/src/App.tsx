import { useCallback, useEffect, useState } from "react";

export const APPEARANCE_MODES = ["auto", "light", "dark", "amoled"] as const;
export type AppearanceMode = (typeof APPEARANCE_MODES)[number];

const STORAGE_KEY = "omk-appearance";

function prefersDark(): boolean {
	return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

export function resolveTheme(
	mode: AppearanceMode,
	systemDark: boolean,
): "light" | "dark" | "amoled" {
	if (mode === "auto") return systemDark ? "dark" : "light";
	return mode;
}

function readStoredMode(): AppearanceMode {
	try {
		const stored = window.localStorage.getItem(STORAGE_KEY);
		if (
			stored !== null &&
			(APPEARANCE_MODES as readonly string[]).includes(stored)
		) {
			return stored as AppearanceMode;
		}
	} catch {
		// Storage is unavailable in some WebView configurations.
	}
	return "auto";
}

export function App(): React.JSX.Element {
	const [mode, setMode] = useState<AppearanceMode>(readStoredMode);
	const [systemDark, setSystemDark] = useState<boolean>(prefersDark);

	useEffect(() => {
		const query = window.matchMedia("(prefers-color-scheme: dark)");
		const onChange = (event: MediaQueryListEvent): void => {
			setSystemDark(event.matches);
		};
		query.addEventListener("change", onChange);
		return () => {
			query.removeEventListener("change", onChange);
		};
	}, []);

	useEffect(() => {
		document.documentElement.dataset.theme = resolveTheme(mode, systemDark);
		try {
			window.localStorage.setItem(STORAGE_KEY, mode);
		} catch {
			// Persisting the preference is best effort.
		}
	}, [mode, systemDark]);

	const onSelect = useCallback((next: AppearanceMode) => {
		setMode(next);
	}, []);

	return (
		<div className="min-h-dvh bg-omk-bg text-omk-on" data-testid="omk-app">
			<header className="border-omk-divider border-b px-4 py-3">
				<h1 className="font-semibold text-lg">Oh My Keymint</h1>
			</header>
			<main className="p-4">
				<p className="text-omk-muted text-sm" data-testid="omk-shell-status">
					React toolchain online.
				</p>
				<section aria-label="Appearance" className="mt-6">
					<h2 className="font-medium text-sm">Appearance</h2>
					<div className="mt-2 flex gap-2">
						{APPEARANCE_MODES.map((value) => (
							<button
								key={value}
								type="button"
								aria-pressed={mode === value}
								data-testid={`omk-theme-${value}`}
								className="rounded-md border border-omk-divider px-3 py-1.5 text-sm aria-pressed:bg-omk-accent aria-pressed:text-omk-on-accent"
								onClick={() => onSelect(value)}
							>
								{value}
							</button>
						))}
					</div>
				</section>
			</main>
		</div>
	);
}
