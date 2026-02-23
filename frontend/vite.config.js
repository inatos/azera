import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig, loadEnv } from "vite";

/**
 * @param {{ mode: string }} opts
 */
export default (opts) => {
	const { mode } = opts;
	// Load env vars
	// @ts-ignore
	process.env = { ...process.env, ...loadEnv(mode, process.cwd(), "") };

	return defineConfig({
		plugins: [sveltekit()],
		server: {
			// This is important for Docker:
			// Explicitly listen on all interfaces.
			host: "0.0.0.0",
			// @ts-ignore
			port: parseInt(process.env.VITE_SERVER_PORT) || 5173,
			watch: {
				usePolling: true // Hot reload
			},
			// Proxy API requests to backend
			proxy: {
				'/api': {
					// In development, proxy to localhost
					// In Docker, this will be replaced by SvelteKit's own handling
					target: 'http://localhost:3000',
					changeOrigin: true,
					rewrite: (path) => path
				}
			}
		},
		// Optimize monaco dependencies to prevent reloading issues
		optimizeDeps: {
			include: [
				"marked",
				"monaco",
				"monaco-editor",
				"monaco-editor/esm/vs/language/json/json.worker",
				"monaco-editor/esm/vs/language/css/css.worker",
				"monaco-editor/esm/vs/language/html/html.worker",
				"monaco-editor/esm/vs/language/typescript/ts.worker",
				"monaco-editor/esm/vs/editor/editor.worker",
				"prismjs",
				'prismjs/components/prism-javascript',
				'prismjs/components/prism-typescript',
				'prismjs/components/prism-css',
				'prismjs/components/prism-json'
			]
		}
	})
};