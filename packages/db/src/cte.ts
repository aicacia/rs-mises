import { FieldPath } from './types.js';

export interface CTE<T> {
	version: '1.0';
	name?: string;
	columns?: string[];
	filters?: CTEFilter<T>[];
	orderBy?: CTEOrderBy<T>[];
	limit?: number;
	offset?: number;
	ctes?: Record<string, CTE<T>>;
}

export type CTEFilter<T> = CTEComparisonFilter<T> | CTELogicalFilter<T> | CTEReferenceFilter<T>;

export interface CTEComparisonFilter<T> {
	type: 'comparison';
	operator: CTEComparisonOperator;
	field: FieldPath<T>;
	value: unknown;
}

export interface CTEReferenceFilter<T> {
	type: 'reference';
	operator: 'in' | 'notIn';
	cteName: string;
	field?: FieldPath<T>;
}

export type CTEComparisonOperator =
	| 'equal'
	| 'notEqual'
	| 'greaterThan'
	| 'lessThan'
	| 'greaterThanOrEqual'
	| 'lessThanOrEqual'
	| 'in'
	| 'contains'
	| 'containsIgnoreCase'
	| 'fuzzyContains'
	| 'includes';

export interface CTELogicalFilter<T> {
	type: 'logical';
	operator: 'and' | 'or';
	filters: CTEFilter<T>[];
}

export interface CTEOrderBy<T> {
	field: FieldPath<T>;
	direction: 'asc' | 'desc';
}

export function createCTE<T>(): CTE<T> {
	return {
		version: '1.0'
	};
}

export function compare<T>(
	field: FieldPath<T>,
	operator: CTEComparisonOperator,
	value: unknown
): CTEComparisonFilter<T> {
	return {
		type: 'comparison',
		operator,
		field,
		value
	};
}

export function equal<T>(field: FieldPath<T>, value: unknown): CTEComparisonFilter<T> {
	return compare(field, 'equal', value);
}

export function notEqual<T>(field: FieldPath<T>, value: unknown): CTEComparisonFilter<T> {
	return compare(field, 'notEqual', value);
}

export function greaterThan<T>(field: FieldPath<T>, value: unknown): CTEComparisonFilter<T> {
	return compare(field, 'greaterThan', value);
}

export function lessThan<T>(field: FieldPath<T>, value: unknown): CTEComparisonFilter<T> {
	return compare(field, 'lessThan', value);
}

export function greaterThanOrEqual<T>(field: FieldPath<T>, value: unknown): CTEComparisonFilter<T> {
	return compare(field, 'greaterThanOrEqual', value);
}

export function lessThanOrEqual<T>(field: FieldPath<T>, value: unknown): CTEComparisonFilter<T> {
	return compare(field, 'lessThanOrEqual', value);
}

export function inOperator<T>(field: FieldPath<T>, value: unknown): CTEComparisonFilter<T> {
	return compare(field, 'in', value);
}

export function contains<T>(field: FieldPath<T>, value: unknown): CTEComparisonFilter<T> {
	return compare(field, 'contains', value);
}

export function containsIgnoreCase<T>(field: FieldPath<T>, value: unknown): CTEComparisonFilter<T> {
	return compare(field, 'containsIgnoreCase', value);
}

export function fuzzyContains<T>(field: FieldPath<T>, value: unknown): CTEComparisonFilter<T> {
	return compare(field, 'fuzzyContains', value);
}

export function includes<T>(field: FieldPath<T>, value: unknown): CTEComparisonFilter<T> {
	return compare(field, 'includes', value);
}

export function inCTE<T>(cteName: string, field?: FieldPath<T>): CTEReferenceFilter<T> {
	return {
		type: 'reference',
		operator: 'in',
		cteName,
		field
	};
}

export function notInCTE<T>(cteName: string, field?: FieldPath<T>): CTEReferenceFilter<T> {
	return {
		type: 'reference',
		operator: 'notIn',
		cteName,
		field
	};
}

export function and<T>(...filters: CTEFilter<T>[]): CTELogicalFilter<T> {
	return {
		type: 'logical',
		operator: 'and',
		filters
	};
}

export function or<T>(...filters: CTEFilter<T>[]): CTELogicalFilter<T> {
	return {
		type: 'logical',
		operator: 'or',
		filters
	};
}
