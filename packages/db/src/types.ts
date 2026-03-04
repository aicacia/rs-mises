import type { CTE } from './cte.js';

/** Runtime state reported by adapters. */
export interface AdapterStatus {
	state: 'idle' | 'syncing' | 'offline' | 'error';
	lastSyncAt?: number;
	error?: Error;
}

/** Persistence adapter for multi-document collections. */
export interface SourceAdapter<T> {
	/** Subscribe to query results and return an unsubscribe function. */
	subscribe(
		cte: CTE,
		onUpdate: (docs: T[]) => void,
		onError: (error: Error) => void
	): UnsubscribeFn;

	/** Create a document. */
	create(doc: T): Promise<void>;

	/** Update a document by id. */
	update(id: string, changes: Partial<T>): Promise<void>;

	/** Delete a document by id. */
	delete(id: string): Promise<void>;

	/** Get adapter status. */
	getStatus(): AdapterStatus;
}

/** Persistence adapter for singleton (at-most-one) values. */
export interface SingletonSourceAdapter<T> {
	/** Subscribe to value changes and return an unsubscribe function. */
	subscribe(
		onUpdate: (value: T | undefined) => void,
		onError: (error: Error) => void
	): UnsubscribeFn;

	/** Replace the current value. */
	set(doc: T): Promise<void>;

	/** Merge partial changes into the current value. */
	update(changes: Partial<T>): Promise<void>;

	/** Get adapter status. */
	getStatus(): AdapterStatus;
}

/** Function that stops an active subscription. */
export type UnsubscribeFn = () => void;

/** Structured error metadata for subscription failures. */
export interface SubscriptionError extends Error {
	/** Error source. */
	type: 'adapter' | 'filter' | 'subscriber';

	/** Original underlying error. */
	originalError: Error;
}
