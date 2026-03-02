/**
 * @aicacia/db - Offline-first local database for JavaScript/TypeScript
 *
 * Public API exports
 */

// Types
export type {
	AdapterStatus,
	CTE,
	CTEFilter,
	CTEOrderBy,
	CollectionConfig,
	D2TSOperation,
	D2TSPipeline,
	ICollection,
	IQueryBuilder,
	ISingleton,
	OrderDirection,
	QueryPredicate,
	SingletonConfig,
	SourceAdapter,
	UnsubscribeFn
} from './types.js';

// Core utilities
export {
	addFilterToCTE,
	addNamedCTE,
	addOrderByToCTE,
	areCTEsEqual,
	cloneCTE,
	createAndFilter,
	createCTE,
	createEqualityFilter,
	createOrderBy,
	createOrFilter,
	deserializeCTE,
	evaluateCTE,
	evaluateFilter,
	getFieldValue,
	serializeCTE,
	setLimitOnCTE,
	setOffsetOnCTE
} from './cte.js';

// Query builder
export { QueryBuilder } from './queryBuilder.js';

// Collections
export { Collection } from './collection.js';
export { Singleton } from './singleton.js';

// Factories
export { createCollection, createSingleton } from './createCollection.js';

// Adapters
export { MemoryAdapter, MemorySingletonAdapter } from './memory-adapter.js';

// D2TS utilities
export {
	applyMutationFilterPipeline,
	compileMutationFilterPipeline,
	filterDocumentsByPredicates
} from './d2ts.js';
