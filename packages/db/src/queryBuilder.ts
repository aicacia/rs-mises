/**
 * Query builder with fluent API for composing queryable CTEs
 */

import type { UnsubscribeFn } from './types.js';
import { type CTE, type CTEFilter, addNamedCTE, createCTE } from './cte.js';
import { evaluateCTE } from './filterEngine.js';

/**
 * Order direction
 */
export type OrderDirection = 'asc' | 'desc';

/**
 * Deep key of an object type
 *
 * Generates a union of all possible paths through an object's properties,
 * including nested paths using dot notation (e.g., 'user.name', 'user.address.city').
 *
 * @example
 * ```typescript
 * interface User {
 *   id: string;
 *   profile: {
 *     name: string;
 *     age: number;
 *   };
 * }
 *
 * type UserKeys = DeepKeyOf<User>; // 'id' | 'profile' | 'profile.name' | 'profile.age'
 * ```
 */
export type DeepKeyOf<T> = T extends object
	? {
			[K in keyof T]-?: K extends string
				? `${K}` | (T[K] extends object ? `${K}.${DeepKeyOf<T[K]> & string}` : never)
				: never;
		}[keyof T]
	: never;

/**
 * Query builder interface
 */
export interface IQueryBuilder<T> {
	/**
	 * Add a filter condition to the CTE
	 */
	where(filter: CTEFilter): IQueryBuilder<T>;

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
	orderBy(field: DeepKeyOf<T> | string, direction?: OrderDirection): IQueryBuilder<T>;

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
	toCTE(): CTE;

	/**
	 * Compile CTE to executable filter function
	 */
	compileToFunction(): (docs: T[]) => T[];
}

/**
 * Query compiler - compiles CTE to executable result
 */
export type QueryCompiler<T> = (cte: CTE) => QuerySubscriptionResult<T>;

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
	private _cte: CTE;
	private _compile: QueryCompiler<T>;

	constructor(compile: QueryCompiler<T>) {
		this._cte = createCTE();
		this._compile = compile;
	}

	private cloneCurrentCTE(): CTE {
		return JSON.parse(JSON.stringify(this._cte));
	}

	where(filter: CTEFilter): IQueryBuilder<T> {
		const builder = new QueryBuilder(this._compile);
		builder._cte = this.cloneCurrentCTE();
		if (!builder._cte.filters) {
			builder._cte.filters = [];
		}
		builder._cte.filters.push(filter);
		return builder;
	}

	orderBy(field: DeepKeyOf<T> | string, direction: OrderDirection = 'asc'): IQueryBuilder<T> {
		const builder = new QueryBuilder(this._compile);
		builder._cte = this.cloneCurrentCTE();
		if (!builder._cte.orderBy) {
			builder._cte.orderBy = [];
		}
		builder._cte.orderBy.push({ field: String(field), direction });
		return builder;
	}

	limit(n: number): IQueryBuilder<T> {
		const builder = new QueryBuilder(this._compile);
		builder._cte = this.cloneCurrentCTE();
		builder._cte.limit = n;
		return builder;
	}

	offset(n: number): IQueryBuilder<T> {
		const builder = new QueryBuilder(this._compile);
		builder._cte = this.cloneCurrentCTE();
		builder._cte.offset = n;
		return builder;
	}

	paginate(page: number, pageSize: number = 10): IQueryBuilder<T> {
		const builder = new QueryBuilder(this._compile);
		builder._cte = this.cloneCurrentCTE();
		builder._cte.offset = page * pageSize;
		builder._cte.limit = pageSize;
		return builder;
	}

	with(name: string, fn: (q: IQueryBuilder<T>) => IQueryBuilder<T>): IQueryBuilder<T> {
		const builder = new QueryBuilder(this._compile);
		builder._cte = this.cloneCurrentCTE();

		const subqueryBuilder = fn(new QueryBuilder(this._compile));
		const subqueryCTE = subqueryBuilder.toCTE();

		addNamedCTE(builder._cte, name, subqueryCTE);

		return builder;
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

	toCTE(): CTE {
		return this.cloneCurrentCTE();
	}

	compileToFunction(): (docs: T[]) => T[] {
		const cte = this._cte;
		return (docs: T[]) => evaluateCTE(cte, docs);
	}
}
