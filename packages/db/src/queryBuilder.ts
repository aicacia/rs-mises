/**
 * Query builder with fluent API for composing queryable CTEs
 */

import type { FieldPath, UnsubscribeFn } from './types.js';
import {
	type CTE,
	type CTEComparisonOperator,
	type CTEFilter,
	and as createAndFilter,
	compare,
	contains,
	containsIgnoreCase,
	createCTE,
	equal,
	fuzzyContains,
	greaterThan,
	greaterThanOrEqual,
	inOperator,
	includes,
	lessThan,
	lessThanOrEqual,
	notEqual,
	or as createOrFilter
} from './cte.js';
import { applyCTE } from './filterEngine.js';

/**
 * Order direction
 */
export type OrderDirection = 'asc' | 'desc';

/**
 * Query builder interface
 */
export interface IQueryBuilder<T> {
	/**
	 * Add a filter condition to the CTE
	 */
	where(filter: CTEFilter<T>): IQueryBuilder<T>;

	/**
	 * Add a comparison filter with full operator control.
	 */
	compare(field: FieldPath<T>, operator: CTEComparisonOperator, value: unknown): IQueryBuilder<T>;

	/**
	 * Add equality filter.
	 */
	equal(field: FieldPath<T>, value: unknown): IQueryBuilder<T>;

	/**
	 * Add not-equal filter.
	 */
	notEqual(field: FieldPath<T>, value: unknown): IQueryBuilder<T>;

	/**
	 * Add greater-than filter.
	 */
	greaterThan(field: FieldPath<T>, value: unknown): IQueryBuilder<T>;

	/**
	 * Add less-than filter.
	 */
	lessThan(field: FieldPath<T>, value: unknown): IQueryBuilder<T>;

	/**
	 * Add greater-than-or-equal filter.
	 */
	greaterThanOrEqual(field: FieldPath<T>, value: unknown): IQueryBuilder<T>;

	/**
	 * Add less-than-or-equal filter.
	 */
	lessThanOrEqual(field: FieldPath<T>, value: unknown): IQueryBuilder<T>;

	/**
	 * Add inclusion filter.
	 */
	in(field: FieldPath<T>, value: unknown): IQueryBuilder<T>;

	/**
	 * Add contains filter.
	 */
	contains(field: FieldPath<T>, value: unknown): IQueryBuilder<T>;

	/**
	 * Add case-insensitive contains filter.
	 */
	containsIgnoreCase(field: FieldPath<T>, value: unknown): IQueryBuilder<T>;

	/**
	 * Add fuzzy contains filter.
	 */
	fuzzyContains(field: FieldPath<T>, value: unknown): IQueryBuilder<T>;

	/**
	 * Add includes filter.
	 */
	includes(field: FieldPath<T>, value: unknown): IQueryBuilder<T>;

	/**
	 * Add logical AND filter over provided filters.
	 */
	and(...filters: CTEFilter<T>[]): IQueryBuilder<T>;

	/**
	 * Add logical OR filter over provided filters.
	 */
	or(...filters: CTEFilter<T>[]): IQueryBuilder<T>;

	/**
	 * Order results by field
	 *
	 * Supports both top-level fields and nested paths using dot notation.
	 *
	 * @param field - Field name or nested path (e.g., 'name' or 'user.profile.name')
	 * @param direction - Sort direction ('asc' or 'desc', default: 'asc')
	 *
	 * @example
	 * ```typescript
	 * collection.query().orderBy('name', 'asc')
	 * collection.query().orderBy('user.profile.age', 'desc')
	 * ```
	 */
	orderBy(field: FieldPath<T>, direction?: OrderDirection): IQueryBuilder<T>;

	/**
	 * Limit number of results
	 */
	limit(n: number): IQueryBuilder<T>;

	/**
	 * Skip first n results
	 */
	offset(n: number): IQueryBuilder<T>;

	/**
	 * Paginate results by page number and page size
	 *
	 * @param page - Page number (0-indexed)
	 * @param pageSize - Number of results per page (default: 10)
	 * @returns Query builder with offset and limit applied
	 */
	paginate(page: number, pageSize?: number): IQueryBuilder<T>;

	/**
	 * Define a reusable CTE subquery
	 */
	with(name: string, fn: (q: IQueryBuilder<T>) => IQueryBuilder<T>): IQueryBuilder<T>;

	/**
	 * Subscribe to query results
	 *
	 * Establishes a subscription to documents matching the query. The subscription
	 * begins immediately and will emit results as they change. Errors thrown in the
	 * onUpdate callback are caught and passed to onError if provided.
	 *
	 * @param onUpdate - Called with documents matching the query.
	 *                   Errors thrown here are caught and sent to onError.
	 * @param onError - Optional callback for errors. Called with adapter errors,
	 *                  filter evaluation errors, or subscriber callback errors.
	 *                  Non-recoverable: unsubscribe is recommended.
	 *                  To retry, create a new subscription.
	 * @returns Unsubscribe function to clean up subscription and stop receiving updates
	 */
	subscribe(onUpdate: (docs: T[]) => void, onError?: (error: Error) => void): UnsubscribeFn;

	/**
	 * Export query as JSON-serializable CTE
	 */
	toCTE(): CTE<T>;

	/**
	 * Compile CTE to executable filter function
	 */
	compileToFunction(): (docs: T[]) => T[];
}

/**
 * Query compiler - compiles CTE to executable result
 */
export type QueryCompiler<T> = (cte: CTE<T>) => QuerySubscriptionResult<T>;

/**
 * Query result subscription callback
 */
export interface QuerySubscriptionCallback<T> {
	onUpdate: (docs: T[]) => void;
	onError: (error: Error) => void;
}

/**
 * Query subscription result function
 */
export type QuerySubscriptionResult<T> = (callbacks: QuerySubscriptionCallback<T>) => UnsubscribeFn;

/**
 * QueryBuilder - fluent API for building JSON-serializable CTEs that can be compiled
 */
export class QueryBuilder<T> implements IQueryBuilder<T> {
	private _cte: CTE<T>;
	private _compile: QueryCompiler<T>;

	constructor(compile: QueryCompiler<T>) {
		this._cte = createCTE();
		this._compile = compile;
	}

	where(filter: CTEFilter<T>): IQueryBuilder<T> {
		if (!this._cte.filters) {
			this._cte.filters = [];
		}
		this._cte.filters.push(filter);
		return this;
	}

	compare(field: FieldPath<T>, operator: CTEComparisonOperator, value: unknown): IQueryBuilder<T> {
		return this.where(compare(field, operator, value));
	}

	equal(field: FieldPath<T>, value: unknown): IQueryBuilder<T> {
		return this.where(equal(field, value));
	}

	notEqual(field: FieldPath<T>, value: unknown): IQueryBuilder<T> {
		return this.where(notEqual(field, value));
	}

	greaterThan(field: FieldPath<T>, value: unknown): IQueryBuilder<T> {
		return this.where(greaterThan(field, value));
	}

	lessThan(field: FieldPath<T>, value: unknown): IQueryBuilder<T> {
		return this.where(lessThan(field, value));
	}

	greaterThanOrEqual(field: FieldPath<T>, value: unknown): IQueryBuilder<T> {
		return this.where(greaterThanOrEqual(field, value));
	}

	lessThanOrEqual(field: FieldPath<T>, value: unknown): IQueryBuilder<T> {
		return this.where(lessThanOrEqual(field, value));
	}

	in(field: FieldPath<T>, value: unknown): IQueryBuilder<T> {
		return this.where(inOperator(field, value));
	}

	contains(field: FieldPath<T>, value: unknown): IQueryBuilder<T> {
		return this.where(contains(field, value));
	}

	containsIgnoreCase(field: FieldPath<T>, value: unknown): IQueryBuilder<T> {
		return this.where(containsIgnoreCase(field, value));
	}

	fuzzyContains(field: FieldPath<T>, value: unknown): IQueryBuilder<T> {
		return this.where(fuzzyContains(field, value));
	}

	includes(field: FieldPath<T>, value: unknown): IQueryBuilder<T> {
		return this.where(includes(field, value));
	}

	and(...filters: CTEFilter<T>[]): IQueryBuilder<T> {
		return this.where(createAndFilter(...filters));
	}

	or(...filters: CTEFilter<T>[]): IQueryBuilder<T> {
		return this.where(createOrFilter(...filters));
	}

	orderBy(field: FieldPath<T>, direction: OrderDirection = 'asc'): IQueryBuilder<T> {
		if (!this._cte.orderBy) {
			this._cte.orderBy = [];
		}
		this._cte.orderBy.push({ field: field, direction });
		return this;
	}

	limit(n: number): IQueryBuilder<T> {
		this._cte.limit = n;
		return this;
	}

	offset(n: number): IQueryBuilder<T> {
		this._cte.offset = n;
		return this;
	}

	paginate(page: number, pageSize: number = 10): IQueryBuilder<T> {
		this._cte.offset = page * pageSize;
		this._cte.limit = pageSize;
		return this;
	}

	with(name: string, fn: (q: IQueryBuilder<T>) => IQueryBuilder<T>): IQueryBuilder<T> {
		const subqueryBuilder = fn(new QueryBuilder(this._compile));
		const subqueryCTE = subqueryBuilder.toCTE();

		if (!this._cte.ctes) {
			this._cte.ctes = {};
		}
		this._cte.ctes[name] = subqueryCTE;

		return this;
	}

	subscribe(onUpdate: (docs: T[]) => void, onError?: (error: Error) => void): UnsubscribeFn {
		const errorHandler =
			onError ||
			((error: Error) => {
				throw error;
			});

		return this._compile(this._cte)({
			onUpdate,
			onError: errorHandler
		});
	}

	toCTE(): CTE<T> {
		return this._cte;
	}

	compileToFunction(): (docs: T[]) => T[] {
		const cte = this._cte;
		return (docs: T[]) => applyCTE(cte, docs);
	}
}
