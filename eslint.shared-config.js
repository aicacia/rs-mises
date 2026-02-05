import prettier from 'eslint-config-prettier';
import { includeIgnoreFile } from '@eslint/compat';
import js from '@eslint/js';
import { defineConfig } from 'eslint/config';
import globals from 'globals';
import ts from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';

/**
 * Base TypeScript flat config used by non-Svelte packages.
 */
export function createTsConfig({ gitignorePath, tsconfigRootDir, rules = {} }) {
  return defineConfig(
    includeIgnoreFile(gitignorePath),
    js.configs.recommended,
    ...ts.configs.recommended,
    prettier,
    {
      languageOptions: {
        globals: { ...globals.browser, ...globals.node },
        parserOptions: {
          projectService: true,
          allowDefaultProject: true,
          tsconfigRootDir
        }
      },
      rules: {
        'no-undef': 'off',
        ...rules
      }
    },
    {
      files: ['eslint.config.js', 'rollup.config.mjs', 'vite.config.js', 'vite.config.mjs', 'svelte.config.js'],
      languageOptions: {
        parserOptions: {
          projectService: false,
          allowDefaultProject: true,
          tsconfigRootDir
        }
      }
    },
  );
}

/**
 * Base Svelte + TypeScript flat config used by Svelte apps/packages.
 */
export function createSvelteConfig({ gitignorePath, tsconfigRootDir, svelteConfig, rules = {} }) {
  return defineConfig(
    includeIgnoreFile(gitignorePath),
    js.configs.recommended,
    ...ts.configs.recommended,
    ...svelte.configs.recommended,
    prettier,
    ...svelte.configs.prettier,
    {
      languageOptions: {
        globals: { ...globals.browser, ...globals.node },
        parserOptions: {
          tsconfigRootDir,
          allowDefaultProject: true,
          projectService: true
        }
      },
      rules: {
        "@typescript-eslint/no-unused-vars": ["warn", { "argsIgnorePattern": "^_", "varsIgnorePattern": "^_" }],
        'no-undef': 'off',
        ...rules
      }
    },
    {
      files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],
      languageOptions: {
        parserOptions: {
          tsconfigRootDir,
          projectService: true,
          allowDefaultProject: true,
          extraFileExtensions: ['.svelte'],
          parser: ts.parser,
          svelteConfig
        }
      }
    },
    {
      files: ['eslint.config.js', 'rollup.config.mjs', 'vite.config.js', 'vite.config.mjs', 'svelte.config.js'],
      languageOptions: {
        parserOptions: {
          projectService: false,
          allowDefaultProject: true,
          tsconfigRootDir
        }
      }
    }
  );
}
