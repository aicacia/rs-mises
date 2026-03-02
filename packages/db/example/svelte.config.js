import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('svelte/config').Config} */
const config = {
	preprocess: vitePreprocess()
};

export default config;
