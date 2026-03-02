/**
 * Collection<T> - self-contained collection with CRUD and query operations
 */

import type {
	AdapterStatus,
	CollectionConfig,
	CTE,
	ICollection,
	IQueryBuilder,
	QueryPredicate,
	SourceAdapter,
	UnsubscribeFn
} from './types.js';
import { QueryBuilder, type QuerySubscriptionResult } from './queryBuilder.js';
import { filterDocumentsByPredicates } from './d2ts.js';

interface ActiveSubscription<T> {
	cte: CTE;
	predicates: QueryPredicate<T>[];
	count: number;
	unsubscribe: UnsubscribeFn;
	adapterSubscriptionCreated: boolean;
	lastResults: T[] | null;
}

/**
 * Collection - represents a collection of documents with CRUD and query operations
 */
export class Collection<T> implements ICollection<T> {
	readonly id: string;
	private _source: SourceAdapter<T>;
	private _keyOf: (doc: T) => string;
	private _activeSubscriptions: Map<string, ActiveSubscription<T>> = new Map();
	private _callbacks: Map<string, Set<(docs: T[]) => void>> = new Map();
	private _errorCallbacks: Map<string, Set<(error: Error) => void>> = new Map();

	constructor(config: CollectionConfig<T>) {
		this.id = config.id;
		this._source = config.source;
		this._keyOf = config.keyOf;
	}

	async create(doc: T): Promise<void> {
		await this._source.create(doc);
	}

	async update(id: string, changes: Partial<T>): Promise<void> {
		await this._source.update(id, changes);
	}

	async delete(id: string): Promise<void> {
		await this._source.delete(id);
	}

	query(): IQueryBuilder<T> {
		return new QueryBuilder((cte: CTE, predicates: QueryPredicate<T>[]) => {
			return this._createQuerySubscription(cte, predicates);
		});
	}

	getStatus(): AdapterStatus {
		return this._source.getStatus();
	}

	/**
	 * Create a query subscription for the given CTE and predicates
	 */
	private _createQuerySubscription(
		cte: CTE,
		predicates: QueryPredicate<T>[]
	): QuerySubscriptionResult<T> {
		return (callbacks) => {
			const cteKey = JSON.stringify(cte);
			const predicateKey = predicates.map((p) => p.toString()).join('|');
			const subscriptionKey = `${cteKey}:${predicateKey}`;

			// Check if we already have an identical subscription
			let activeSubscription = this._activeSubscriptions.get(subscriptionKey);

			if (!activeSubscription) {
				// Initialize callback storage BEFORE creating adapter subscription
				// This is important because adapter.subscribe may call onUpdate immediately
				const callbackSet = new Set<(docs: T[]) => void>();
				const errorSet = new Set<(error: Error) => void>();

				this._callbacks.set(subscriptionKey, callbackSet);
				this._errorCallbacks.set(subscriptionKey, errorSet);

				// Create the subscription marker
				activeSubscription = {
					cte,
					predicates,
					count: 0,
					unsubscribe: () => {},
					adapterSubscriptionCreated: false,
					lastResults: null
				};

				this._activeSubscriptions.set(subscriptionKey, activeSubscription);
			}

			// Register callback BEFORE creating adapter subscription so it's in the set
			// when the adapter calls onUpdate immediately
			const callbackSet = this._callbacks.get(subscriptionKey)!;
			callbackSet.add(callbacks.onUpdate);

			const errorSet = this._errorCallbacks.get(subscriptionKey)!;
			errorSet.add(callbacks.onError);

			activeSubscription.count++;

			// Create adapter subscription if not yet created
			if (!activeSubscription.adapterSubscriptionCreated) {
				activeSubscription.adapterSubscriptionCreated = true;
				const unsubscribe = this._source.subscribe(
					cte,
					(docs) => this._handleAdapterUpdate(subscriptionKey, docs, predicates),
					(error) => this._handleAdapterError(subscriptionKey, error)
				);

				activeSubscription.unsubscribe = unsubscribe;
			} else if (activeSubscription.lastResults !== null) {
				// If we already have cached results, emit them immediately to this new subscriber
				try {
					callbacks.onUpdate(activeSubscription.lastResults);
				} catch (error) {
					callbacks.onError(error instanceof Error ? error : new Error(String(error)));
				}
			}

			// Return unsubscribe function
			return () => {
				const callbackSet = this._callbacks.get(subscriptionKey);
				if (callbackSet) {
					callbackSet.delete(callbacks.onUpdate);
				}

				const errorSet = this._errorCallbacks.get(subscriptionKey);
				if (errorSet) {
					errorSet.delete(callbacks.onError);
				}

				const activeSubscription = this._activeSubscriptions.get(subscriptionKey);
				if (activeSubscription) {
					activeSubscription.count--;

					// Clean up if no more subscribers
					if (activeSubscription.count <= 0) {
						activeSubscription.unsubscribe();
						this._activeSubscriptions.delete(subscriptionKey);
						this._callbacks.delete(subscriptionKey);
						this._errorCallbacks.delete(subscriptionKey);
					}
				}
			};
		};
	}

	private _handleAdapterUpdate(
		subscriptionKey: string,
		docs: T[],
		predicates: QueryPredicate<T>[]
	): void {
		// Filter docs through predicates (D2TS pipeline)
		const filteredDocs = filterDocumentsByPredicates(docs, predicates);

		// Cache results for new subscribers
		const activeSubscription = this._activeSubscriptions.get(subscriptionKey);
		if (activeSubscription) {
			activeSubscription.lastResults = filteredDocs;
		}

		// Notify all callbacks for this subscription
		const callbackSet = this._callbacks.get(subscriptionKey);
		if (callbackSet) {
			for (const callback of callbackSet) {
				try {
					callback(filteredDocs);
				} catch (error) {
					const errorSet = this._errorCallbacks.get(subscriptionKey);
					if (errorSet) {
						for (const errorCallback of errorSet) {
							errorCallback(error instanceof Error ? error : new Error(String(error)));
						}
					}
				}
			}
		}
	}

	private _handleAdapterError(subscriptionKey: string, error: Error): void {
		const errorSet = this._errorCallbacks.get(subscriptionKey);
		if (errorSet) {
			for (const callback of errorSet) {
				try {
					callback(error);
				} catch {
					// Silently ignore errors from error handlers to prevent infinite loops
				}
			}
		}
	}
}
