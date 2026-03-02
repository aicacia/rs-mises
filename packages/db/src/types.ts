/**
 * Core TypeScript interfaces for @aicacia/db
 */

/**
 * CTE (Common Table Expression) - JSON serialization of compiled queries
 */
export interface CTE {
	version: '1.0';
	name?: string;
	columns?: string[];
	filters?: CTEFilter[];
	orderBy?: CTEOrderBy[];
	limit?: number;
	offset?: number;
	ctes?: Record<string, CTE>;
}

export interface CTEFilter {
	type: 'comparison' | 'logical';
	operator?: string;
	field?: string;
	value?: unknown;
	filters?: CTEFilter[];
}

export interface CTEOrderBy {
	field: string;
	direction: 'asc' | 'desc';
}

/**
 * D2TS Pipeline - compiled filter operations for mutation result filtering
 */
export interface D2TSPipeline {
	operations: D2TSOperation[];
}

export interface D2TSOperation {
	type: string;
	[key: string]: unknown;
}

/**
 * Adapter Status - diagnostic information about adapter state
 */
export interface AdapterStatus {
	state: 'idle' | 'syncing' | 'offline' | 'error';
	lastSyncAt?: number;
	error?: Error;
}

/**
 * Source Adapter - interface for persistence layers (collections)
 *
 * Designed for collections with multiple documents. For singleton/at-most-one semantics,
 * use SingletonSourceAdapter instead.
 */
export interface SourceAdapter<T> {
	/**
	 * Subscribe to query results
	 * @param cte - Common Table Expression defining the query
	 * @param onUpdate - Called with matching documents when results change
	 * @param onError - Called when subscription encounters an error
	 * @returns Unsubscribe function to clean up subscription
	 */
	subscribe(
		cte: CTE,
		onUpdate: (docs: T[]) => void,
		onError: (error: Error) => void
	): UnsubscribeFn;

	/**
	 * Create a new document
	 * @throws Error if creation fails
	 */
	create(doc: T): Promise<void>;

	/**
	 * Update an existing document
	 * @throws Error if update fails
	 */
	update(id: string, changes: Partial<T>): Promise<void>;

	/**
	 * Delete a document
	 * @throws Error if deletion fails
	 */
	delete(id: string): Promise<void>;

	/**
	 * Get current adapter status
	 */
	getStatus(): AdapterStatus;
}

/**
 * Singleton Source Adapter - interface for at-most-one document persistence
 *
 * Designed specifically for singleton/at-most-one-document semantics.
 * Adapters own all implementation details: schema, validation, versioning, migration,
 * persistence, sync, and conflict resolution.
 */
export interface SingletonSourceAdapter<T> {
	/**
	 * Subscribe to singleton value changes
	 * @param onUpdate - Called with the current value (T | undefined) when it changes
	 * @param onError - Called when subscription encounters an error
	 * @returns Unsubscribe function to clean up subscription
	 */
	subscribe(
		onUpdate: (value: T | undefined) => void,
		onError: (error: Error) => void
	): UnsubscribeFn;

	/**
	 * Replace entire singleton value
	 * @param doc - The new document (replaces any existing value)
	 * @throws Error if operation fails
	 */
	set(doc: T): Promise<void>;

	/**
	 * Merge changes into singleton value
	 * @param changes - Partial document with fields to update
	 * @throws Error if operation fails or singleton not initialized
	 */
	update(changes: Partial<T>): Promise<void>;

	/**
	 * Get current adapter status
	 */
	getStatus(): AdapterStatus;
}

/**
 * Unsubscribe function
 */
export type UnsubscribeFn = () => void;

/**
 * Query predicate function
 */
export type QueryPredicate<T> = (doc: T) => boolean;

/**
 * Order direction
 */
export type OrderDirection = 'asc' | 'desc';

/**
 * Collection configuration
 */
export interface CollectionConfig<T> {
	/** Unique collection identifier */
	id: string;

	/** Source adapter for this collection */
	source: SourceAdapter<T>;

	/** Function to extract unique key from document */
	keyOf: (doc: T) => string;
}

/**
 * Singleton configuration
 */
export interface SingletonConfig<T> {
	/** Unique singleton identifier */
	id: string;

	/** Source adapter for this singleton */
	source: SingletonSourceAdapter<T>;

	/** Default value if not yet set */
	defaultValue?: T;
}

/**
 * Query builder result subscription
 */
export interface QuerySubscription<T> {
	(onUpdate: (docs: T[]) => void, onError?: (error: Error) => void): UnsubscribeFn;
}

/**
 * Query builder interface
 */
export interface IQueryBuilder<T> {
	/**
	 * Filter documents by predicate
	 */
	filter(predicate: QueryPredicate<T>): IQueryBuilder<T>;

	/**
	 * Order results by field
	 */
	orderBy(field: keyof T, direction?: OrderDirection): IQueryBuilder<T>;

	/**
	 * Limit number of results
	 */
	limit(n: number): IQueryBuilder<T>;

	/**
	 * Skip first n results
	 */
	offset(n: number): IQueryBuilder<T>;

	/**
	 * Define a reusable CTE subquery
	 */
	with(name: string, fn: (q: IQueryBuilder<T>) => IQueryBuilder<T>): IQueryBuilder<T>;

	/**
	 * Subscribe to query results
	 */
	subscribe(onUpdate: (docs: T[]) => void, onError?: (error: Error) => void): UnsubscribeFn;

	/**
	 * Export query as CTE
	 */
	toCTE(): CTE;
}

/**
 * Collection interface
 */
export interface ICollection<T> {
	readonly id: string;

	/**
	 * Create a new document
	 * @throws Error if creation fails
	 */
	create(doc: T): Promise<void>;

	/**
	 * Update an existing document
	 * @throws Error if update fails
	 */
	update(id: string, changes: Partial<T>): Promise<void>;

	/**
	 * Delete a document
	 * @throws Error if deletion fails
	 */
	delete(id: string): Promise<void>;

	/**
	 * Build a query
	 */
	query(): IQueryBuilder<T>;

	/**
	 * Get adapter status
	 */
	getStatus(): AdapterStatus;
}

/**
 * Singleton interface
 */
export interface ISingleton<T> {
	readonly id: string;

	/**
	 * Subscribe to singleton value
	 */
	subscribe(
		onUpdate: (value: T | undefined) => void,
		onError?: (error: Error) => void
	): UnsubscribeFn;

	/**
	 * Replace entire singleton value
	 * @throws Error if set fails
	 */
	set(doc: T): Promise<void>;

	/**
	 * Merge changes into singleton value
	 * @throws Error if update fails
	 */
	update(changes: Partial<T>): Promise<void>;

	/**
	 * Get current adapter status
	 */
	getStatus(): AdapterStatus;
}
