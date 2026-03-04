// Core types
export type { AdapterStatus, SourceAdapter, UnsubscribeFn, SubscriptionError } from './types.js';

// Collection types and API
export type { CollectionConfig, ICollection } from './collection.js';
export { Collection, createCollection } from './collection.js';

// Singleton types and API
export type { SingletonConfig, ISingleton } from './singleton.js';
export { Singleton, createSingleton } from './singleton.js';

// Query builder types and API
export type {
	IQueryBuilder,
	OrderDirection,
	QueryCompiler,
	QuerySubscriptionResult,
	DeepKeyOf
} from './queryBuilder.js';
export { QueryBuilder } from './queryBuilder.js';

// CTE types and operations
export type { CTE, CTEFilter, CTEOrderBy } from './cte.js';
export {
	addNamedCTE,
	createAndFilter,
	createCTE,
	createEqualityFilter,
	createOrFilter
} from './cte.js';

// Adapters
export { MemoryAdapter, MemorySingletonAdapter } from './memoryAdapter.js';

// Filter engine
export { applyFilters } from './d2ts.js';
export {
	evaluateCTE,
	evaluateFilter,
	getFieldValue,
	applyOrderBy,
	applyPagination
} from './filterEngine.js';

// Test utilities
export {
	createTestCollection,
	createTestSingleton,
	type TestCollectionResult,
	type TestSingletonResult
} from './test-utils.js';
