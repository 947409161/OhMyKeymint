import { useState } from "react";
import IconApp from "~icons/material-symbols/apps";
import { cx } from "../../utils/cx";
import { Icon } from "./Icon";

export interface AppIconProps {
	packageName: string;
	size?: number;
	className?: string;
}

type IconState = "loading" | "loaded";

/**
 * App icons are served by the host through the `ksu://icon/<package>` scheme,
 * which the KernelSU WebView intercepts and the WebUI's content security
 * policy already allows. There is no bridge API for them.
 *
 * Outside that WebView — vite dev, Playwright — the request fails, so the
 * frame falls back to a glyph instead of an empty box. The frame is a fixed
 * square either way, so replacing the glyph with the icon shifts nothing.
 *
 * The image is laid over the glyph rather than hidden until it arrives.
 * Browsers do not fetch an image that is not being rendered, so an image that
 * is `display: none` while it loads never loads, its load event never arrives,
 * and the glyph becomes permanent — on a device that serves every icon. The
 * image therefore stays in the frame whether it arrives or not: an empty `alt`
 * on a failed request paints nothing, so the glyph behind it is what shows.
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
				"relative flex shrink-0 items-center justify-center overflow-hidden rounded-omk-md bg-omk-container-high",
				className,
			)}
		>
			{state === "loaded" ? null : (
				<Icon as={IconApp} size="md" className="text-omk-muted" />
			)}
			<img
				src={`ksu://icon/${packageName}`}
				// The adjacent row text names the app, so the image is decorative.
				alt=""
				width={size}
				height={size}
				loading="lazy"
				decoding="async"
				onLoad={() => setState("loaded")}
				// Positioned over the glyph so the frame keeps its size and the
				// image stays rendered — and therefore keeps loading — while it
				// is still on its way.
				className="absolute inset-0 size-full object-cover"
			/>
		</span>
	);
}
