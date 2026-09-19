import { useCallback, useEffect, useState } from "react";
import { GalleryView } from "./views/GalleryView";

export const APPEARANCE_MODES = ["auto", "light", "dark", "amoled"] as const;
export type AppearanceMode = (typeof APPEARANCE_MODES)[number];

export const RADIUS_CHARACTERS = ["soft", "tight"] as const;
export type RadiusCharacter = (typeof RADIUS_CHARACTERS)[number];

const THEME_KEY = "omk-appearance";
const RADIUS_KEY = "omk-radius";

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

function readStored<T extends string>(
	key: string,
	allowed: readonly T[],
	fallback: T,
): T {
	try {
		const stored = window.localStorage.getItem(key);
		if (stored !== null && (allowed as readonly string[]).includes(stored)) {
			return stored as T;
		}
	} catch {
		// Storage is unavailable in some WebView configurations.
	}
	return fallback;
}

function persist(key: string, value: string): void {
	try {
		window.localStorage.setItem(key, value);
	} catch {
		// Persisting the preference is best effort.
	}
}

export function App(): React.JSX.Element {
	const [mode, setMode] = useState<AppearanceMode>(() =>
		readStored(THEME_KEY, APPEARANCE_MODES, "auto"),
	);
	const [radius, setRadius] = useState<RadiusCharacter>(() =>
		readStored(RADIUS_KEY, RADIUS_CHARACTERS, "soft"),
	);
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
		persist(THEME_KEY, mode);
	}, [mode, systemDark]);

	useEffect(() => {
		document.documentElement.dataset.radius = radius;
		persist(RADIUS_KEY, radius);
	}, [radius]);

	const onModeChange = useCallback((next: string) => {
		if ((APPEARANCE_MODES as readonly string[]).includes(next)) {
			setMode(next as AppearanceMode);
		}
	}, []);

	const onRadiusChange = useCallback(
		(next: RadiusCharacter) => setRadius(next),
		[],
	);

	if (import.meta.env.DEV) {
		return (
			<GalleryView
				mode={mode === "auto" ? resolveTheme(mode, systemDark) : mode}
				onModeChange={onModeChange}
				radius={radius}
				onRadiusChange={onRadiusChange}
			/>
		);
	}

	return (
		<div className="min-h-dvh bg-omk-bg p-4 text-omk-on" data-testid="omk-app">
			<h1 className="text-omk-title">Oh My Keymint</h1>
			<p
				className="mt-2 text-omk-body text-omk-muted"
				data-testid="omk-shell-status"
			>
				Views are not implemented yet.
			</p>
		</div>
	);
}
