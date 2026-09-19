import { useState } from "react";
import IconApp from "~icons/material-symbols/apps";
import { cx } from "../../utils/cx";
import { Icon } from "./Icon";

export interface AppIconProps {
	packageName: string;
	size?: number;
	className?: string;
}

type IconState = "loading" | "loaded" | "error";

/**
 * App icons are served by the host through the `ksu://icon/<package>` scheme,
 * which the KernelSU WebView intercepts and the WebUI's content security
 * policy already allows. There is no bridge API for them.
 *
 * Outside that WebView — vite dev, Playwright — the request fails, so the
 * frame falls back to a glyph instead of an empty box. The frame is a fixed
 * square either way, so replacing the glyph with the icon shifts nothing.
 *
 * The icon is content, not chrome: a raster image the user recognises an app
 * by. It keeps its colour while every surface, divider and text tone around it
 * stays on the grayscale ramp — the same exemption the design standard already
 * grants to images.
 */
export function AppIcon({ packageName, size = 40, className }: AppIconProps) {
	const [state, setState] = useState<IconState>("loading");

	return (
		<span
			style={{ width: size, height: size }}
			className={cx(
				"flex shrink-0 items-center justify-center overflow-hidden rounded-omk-md bg-omk-container-high",
				className,
			)}
		>
			{state === "loaded" ? null : (
				<Icon as={IconApp} size="md" className="text-omk-muted" />
			)}
			{state === "error" ? null : (
				<img
					src={`ksu://icon/${packageName}`}
					// The adjacent row text names the app, so the image is decorative.
					alt=""
					width={size}
					height={size}
					loading="lazy"
					decoding="async"
					onLoad={() => setState("loaded")}
					onError={() => setState("error")}
					className={cx(
						"size-full object-cover",
						state === "loaded" ? "block" : "hidden",
					)}
				/>
			)}
		</span>
	);
}
