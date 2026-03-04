import type { CTE, CTEFilter, CTEOrderBy } from './cte.js';
import { compareValues, createDocComparator, getFieldValue } from './utils.js';

export { getFieldValue };

export function evaluateFilter<T>(filter: CTEFilter, doc: T): boolean {
	if (filter.type === 'comparison') {
		const fieldValue = getFieldValue(doc, filter.field);

		switch (filter.operator) {
			case 'equal':
				return fieldValue === filter.value;
			case 'notEqual':
				return fieldValue !== filter.value;
			case 'greaterThan':
				return (fieldValue as number) > (filter.value as number);
			case 'lessThan':
				return (fieldValue as number) < (filter.value as number);
			case 'greaterThanOrEqual':
				return (fieldValue as number) >= (filter.value as number);
			case 'lessThanOrEqual':
				return (fieldValue as number) <= (filter.value as number);
			case 'in':
				return (filter.value as unknown[]).includes(fieldValue);
			case 'contains':
				return String(fieldValue).includes(String(filter.value));
			case 'includes':
				return Array.isArray(fieldValue) && fieldValue.includes(filter.value);
		}
	}

	if (filter.type === 'logical') {
		const subResults = filter.filters.map((f) => evaluateFilter(f, doc));

		if (filter.operator === 'and') {
			return subResults.every((r) => r);
		}
		if (filter.operator === 'or') {
			return subResults.some((r) => r);
		}
	}

	return true;
}

export function applyFilters<T>(docs: T[], cte: CTE): T[] {
	let results = [...docs];

	if (cte.filters && cte.filters.length > 0) {
		results = results.filter((doc) => {
			return cte.filters!.every((f) => evaluateFilter(f, doc));
		});
	}

	return results;
}

export function applyOrderBy<T>(docs: T[], orderBy: CTEOrderBy[]): T[] {
	return [...docs].sort(createDocComparator(orderBy));
}

export function applyPagination<T>(docs: T[], offset?: number, limit?: number): T[] {
	let results = docs;

	if (offset !== undefined && offset > 0) {
		results = results.slice(offset);
	}

	if (limit !== undefined && limit > 0) {
		results = results.slice(0, limit);
	}

	return results;
}

export function evaluateCTE<T>(cte: CTE, docs: T[]): T[] {
	let results = docs;

	results = applyFilters(results, cte);

	if (cte.orderBy && cte.orderBy.length > 0) {
		results = applyOrderBy(results, cte.orderBy);
	}

	results = applyPagination(results, cte.offset, cte.limit);

	return results;
}
