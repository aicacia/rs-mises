import { snakeCase } from './snakeCase.js';

function isObject(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function shouldSnakeCaseKey(key: string): boolean {
	return /^[a-zA-Z][a-zA-Z0-9]*$/.test(key);
}

export function snakeCaseObject(value: unknown): unknown {
	if (Array.isArray(value)) {
		return value.map((entry) => snakeCaseObject(entry));
	}

	if (!isObject(value)) {
		return value;
	}

	const result: Record<string, unknown> = {};
	for (const [key, entry] of Object.entries(value)) {
		const nextKey = shouldSnakeCaseKey(key) ? snakeCase(key) : key;
		result[nextKey] = snakeCaseObject(entry);
	}

	return result;
}