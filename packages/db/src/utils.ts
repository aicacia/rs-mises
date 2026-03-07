import type { CTEOrderBy } from './cte.js';
import { FieldPath } from './types.js';

/**
 * Compare two values for sorting.
 * Null/undefined values sort last.
 */
export function compareValues(a: unknown, b: unknown): number {
	if (a === null || a === undefined) return 1;
	if (b === null || b === undefined) return -1;

	const aComp = a as bigint | boolean | number | string;
	const bComp = b as bigint | boolean | number | string;

	if (aComp < bComp) return -1;
	if (aComp > bComp) return 1;
	return 0;
}

/**
 * Get nested field value from document using dot notation.
 */
export function getFieldValue<T>(doc: T, field: FieldPath<T>): unknown {
	const parts = field.split('.');
	let value: unknown = doc;

	for (const part of parts) {
		if (value === null || value === undefined) return undefined;
		value = (value as Record<string, unknown>)[part];
	}

	return value;
}

/**
 * Create a comparator function for sorting documents by multiple fields.
 */
export function createItemSortFunction<T>(orderBy: CTEOrderBy<T>[]): (a: T, b: T) => number {
	return (a: T, b: T) => {
		for (const order of orderBy) {
			const aVal = getFieldValue(a, order.field);
			const bVal = getFieldValue(b, order.field);

			const comparison = compareValues(aVal, bVal);
			if (comparison !== 0) {
				return order.direction === 'asc' ? comparison : -comparison;
			}
		}
		return 0;
	};
}

/**
 * Normalize unknown error to Error instance.
 */
export function toError(error: unknown): Error {
	return error instanceof Error ? error : new Error(String(error));
}
