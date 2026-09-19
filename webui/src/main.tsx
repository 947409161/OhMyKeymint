import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./styles/theme.css";
import { isSupported, renderBlockingPage } from "./webview/webview";

const root = document.querySelector<HTMLDivElement>("#app");

if (root === null) {
	throw new Error("OMK WebUI root element is missing");
}

if (isSupported()) {
	createRoot(root).render(<App />);
} else {
	root.replaceChildren(renderBlockingPage());
}
