import { createRoot } from "react-dom/client";
import { App } from "./App";
import { getBridge, setBridge } from "./bridge";
import { createDevBridge } from "./bridge/dev";
import { i18n } from "./i18n";
import "./styles/theme.css";
import { isSupported, renderBlockingPage } from "./webview/webview";

const root = document.querySelector<HTMLDivElement>("#app");

if (root === null) {
	throw new Error("OMK WebUI root element is missing");
}

if (!isSupported()) {
	root.replaceChildren(renderBlockingPage());
} else {
	// Outside a WebView there is no KernelSU bridge, so install the
	// deterministic stand-in and let the application run its real code path.
	if (!getBridge().isKsuWebui()) {
		setBridge(createDevBridge());
	}

	try {
		await i18n.init();
	} catch (error) {
		// Translations are additive; render with the keys rather than fail.
		console.error("Unable to load translations:", error);
	}

	createRoot(root).render(<App />);
}
