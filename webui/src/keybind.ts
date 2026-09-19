export type KeybindHandler = () => boolean | undefined;

export class Keybind {
	#callbacks = new Map<string, KeybindHandler[]>();

	constructor() {
		document.addEventListener("keydown", (e) => this.#handleKeydown(e));
	}

	#handleKeydown(e: KeyboardEvent): void {
		const key = this.#resolveEvent(e);
		if (key === null) return;
		// Claim the event only when a handler actually acts on it. A shortcut
		// layer that always calls preventDefault swallows keys other
		// components own — Escape in particular belongs to whichever dialog is
		// open, and Headless UI closes its own.
		if (!this.#emit(key)) return;
		e.preventDefault();
		e.stopPropagation();
	}

	#resolveEvent(e: KeyboardEvent): string | null {
		const ctrl = e.ctrlKey;
		const key = e.key.toLowerCase();

		if (ctrl && key === "a") return "keybind-select-all";
		if (ctrl && key === "d") return "keybind-deselect-all";
		if (ctrl && key === "f") return "keybind-search";
		if (ctrl && key === "s") return "keybind-save";
		if (key === "escape") return "keybind-esc";

		return null;
	}

	on(event: string, callback: KeybindHandler): void {
		const cbs = this.#callbacks.get(event) ?? [];
		cbs.push(callback);
		this.#callbacks.set(event, cbs);
	}

	/** @returns true when at least one handler claimed the event. */
	#emit(event: string): boolean {
		let handled = false;
		for (const callback of this.#callbacks.get(event) ?? []) {
			if (callback() === true) handled = true;
		}
		return handled;
	}
}
