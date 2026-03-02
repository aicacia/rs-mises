import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import path from 'node:path';

export default defineConfig({
	root: '.',
	build: {
		outDir: 'dist',
		emptyOutDir: true
	},
	plugins: [svelte()],
	resolve: {
		alias: {
			'@': path.resolve(path.dirname(new URL(import.meta.url).pathname), './src')
		}
	},
	server: {
		open: '/index.html'
	}
});
