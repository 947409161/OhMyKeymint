import { useCallback, useEffect, useState } from "react";
import { SnackbarHost } from "./components/molecules/Snackbar";
import { GalleryView } from "./views/GalleryView";

export const APPEARANCE_MODES = ["auto", "light", "dark", "amoled"] as const;
export type AppearanceMode = (typeof APPEARANCE_MODES)[number];

const STORAGE_KEY = "omk-appearance";

function prefersDark(): boolean {
	return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

/** Maps the persisted preference onto the concrete theme attribute. */
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
		const onChange = (event: MediaQueryListEvent): void =>
			setSystemDark(event.matches);
		query.addEventListener("change", onChange);
		return () => query.removeEventListener("change", onChange);
	}, []);

	useEffect(() => {
		document.documentElement.dataset.theme = resolveTheme(mode, systemDark);
		try {
			window.localStorage.setItem(STORAGE_KEY, mode);
		} catch {
			// Persisting the preference is best effort.
		}
	}, [mode, systemDark]);

	const onAppearanceChange = useCallback((next: string) => {
		if ((APPEARANCE_MODES as readonly string[]).includes(next)) {
			setMode(next as AppearanceMode);
		}
	}, []);

	return (
		<>
			{import.meta.env.DEV ? (
				<GalleryView
					theme={resolveTheme(mode, systemDark)}
					appearance={mode}
					onAppearanceChange={onAppearanceChange}
				/>
			) : (
				<div
					className="min-h-dvh bg-omk-bg p-4 text-omk-on"
					data-testid="omk-app"
				>
					<h1 className="text-omk-title">Oh My Keymint</h1>
					<p
						className="mt-2 text-omk-body text-omk-muted"
						data-testid="omk-shell-status"
					>
						Views are not implemented yet.
					</p>
				</div>
			)}
			<SnackbarHost />
		</>
	);
}
