/**
 * Query builder with fluent API for composing queries
 */

import type { CTE, IQueryBuilder, OrderDirection, QueryPredicate, UnsubscribeFn } from './types.js';
import {
	addNamedCTE,
	addOrderByToCTE,
	cloneCTE,
	createCTE,
	createOrderBy,
	setLimitOnCTE,
	setOffsetOnCTE
} from './cte.js';

/**
 * QueryBuilder - fluent API for building and executing queries
 */
export class QueryBuilder<T> implements IQueryBuilder<T> {
	private _cte: CTE;
	private _predicates: QueryPredicate<T>[] = [];
	private _executeQuery: (cte: CTE, predicates: QueryPredicate<T>[]) => QuerySubscriptionResult<T>;

	constructor(
		executeQuery: (cte: CTE, predicates: QueryPredicate<T>[]) => QuerySubscriptionResult<T>
	) {
		this._cte = createCTE();
		this._executeQuery = executeQuery;
	}

	filter(predicate: QueryPredicate<T>): IQueryBuilder<T> {
		const builder = new QueryBuilder(this._executeQuery);
		builder._cte = cloneCTE(this._cte);
		builder._predicates = [...this._predicates, predicate];
		return builder;
	}

	orderBy(field: keyof T, direction: OrderDirection = 'asc'): IQueryBuilder<T> {
		const builder = new QueryBuilder(this._executeQuery);
		builder._cte = addOrderByToCTE(this._cte, createOrderBy(String(field), direction));
		builder._predicates = [...this._predicates];
		return builder;
	}

	limit(n: number): IQueryBuilder<T> {
		const builder = new QueryBuilder(this._executeQuery);
		builder._cte = setLimitOnCTE(this._cte, n);
		builder._predicates = [...this._predicates];
		return builder;
	}

	offset(n: number): IQueryBuilder<T> {
		const builder = new QueryBuilder(this._executeQuery);
		builder._cte = setOffsetOnCTE(this._cte, n);
		builder._predicates = [...this._predicates];
		return builder;
	}

	with(name: string, fn: (q: IQueryBuilder<T>) => IQueryBuilder<T>): IQueryBuilder<T> {
		const builder = new QueryBuilder(this._executeQuery);
		builder._cte = cloneCTE(this._cte);
		builder._predicates = [...this._predicates];

		// Build the subquery
		const subqueryBuilder = fn(new QueryBuilder(this._executeQuery));
		const subqueryCTE = subqueryBuilder.toCTE();

		// Add it as a named CTE
		builder._cte = addNamedCTE(builder._cte, name, subqueryCTE);

		return builder;
	}

	subscribe(onUpdate: (docs: T[]) => void, onError?: (error: Error) => void): UnsubscribeFn {
		return this._executeQuery(
			this._cte,
			this._predicates
		)({
			onUpdate,
			onError: onError || (() => {})
		});
	}

	toCTE(): CTE {
		return cloneCTE(this._cte);
	}
}

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
