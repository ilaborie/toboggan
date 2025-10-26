import { defineConfig } from "vite";

export default defineConfig({
	root: ".",
	publicDir: "assets",
	build: {
		outDir: "dist",
		rollupOptions: {
			input: {
				main: "./index.html",
			},
		},
	},
	server: {
		port: 8000,
		proxy: {
			"/public": "http://localhost:8080",
		},
	},
});
