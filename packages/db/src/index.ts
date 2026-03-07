// Core types
export type { AdapterStatus, FieldPath, SourceAdapter, UnsubscribeFn } from './types.js';

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
	QuerySubscriptionResult
} from './queryBuilder.js';
export { QueryBuilder } from './queryBuilder.js';

// CTE types and operations
export type { CTE, CTEFilter, CTEOrderBy } from './cte.js';
export {
	and,
	compare,
	contains,
	containsIgnoreCase,
	createCTE,
	equal,
	fuzzyContains,
	greaterThan,
	greaterThanOrEqual,
	inCTE,
	inOperator,
	includes,
	lessThan,
	lessThanOrEqual,
	notEqual,
	notInCTE,
	or
} from './cte.js';

// Adapters
export { MemoryAdapter, MemorySingletonAdapter } from './memoryAdapter.js';

// Filter engine
export { applyCTE, applyFilter, applyOrderBy, applyPagination } from './filterEngine.js';

// Utilities
export { getFieldValue } from './utils.js';

// Test utilities
export {
	createTestCollection,
	createTestSingleton,
	type TestCollectionResult,
	type TestSingletonResult
} from './test-utils.js';
