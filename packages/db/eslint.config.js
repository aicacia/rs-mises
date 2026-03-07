import { fileURLToPath } from 'node:url';
import { defineConfig } from 'eslint/config';
import { createTsConfig } from '../../eslint.shared-config.js';

const gitignorePath = fileURLToPath(new URL('./.gitignore', import.meta.url));
const tsconfigRootDir = fileURLToPath(new URL('.', import.meta.url));
const eslintTsconfigPath = fileURLToPath(new URL('./tsconfig.eslint.json', import.meta.url));

export default defineConfig(
	createTsConfig({
		gitignorePath,
		tsconfigRootDir,
		rules: {
			'@typescript-eslint/no-explicit-any': 'warn',
			'@typescript-eslint/no-unused-vars': [
				'error',
				{
					argsIgnorePattern: '^_',
					varsIgnorePattern: '^_',
					caughtErrorsIgnorePattern: '^_',
					ignoreRestSiblings: true
				}
			]
		}
	}),
	{
		files: ['src/**/*.test.ts'],
		languageOptions: {
			parserOptions: {
				projectService: false,
				project: [eslintTsconfigPath],
				tsconfigRootDir
			}
		}
	}
);
