import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
import Icons from "unplugin-icons/vite";
import { defineConfig } from "vite";

export default defineConfig({
	base: "",
	plugins: [
		react(),
		tailwindcss(),
		Icons({ compiler: "jsx", jsx: "react", autoInstall: false }),
	],
	resolve: {
		alias: {
			"@": fileURLToPath(new URL("./src", import.meta.url)),
			"@components": fileURLToPath(new URL("./src/components", import.meta.url)),
			"@bridge": fileURLToPath(new URL("./src/bridge", import.meta.url)),
		},
	},
	build: {
		target: "chrome120",
		cssTarget: "chrome120",
		outDir: "../template/webroot",
		emptyOutDir: true,
		cssCodeSplit: false,
	},
});
