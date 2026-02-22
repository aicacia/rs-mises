import { fileURLToPath } from 'node:url';
import { createSvelteConfig } from '../../eslint.shared-config.js';
import { defineConfig } from 'eslint/config';
import svelteConfig from './svelte.config.js';

const gitignorePath = fileURLToPath(new URL('./.gitignore', import.meta.url));
const tsconfigRootDir = fileURLToPath(new URL('.', import.meta.url));

export default defineConfig(createSvelteConfig({ gitignorePath, tsconfigRootDir, svelteConfig }), {
	ignores: ['src/routes/.well-known/**']
});
