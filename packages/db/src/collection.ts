/**
 * Collection<T> - self-contained collection with CRUD and query operations
 * REFACTORED VERSION - Uses per-CTE subscriptions instead of global cache
 */

import type { AdapterStatus, SourceAdapter, UnsubscribeFn } from './types.js';
import type { CTE } from './cte.js';
import { QueryBuilder, type IQueryBuilder, type QuerySubscriptionResult } from './queryBuilder.js';
import { toError } from './utils.js';

export interface CollectionConfig<T> {
	id: string;
	source: SourceAdapter<T>;
	keyOf: (doc: T) => string;
}

export interface ICollection<T> {
	readonly id: string;
	create(doc: T): Promise<void>;
	update(id: string, changes: Partial<T>): Promise<void>;
	delete(id: string): Promise<void>;
	query(): IQueryBuilder<T>;
	getStatus(): AdapterStatus;
}

interface QuerySubscription<T> {
	cte: CTE;
	count: number;
	lastResults: T[] | null;
}

interface CteSubscription {
	count: number;
	unsubscribe: UnsubscribeFn;
	documentKeys: Set<string>;
}

/**
 * Collection manages document subscriptions using a dual-layer pattern:
 * 1. Per-CTE adapter subscriptions (_cteSubscriptions) - one per unique query
 * 2. Query-level subscriptions (_querySubscriptions, _callbacks) - tracks user subscriptions
 *
 * This allows multiple user subscriptions to the same query to share a single adapter subscription,
 * improving efficiency and reducing redundant filtering operations.
 */
export class Collection<T> implements ICollection<T> {
	readonly id: string;
	private _source: SourceAdapter<T>;
	private _keyOf: (doc: T) => string;

	/** Cache of documents currently needed by active subscriptions */
	private _cache: Map<string, T> = new Map();

	/** Map of CTE key -> adapter subscription (one per unique query) */
	private _cteSubscriptions: Map<string, CteSubscription> = new Map();

	/** Map of CTE key -> query metadata (tracks user subscriptions) */
	private _querySubscriptions: Map<string, QuerySubscription<T>> = new Map();

	/** Map of CTE key -> set of user update callbacks */
	private _callbacks: Map<string, Set<(docs: T[]) => void>> = new Map();

	/** Map of CTE key -> set of user error callbacks */
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
		return new QueryBuilder((cte: CTE) => {
			return this._createQuerySubscription(cte);
		});
	}

	getStatus(): AdapterStatus {
		return this._source.getStatus();
	}

	private _createQuerySubscription(cte: CTE): QuerySubscriptionResult<T> {
		return (callbacks) => {
			const subscriptionKey = JSON.stringify(cte);

			let callbackSet = this._callbacks.get(subscriptionKey);
			let errorSet = this._errorCallbacks.get(subscriptionKey);

			const isFirstSubscriber = !callbackSet;

			if (isFirstSubscriber) {
				callbackSet = new Set<(docs: T[]) => void>();
				errorSet = new Set<(error: Error) => void>();
				this._callbacks.set(subscriptionKey, callbackSet);
				this._errorCallbacks.set(subscriptionKey, errorSet);

				this._querySubscriptions.set(subscriptionKey, {
					cte,
					count: 0,
					lastResults: null
				});
			}

			callbackSet!.add(callbacks.onUpdate);
			errorSet!.add(callbacks.onError);

			const querySubscription = this._querySubscriptions.get(subscriptionKey)!;
			querySubscription.count++;

			// Check if this CTE was already subscribed before we ensure it
			const cteWasAlreadySubscribed = this._cteSubscriptions.has(subscriptionKey);

			// Ensure CTE subscription exists (per-query, not global!)
			this._ensureCteSubscription(subscriptionKey, cte);

			// If CTE was already subscribed when we requested subscription,
			// send cached results to this subscriber
			if (cteWasAlreadySubscribed && isFirstSubscriber) {
				const cteSubscription = this._cteSubscriptions.get(subscriptionKey);
				const cachedDocs = cteSubscription
					? Array.from(cteSubscription.documentKeys)
							.map((key) => this._cache.get(key))
							.filter((doc): doc is T => doc !== undefined)
					: [];
				querySubscription.lastResults = cachedDocs;
				try {
					callbacks.onUpdate(cachedDocs);
				} catch (error) {
					callbacks.onError(toError(error));
				}
			} else if (!isFirstSubscriber && querySubscription.lastResults !== null) {
				// For non-first subscribers, send cached results if available
				try {
					callbacks.onUpdate(querySubscription.lastResults);
				} catch (error) {
					callbacks.onError(toError(error));
				}
			}

			// Return unsubscribe function
			return () => {
				const callbackSet = this._callbacks.get(subscriptionKey);
				const errorSet = this._errorCallbacks.get(subscriptionKey);
				const querySubscription = this._querySubscriptions.get(subscriptionKey);

				if (callbackSet) {
					callbackSet.delete(callbacks.onUpdate);
				}

				if (errorSet) {
					errorSet.delete(callbacks.onError);
				}

				if (querySubscription) {
					querySubscription.count--;

					if (querySubscription.count <= 0) {
						this._querySubscriptions.delete(subscriptionKey);
						this._callbacks.delete(subscriptionKey);
						this._errorCallbacks.delete(subscriptionKey);

						// Decrement CTE subscription when query is fully unsubscribed
						this._decrementCteSubscription(subscriptionKey);
					}
				}
			};
		};
	}

	/**
	 * Ensure we have an active CTE subscription for the given query.
	 *
	 * Multiple user subscriptions with identical CTEs share a single adapter subscription.
	 * Reference counting ensures the adapter subscription is cleaned up when all users unsubscribe.
	 *
	 * @param cteKey - JSON stringified CTE used as deduplication key
	 * @param cte - The actual CTE to pass to the adapter
	 */
	private _ensureCteSubscription(cteKey: string, cte: CTE): void {
		// Already subscribed to this specific CTE
		if (this._cteSubscriptions.has(cteKey)) {
			this._cteSubscriptions.get(cteKey)!.count++;
			return;
		}

		// Create subscription entry BEFORE subscribing so callbacks can find it
		const subscriptionEntry: CteSubscription = {
			count: 1,
			unsubscribe: () => {},
			documentKeys: new Set()
		};
		this._cteSubscriptions.set(cteKey, subscriptionEntry);

		// Subscribe to source with the ACTUAL CTE (filtered query)
		const unsubscribe = this._source.subscribe(
			cte,
			(docs) => this._handleCteUpdate(cteKey, docs),
			(error) => this._handleCteError(cteKey, error)
		);

		subscriptionEntry.unsubscribe = unsubscribe;
	}

	/**
	 * Decrement and cleanup CTE subscription reference count.
	 *
	 * When count reaches zero:
	 * 1. Unsubscribe from the adapter
	 * 2. Remove the CTE subscription entry
	 * 3. Clean up cached documents that are no longer needed by any active subscription
	 *
	 * Called when a query subscription is destroyed.
	 */
	private _decrementCteSubscription(cteKey: string): void {
		const cteSubscription = this._cteSubscriptions.get(cteKey);
		if (!cteSubscription) return;

		cteSubscription.count--;

		if (cteSubscription.count <= 0) {
			cteSubscription.unsubscribe();
			this._cteSubscriptions.delete(cteKey);

			// Clean up cached documents that are no longer needed by any subscription
			for (const docKey of cteSubscription.documentKeys) {
				const stillNeeded = Array.from(this._cteSubscriptions.values()).some((sub) =>
					sub.documentKeys.has(docKey)
				);

				if (!stillNeeded) {
					this._cache.delete(docKey);
				}
			}
		}
	}

	/**
	 * Update cache from CTE subscription and dispatch to interested queries.
	 *
	 * When the adapter emits new results:
	 * 1. Compare previous vs. next documents to track changes
	 * 2. Update the cache, removing orphaned docs and adding new ones
	 * 3. Apply any additional filters (D2TS pipeline)
	 * 4. Notify all subscribers for this query
	 */
	private _handleCteUpdate(cteKey: string, docs: T[]): void {
		const cteSubscription = this._cteSubscriptions.get(cteKey);
		if (!cteSubscription) return;

		// Build maps of previous and next documents
		const previousDocsByKey = new Map<string, T>();
		for (const key of cteSubscription.documentKeys) {
			const existing = this._cache.get(key);
			if (existing !== undefined) {
				previousDocsByKey.set(key, existing);
			}
		}

		const nextDocsByKey = new Map<string, T>();
		for (const doc of docs) {
			nextDocsByKey.set(this._keyOf(doc), doc);
		}

		// Track removed documents
		for (const oldKey of cteSubscription.documentKeys) {
			if (!nextDocsByKey.has(oldKey)) {
				// Check if any other subscription still needs this document
				const stillNeeded = Array.from(this._cteSubscriptions.values()).some(
					(sub) => sub !== cteSubscription && sub.documentKeys.has(oldKey)
				);

				if (!stillNeeded) {
					this._cache.delete(oldKey);
				}
			}
		}

		// Update with new documents
		cteSubscription.documentKeys.clear();
		for (const [key, doc] of nextDocsByKey) {
			this._cache.set(key, doc);
			cteSubscription.documentKeys.add(key);
		}

		// Dispatch to query subscribers for this CTE
		const querySubscription = this._querySubscriptions.get(cteKey);
		if (querySubscription) {
			querySubscription.lastResults = docs;

			const callbackSet = this._callbacks.get(cteKey);
			if (callbackSet) {
				for (const callback of callbackSet) {
					try {
						callback(docs);
					} catch (error) {
						const errorSet = this._errorCallbacks.get(cteKey);
						if (errorSet) {
							for (const errorCallback of errorSet) {
								errorCallback(toError(error));
							}
						}
					}
				}
			}
		}
	}

	/**
	 * Handle error from CTE subscription
	 */
	private _handleCteError(cteKey: string, error: Error): void {
		const errorSet = this._errorCallbacks.get(cteKey);
		if (errorSet) {
			for (const callback of errorSet) {
				callback(error);
			}
		}
	}
}

export function createCollection<T>(config: CollectionConfig<T>): ICollection<T> {
	return new Collection(config);
}
