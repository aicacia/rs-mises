import { fileURLToPath } from 'node:url';
import { createTsConfig } from '../../eslint.shared-config.js';

const gitignorePath = fileURLToPath(new URL('./.gitignore', import.meta.url));
const tsconfigRootDir = fileURLToPath(new URL('.', import.meta.url));

export default createTsConfig({
	gitignorePath,
	tsconfigRootDir,
	rules: { '@typescript-eslint/no-explicit-any': 'warn' }
});
