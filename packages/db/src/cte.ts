/**
 * CTE (Common Table Expression) utilities and compilation
 */

import type { CTE, CTEFilter, CTEOrderBy, QueryPredicate } from './types.js';

/**
 * Create an empty CTE
 */
export function createCTE(): CTE {
	return {
		version: '1.0'
	};
}

/**
 * Clone a CTE
 */
export function cloneCTE(cte: CTE): CTE {
	return JSON.parse(JSON.stringify(cte));
}

/**
 * Create a filter operation for equality comparison
 */
export function createEqualityFilter(field: string, value: unknown): CTEFilter {
	return {
		type: 'comparison',
		operator: 'equal',
		field,
		value
	};
}

/**
 * Create a filter operation for range comparison
 */
export function createComparisonFilter(field: string, operator: string, value: unknown): CTEFilter {
	return {
		type: 'comparison',
		operator,
		field,
		value
	};
}

/**
 * Create a logical AND filter
 */
export function createAndFilter(...filters: CTEFilter[]): CTEFilter {
	return {
		type: 'logical',
		operator: 'and',
		filters
	};
}

/**
 * Create a logical OR filter
 */
export function createOrFilter(...filters: CTEFilter[]): CTEFilter {
	return {
		type: 'logical',
		operator: 'or',
		filters
	};
}

/**
 * Create an order-by clause
 */
export function createOrderBy(field: string, direction: 'asc' | 'desc' = 'asc'): CTEOrderBy {
	return { field, direction };
}

/**
 * Add a filter to a CTE
 */
export function addFilterToCTE(cte: CTE, filter: CTEFilter): CTE {
	const updated = cloneCTE(cte);
	if (!updated.filters) {
		updated.filters = [];
	}
	updated.filters.push(filter);
	return updated;
}

/**
 * Add an order-by to a CTE
 */
export function addOrderByToCTE(cte: CTE, orderBy: CTEOrderBy): CTE {
	const updated = cloneCTE(cte);
	if (!updated.orderBy) {
		updated.orderBy = [];
	}
	updated.orderBy.push(orderBy);
	return updated;
}

/**
 * Set limit on a CTE
 */
export function setLimitOnCTE(cte: CTE, limit: number): CTE {
	const updated = cloneCTE(cte);
	updated.limit = limit;
	return updated;
}

/**
 * Set offset on a CTE
 */
export function setOffsetOnCTE(cte: CTE, offset: number): CTE {
	const updated = cloneCTE(cte);
	updated.offset = offset;
	return updated;
}

/**
 * Add a named CTE
 */
export function addNamedCTE(parent: CTE, name: string, child: CTE): CTE {
	const updated = cloneCTE(parent);
	if (!updated.ctes) {
		updated.ctes = {};
	}
	updated.ctes[name] = cloneCTE(child);
	return updated;
}

/**
 * Evaluate a filter against a document
 * Note: This is a simplified evaluation for in-memory filtering.
 * Adapters may implement more sophisticated query execution.
 */
export function evaluateFilter<T>(filter: CTEFilter, doc: T): boolean {
	if (filter.type === 'comparison') {
		if (filter.operator === 'equal') {
			const fieldValue = getFieldValue(doc, filter.field!);
			return fieldValue === filter.value;
		}
		if (filter.operator === 'notEqual') {
			const fieldValue = getFieldValue(doc, filter.field!);
			return fieldValue !== filter.value;
		}
		if (filter.operator === 'greaterThan') {
			const fieldValue = getFieldValue(doc, filter.field!);
			return (fieldValue as number) > (filter.value as number);
		}
		if (filter.operator === 'lessThan') {
			const fieldValue = getFieldValue(doc, filter.field!);
			return (fieldValue as number) < (filter.value as number);
		}
		if (filter.operator === 'greaterThanOrEqual') {
			const fieldValue = getFieldValue(doc, filter.field!);
			return (fieldValue as number) >= (filter.value as number);
		}
		if (filter.operator === 'lessThanOrEqual') {
			const fieldValue = getFieldValue(doc, filter.field!);
			return (fieldValue as number) <= (filter.value as number);
		}
		if (filter.operator === 'includes') {
			const fieldValue = getFieldValue(doc, filter.field!);
			return Array.isArray(fieldValue) && fieldValue.includes(filter.value);
		}
	}
	if (filter.type === 'logical') {
		if (filter.operator === 'and') {
			return (filter.filters || []).every((f) => evaluateFilter(f, doc));
		}
		if (filter.operator === 'or') {
			return (filter.filters || []).some((f) => evaluateFilter(f, doc));
		}
	}
	return true;
}

/**
 * Evaluate a CTE against a document array
 */
export function evaluateCTE<T>(cte: CTE, docs: T[]): T[] {
	let results = [...docs];

	// Apply filters
	if (cte.filters && cte.filters.length > 0) {
		results = results.filter((doc) => {
			return cte.filters!.every((filter) => evaluateFilter(filter, doc));
		});
	}

	// Apply ordering
	if (cte.orderBy && cte.orderBy.length > 0) {
		results.sort((a, b) => {
			for (const order of cte.orderBy!) {
				const aVal = getFieldValue(a, order.field);
				const bVal = getFieldValue(b, order.field);

				let comparison = 0;
				if (aVal !== null && aVal !== undefined && bVal !== null && bVal !== undefined) {
					const aCompare = aVal as any;
					const bCompare = bVal as any;
					if (aCompare < bCompare) comparison = -1;
					else if (aCompare > bCompare) comparison = 1;
				}

				if (comparison !== 0) {
					return order.direction === 'asc' ? comparison : -comparison;
				}
			}
			return 0;
		});
	}

	// Apply offset and limit
	if (cte.offset) {
		results = results.slice(cte.offset);
	}
	if (cte.limit) {
		results = results.slice(0, cte.limit);
	}

	return results;
}

/**
 * Get field value from a document by dot notation
 */
export function getFieldValue<T>(doc: T, field: string): unknown {
	const parts = field.split('.');
	let value: unknown = doc;

	for (const part of parts) {
		if (value === null || value === undefined) return undefined;
		value = (value as Record<string, unknown>)[part];
	}

	return value;
}

/**
 * Serialize a CTE to JSON string
 */
export function serializeCTE(cte: CTE): string {
	return JSON.stringify(cte);
}

/**
 * Deserialize a CTE from JSON string
 */
export function deserializeCTE(json: string): CTE {
	return JSON.parse(json);
}

/**
 * Check if two CTEs are equal
 */
export function areCTEsEqual(cte1: CTE, cte2: CTE): boolean {
	if (cte1 === cte2) return true;
	// Use JSON serialization for reliable deep comparison
	return JSON.stringify(cte1) === JSON.stringify(cte2);
}
