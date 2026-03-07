import { fuzzyEquals } from '@aicacia/string-fuzzy_equals';
import type { CTE, CTEFilter, CTEOrderBy } from './cte.js';
import { createItemSortFunction, getFieldValue } from './utils.js';

function toSearchString(value: unknown): string {
	if (value === null || value === undefined) {
		return '';
	}

	return String(value);
}

function fuzzyContainsMatch(value: unknown, search: unknown): boolean {
	const haystack = toSearchString(value).toLowerCase();
	const needle = toSearchString(search).toLowerCase();

	if (needle.length === 0) {
		return true;
	}

	if (haystack.includes(needle)) {
		return true;
	}

	return fuzzyEquals(needle, haystack, false);
}

export interface CTEContext<T> {
	results: Record<string, T[]>;
}

function createCTEContext<T>(): CTEContext<T> {
	return {
		results: {}
	};
}

function applyNestedCTEs<T>(cte: CTE<T>, docs: T[], context: CTEContext<T>): void {
	if (!cte.ctes) {
		return;
	}

	for (const [name, nestedCTE] of Object.entries(cte.ctes)) {
		applyNestedCTEs(nestedCTE, docs, context);
		const nestedResults = applyFiltersWithContext(docs, nestedCTE, context);
		context.results[name] = nestedResults;
	}
}

export function applyFilter<T>(filter: CTEFilter<T>, doc: T, context?: CTEContext<T>): boolean {
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
			case 'containsIgnoreCase':
				return toSearchString(fieldValue)
					.toLowerCase()
					.includes(toSearchString(filter.value).toLowerCase());
			case 'fuzzyContains':
				return fuzzyContainsMatch(fieldValue, filter.value);
			case 'includes':
				return Array.isArray(fieldValue) && fieldValue.includes(filter.value);
		}
	}

	if (filter.type === 'logical') {
		const subResults = filter.filters.map((f) => applyFilter(f, doc, context));

		if (filter.operator === 'and') {
			return subResults.every((r) => r);
		}
		if (filter.operator === 'or') {
			return subResults.some((r) => r);
		}
	}

	if (filter.type === 'reference') {
		if (!context) {
			return true;
		}

		const cteResults = context.results[filter.cteName];
		if (!cteResults) {
			return filter.operator === 'notIn';
		}

		if (filter.field) {
			const fieldValue = getFieldValue(doc, filter.field);
			const existsInCTE = Array.from(cteResults).some(
				(cteDoc) => getFieldValue(cteDoc, filter.field!) === fieldValue
			);
			return filter.operator === 'in' ? existsInCTE : !existsInCTE;
		} else {
			const existsInCTE = cteResults.includes(doc);
			return filter.operator === 'in' ? existsInCTE : !existsInCTE;
		}
	}

	return true;
}

function applyFiltersWithContext<T>(docs: T[], cte: CTE<T>, context: CTEContext<T>): T[] {
	let results = docs;

	if (cte.filters && cte.filters.length > 0) {
		const filtered: T[] = [];

		for (const doc of results) {
			let passesAllFilters = true;

			for (const filter of cte.filters) {
				if (!applyFilter(filter, doc, context)) {
					passesAllFilters = false;
					break;
				}
			}

			if (passesAllFilters) {
				filtered.push(doc);
			}
		}
		results = filtered;
	}

	return results;
}

export function applyFilters<T>(docs: T[], cte: CTE<T>): T[] {
	let results = docs;

	if (cte.filters && cte.filters.length > 0) {
		const filtered: T[] = [];

		for (const doc of results) {
			let passesAllFilters = true;

			for (const filter of cte.filters) {
				if (!applyFilter(filter, doc)) {
					passesAllFilters = false;
					break;
				}
			}

			if (passesAllFilters) {
				filtered.push(doc);
			}
		}
		results = filtered;
	}

	return results;
}

/** Apply ordering to documents based on CTEOrderBy criteria. NOTE: mutates array */
export function applyOrderBy<T>(docs: T[], orderBy: CTEOrderBy<T>[]): T[] {
	return docs.sort(createItemSortFunction(orderBy));
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

export function applyCTE<T>(cte: CTE<T>, docs: T[]): T[] {
	let results = docs;

	const context = createCTEContext<T>();
	applyNestedCTEs(cte, docs, context);

	results = applyFiltersWithContext(results, cte, context);

	if (cte.orderBy && cte.orderBy.length > 0) {
		applyOrderBy(results, cte.orderBy);
	}

	results = applyPagination(results, cte.offset, cte.limit);

	return results;
}
