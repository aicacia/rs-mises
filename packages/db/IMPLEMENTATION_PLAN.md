# Implementation Plan: Aicacia DB - Offline-First Local Database

## Overview

**Goal**: Build a practical offline-first application data layer for TypeScript/JavaScript with streaming results, pluggable persistence, and simple error handling.

**Separation of Concerns**:

- **Core library** (`@aicacia/db`): defines opinionated public API, manages query subscriptions, compiles queries to CTE format for adapters and D2TS pipelines for mutation filtering.
- **Source adapter** (`@aicacia/db-adapter-*`): implements persistence layer (local, remote, hybrid). Owns schema, schema migration, query execution, conflict resolution, sync strategy, and error recovery. Complexity hidden behind simple adapter interface.

**Core Pattern**: Each collection has ONE source adapter that handles data operations and query execution. Core provides a simple, opinionated API without exposing implementation details. Mutations throw on error. Queries are streaming and reactive with error handlers.

## Architectural Principles

1. **Opinionated Public API**: Core provides a simple, predictable interface. Implementation details are hidden.
2. **Throw on Error**: All operations throw errors directly. No Result wrappers. Errors handled via onError callbacks in subscriptions.
3. **Streaming Only**: Queries only support `.subscribe()` with onError handlers. No snapshot modes.
4. **Single Adapter per Collection**: One collection = one adapter. No merging or coordination logic.
5. **Adapter Owns Schema**: Adapters own schema, types, versioning, and migration. Core is schema-agnostic.
6. **CTE Protocol**: Queries compile to Common Table Expressions (JSON) as the serialization protocol between core and adapters.
7. **Adapter Executes Queries**: Adapters execute queries and return matching documents. Core uses D2TS pipelines only for filtering mutations.
8. **Adapter Complexity Hidden**: Adapters handle sync, conflict resolution, persistence, and retry—but this is completely hidden behind the interface.
9. **Reactive Updates**: When mutations succeed, updates appear immediately in subscribed queries.

---

## Architecture

```
┌──────────────────────────────────────────┐
│  Application Layer                       │
│  - Svelte 5 Stores & Components          │
│  - Collection CRUD (create/update/delete)│
│  - Queries (streaming subscriptions)     │
└──────────────────────────────────────────┘
              ↓
┌──────────────────────────────────────────┐
│  Core (@aicacia/db)                      │
│  - Query subscription management         │
│  - Stream coordination & event emission  │
│  - Query compilation (CTE + D2TS)        │
│  - Error propagation via callbacks       │
│  (All implementation details hidden)     │
└──────────────────────────────────────────┘
              ↓
┌─────────────────────────────────────────┐
│  SOURCE ADAPTER (1 per collection)      │
│  - Schema & type definitions            │
│  - Query execution (from CTE)           │
│  - Persistence (local/remote/hybrid)    │
│  - Schema versioning & migration        │
│  - Syncing & conflict resolution        │
│  - Retry & offline queue (if needed)    │
│  - Network detection                    │
│  (Complexity is adapter's responsibility)│
└─────────────────────────────────────────┘

Built-In Adapter Examples:
- MemoryAdapter: in-memory (testing/demo)
- IndexedDBAdapter: local persistence with optional sync
- RemoteAdapter: server-backed storage
```

---

## Core Concepts

### Streaming & Query Model

- **Reactive Queries**: Queries are subscriptions that emit arrays of matching documents as adapter state changes.
- **Document Identity**: Core uses a `keyOf` function provided at collection creation to identify unique documents.
- **Adapter-Owned Schema**: Adapters own all schema definitions, validation, versioning, and migration. Core is schema-agnostic.
- **In-Memory Results**: Active query results are cached in memory; full document storage is handled by the adapter.
- **Throw on Error**: Mutations and operations throw errors directly. Subscriptions handle errors via `onError` callbacks.
- **CTE Query Protocol**: Queries compile to Common Table Expression (JSON) format for adapter execution and serialization.

### Source Adapter Contract

Two adapter interfaces exist, each tailored to its data model:

#### Collections: `SourceAdapter<T>`

For multi-document query semantics:

```typescript
interface SourceAdapter<T> {
	// Subscribe to query results; adapter executes CTE and emits matching docs
	subscribe(
		cte: CTE,
		onUpdate: (docs: T[]) => void,
		onError: (error: Error) => void
	): UnsubscribeFn;

	// ID-based mutations throw on error
	create(doc: T): Promise<void>;
	update(id: string, changes: Partial<T>): Promise<void>;
	delete(id: string): Promise<void>;

	// Adapter status for debugging
	getStatus(): AdapterStatus;
}
```

#### Singletons: `SingletonSourceAdapter<T>`

For at-most-one-document semantics:

```typescript
interface SingletonSourceAdapter<T> {
	// Subscribe to singleton value (not array)
	subscribe(
		onUpdate: (value: T | undefined) => void,
		onError: (error: Error) => void
	): UnsubscribeFn;

	// Value-based mutations without ID
	set(doc: T): Promise<void>; // Replace entire value
	update(changes: Partial<T>): Promise<void>; // Merge changes

	// Adapter status for debugging
	getStatus(): AdapterStatus;
}
```

**Design Rationale**: Each adapter interface matches its data model exactly—collections manage arrays of IDed documents; singletons manage a single optional value. This eliminates impedance mismatch and allows adapters to optimize for their use case without unnecessary complexity.

```typescript
interface AdapterStatus {
	state: 'idle' | 'syncing' | 'offline' | 'error';
	lastSyncAt?: number;
	error?: Error;
}
```

**Query Contract (CTE Protocol for Collections)**:

- Core query builders compile to a deterministic, JSON-serializable Common Table Expression (CTE).
- The CTE is passed to `SourceAdapter.subscribe(cte, ...)` for query execution by the adapter.
- Core also compiles CTEs into D2TS pipelines for filtering mutation results before emitting to subscribers.
- CTEs serve as the serialization protocol between core and collection adapters.
- **Singletons do not use CTEs** — they stream value updates directly without query compilation.

**What Adapters Own** (complexity is hidden):

- Schema definitions, validation, and types
- Schema versioning and document migration
- Query execution (from CTE input)
- All persistence (local, remote, hybrid)
- Sync protocols and conflict resolution
- Retry logic, network detection, offline queues
- Mutation deduplication and IDs
- Error recovery and durability

**What Core Owns**:

- Query builder (fluent API)
- Query compilation (CTE for adapters, D2TS for mutation filtering)
- Query subscription management
- Event emission and error propagation
- Mutation operations (delegates to adapter)

### Collections & Singletons

Each has its own adapter interface, reflecting its data model:

#### `Collection<T>` — Multiple Documents with ID-Based Queries

A self-contained, fully independent unit:

- CRUD operations (`.create()`, `.update(id, ...)`, `.delete(id)`) throw on error
- Queries via `.query()` builder (streaming only with CTE protocol)
- Uses `SourceAdapter<T>` for persistence, schema validation, query execution
- Subscriptions include `onError` handlers
- Queries return `unsubscribe()` function for cleanup

#### `Singleton<T>` — At-Most-One Document

For non-ID-based storage (config, user preferences, settings):

- `.subscribe(onUpdate, onError)` — Stream current value (T | undefined)
- `.set(doc: T): Promise<void>` — Replace entire document
- `.update(changes: Partial<T>): Promise<void>` — Merge changes
- Uses `SingletonSourceAdapter<T>` for persistence and value streaming
- No ID parameter; adapter owns storage strategy
- Required for data that doesn't fit ID-based collections

**Adapter Complexity is Hidden** — Each adapter owns its implementation details:

- Schema definitions, validation, versioning, migration
- Query execution (collection) or value streaming (singleton)
- Persistence layer (IndexedDB, SQLite, server, etc.)
- Sync protocols, conflict resolution, offline queues
- Retry logic, network detection, error recovery

---

## Query API

Queries are built with a fluent interface. The core supports:

```typescript
// Filter
.filter((doc) => doc.status === 'active')

// Multiple filters compose
.filter((doc) => doc.priority > 3)
.filter((doc) => doc.tags?.includes('urgent'))

// Order
.orderBy('createdAt', 'desc')
.orderBy('name', 'asc')

// Limit and offset
.limit(10)
.offset(20)

// CTE (common table expression) for reusable query composition
.with('activeRecipes', (q) =>
	q.filter((doc) => doc.status === 'active')
)

// Execute (streaming only)
.subscribe(
	(docs) => { /* data updates */ },
	(error) => { /* error handling */ }
)
```

**CTEs** serve dual purpose:

1. **Serialization protocol**: Queries compile to JSON CTEs that adapters can execute
2. **Reusable composition**: Named subqueries for complex query building

No joins across collections. Adapters execute CTE queries and return results.

---

## Creating Collections & Singletons

### Collections

```typescript
interface CollectionConfig<T> {
	id: string;
	source: SourceAdapter<T>; // Adapter for multi-document management
	keyOf: (doc: T) => string; // Extract document ID
}

const recipes = createCollection<Recipe>({
	id: 'recipes',
	source: new IndexedDBAdapter({
		dbName: 'recipes-db',
		syncUrl: 'https://api.example.com/recipes',
		schema: recipeSchema,
		version: 2,
		migrations: {
			1: (doc) => ({ ...doc, tags: [] }),
			2: (doc) => ({ ...doc, author: doc.createdBy })
		}
	}),
	keyOf: (doc) => doc.id
});
```

### Singletons

```typescript
interface SingletonConfig<T> {
	id: string;
	source: SingletonSourceAdapter<T>; // Adapter for single-value management
	defaultValue?: T;
}

const settings = createSingleton<Settings>({
	id: 'settings',
	source: new IndexedDBSingletonAdapter({
		dbName: 'settings-db',
		storeName: 'app-settings',
		schema: settingsSchema
	}),
	defaultValue: { theme: 'light', language: 'en' }
});
```

**Schema Ownership**:

- **Adapter owns**: Schema definition, validation, versioning, migration strategy, and type definitions
- **Core**: Schema-agnostic; relies entirely on adapter for type safety and validation
- Each adapter pattern (collection vs singleton) owns the storage strategy appropriate to its data model

**Adapter Examples**:

- **MemoryAdapter**: In-memory storage (testing/demo; no persistence; both collection & singleton variants)
- **IndexedDBAdapter** / **IndexedDBSingletonAdapter**: Local persistence with optional sync
- **RemoteAdapter** / **RemoteSingletonAdapter**: Server-backed (server owns all data)
- Each adapter variant owns schema handling, query/value execution, and sync protocols appropriate to its model

---

## Adapter Complexity is Hidden

Each adapter may implement sophisticated features:

- **Persistence**: IndexedDB, SQLite, file system, cloud, CRDT type
- **Versioning**: Semantic versioning, pragma-based, field-based, or none
- **Migration**: Eager (on write), lazy (on read), background, or streaming
- **Sync**: REST polling, WebSocket, gRPC, p2p, CRDT convergence
- **Conflict Resolution**: Last-write-wins, 3-way merge, CRDT merging, manual resolution
- **Offline Support**: Mutation queuing, sync on reconnect, eventual consistency
- **Retry Logic**: Exponential backoff, circuit breaker, dead letter queues

**All of this complexity is behind the `SourceAdapter<T>` interface.** Application code sees only:

```typescript
try {
	await collection.create(doc);      // Simple from API perspective
	await collection.update(id, {...}); // Adapter handles everything inside
	await collection.delete(id);       // Throws on error
} catch (error) {
	console.error('Operation failed:', error);
}
```

The adapter implementation is responsible for all edge cases, retry logic, and error handling.

---

## Query Execution

Queries are built via a fluent API and return reactive streams only.

```typescript
// Streaming query - returns unsubscribe function
const unsubscribe = collection
	.query()
	.filter((doc) => doc.status === 'active')
	.orderBy('createdAt', 'desc')
	.limit(10)
	.subscribe(
		(docs) => {
			// Called whenever matching documents change
			console.log(docs); // docs: T[]
		},
		(error) => {
			// Called on subscription errors
			console.error('Query error:', error);
		}
	);

// Clean up subscription
unsubscribe();
```

**How It Works**:

- Queries compile to CTE (JSON) format
- CTE passed to adapter's `subscribe(cte, onUpdate, onError)` method
- Adapter executes query and emits matching documents
- Core maintains D2TS pipeline for filtering mutation results
- Multiple subscriptions to identical CTEs share adapter subscription
- Results are cached in memory; only subscribed queries are actively maintained

**Implementation Note**: Query filtering happens at the adapter level (adapters execute CTEs). Core uses D2TS pipelines only for filtering mutation results before emitting to subscribers.

### Mutations

#### Collection Mutations

Mutations forward to adapter and throw on error.

```typescript
// Create (throws on error)
try {
	await collection.create({
		id: '123',
		name: 'Recipe',
		status: 'active'
	});
	console.log('Created successfully');
} catch (error) {
	console.error('Failed:', error.message);
}

// Update (throws on error)
await collection.update('123', { status: 'archived' });

// Delete (throws on error)
await collection.delete('123');
```

**Semantics** — `.create()`, `.update(id, changes)`, `.delete(id)` throw on error. Adapter handles validation, persistence, sync, and conflict resolution. Core filters mutation results through D2TS pipeline before emitting to subscribers.

#### Singleton Mutations

```typescript
// Set (replace entire value)
await settings.set({ theme: 'dark', language: 'es' });

// Update (merge changes)
await settings.update({ theme: 'light' });
```

**Semantics** — `.set()` and `.update()` throw on error. Both methods delegate to adapter which commits the change atomically and broadcasts to subscribers.

---

## Persistence & Caching

### Source Adapter Responsibility

Two adapter types, two interface contracts, one principle: **Adapters own implementation complexity**.

#### Collection Adapters: `SourceAdapter<T>`

**MUST implement**:

- **CTE Query Execution**: Parse CTE protocol and return matching documents
- **CRUD Operations**: Implement `.create()`, `.update(id, changes)`, `.delete(id)`
- **Schema & Validation**: Define schema, validate documents, handle type definitions
- **Persistence**: Store and retrieve documents

#### Singleton Adapters: `SingletonSourceAdapter<T>`

**MUST implement**:

- **Value Streaming**: Subscribe to value changes and emit T | undefined
- **Set/Update Operations**: Implement `.set(doc)` and `.update(changes)`
- **Schema & Validation**: Define schema, validate documents, handle type definitions
- **Persistence**: Store and retrieve singleton value

#### Both Adapter Types MAY implement (all optional, adapter choice):

- **Sync protocol**: REST, GraphQL, WebSocket, CRDT p2p, etc.
- **Conflict resolution**: LWW, CRDT merge, custom logic, manual resolution
- **Mutation queueing**: Offline queue with retry, or fail fast
- **Deduplication**: Mutation IDs and replay detection, or idempotent at protocol level
- **Recovery**: Startup state loading, corruption handling, compaction
- **Deletes**: Tombstones, soft deletes, hard deletes
- **Network detection**: Automatic or manual
- **Versioning & Migration**: Document schema versioning strategy

**Core does NOT care how adapters implement these** — only that they implement their respective interface and fulfill the contract.

### Memory Management

**Collection holds**:

- Current query results in memory (only what matches active subscriptions)
- D2TS pipelines for mutation filtering
- Active subscription mappings

**Adapters hold**:

- Schema definitions and validation logic
- All document data (persistent or ephemeral)
- Query execution state
- Any caches, indexes, logs, mutation queues

**For large datasets**:

- Pagination via queries (limit/offset)
- Adapter-level indexing for performance
- Collections don't hold entire datasets in memory - only active query results

---

## Svelte Integration

### Reactive Stores

Export `@aicacia/db/svelte` module for seamless component integration:

```typescript
import { collection, singleton } from '@aicacia/db/svelte';

const recipes = collection(recipesCollection.query().where(/* ... */));
const settings = singleton(userSettingsSingleton);
```

- **Auto-subscription**: Subscribe on mount, unsubscribe on unmount automatically
- **Reactive syntax**: Use `$recipes` and `$settings` in components
- **Simple bindings**: Pass queries directly to stores

---

## Public API Reference

### Core Package (`@aicacia/db`)

**Factories**:

- `createCollection<T>(config)` — Create a collection
- `createSingleton<T>(config)` — Create a singleton (at-most-one document)

**Collection Methods**:

- `.create(doc)` → `Promise<void>` — Create document (throws on error)
- `.update(id, changes)` → `Promise<void>` — Update document (throws on error)
- `.delete(id)` → `Promise<void>` — Delete document (throws on error)
- `.query()` → `QueryBuilder<T>` — Build a query
- `.getStatus()` → `AdapterStatus` — Get adapter state

**QueryBuilder Methods**:

- `.filter(predicate)` → `QueryBuilder<T>` — Filter documents
- `.orderBy(field, direction)` → `QueryBuilder<T>` — Sort results
- `.limit(n)` → `QueryBuilder<T>` — Limit result count
- `.offset(n)` → `QueryBuilder<T>` — Skip first n results
- `.with(name, query)` → `QueryBuilder<T>` — Define a reusable CTE
- `.subscribe(onUpdate, onError)` → `UnsubscribeFn` — Stream updates
- `.toCTE()` → `CTE` — Export query as JSON CTE (for serialization)

**SourceAdapter Interface**:

```typescript
interface SourceAdapter<T> {
	// Execute CTE query and stream results
	subscribe(
		cte: CTE,
		onUpdate: (docs: T[]) => void,
		onError: (error: Error) => void
	): UnsubscribeFn;

	// Mutations throw on error
	create(doc: T): Promise<void>;
	update(id: string, changes: Partial<T>): Promise<void>;
	delete(id: string): Promise<void>;

	getStatus(): AdapterStatus;
}
```

### Svelte 5 Package (`@aicacia/db/svelte`)

Svelte 5 only. Rune-based stores auto-subscribe/cleanup.

```typescript
import { collection, singleton } from '@aicacia/db/svelte';

// Readable store (updates reactive)
const recipes = collection(recipesCollection.query().filter((d) => d.active));

// Writable store (for singletons)
const settings = singleton(settingsSingleton);
```

Usage in components:

```svelte
{#each $recipes as recipe (recipe.id)}
  <div>{recipe.name}</div>
{/each}

<button on:click={() => $settings.theme = 'dark'}>
  Set dark
</button>
```

---

## Design Principles

| Principle               | How We Follow It                                                       |
| ----------------------- | ---------------------------------------------------------------------- |
| **Opinionated API**     | Simple public interface; implementation hidden; Svelte 5 only          |
| **Throw on Error**      | Operations throw errors; subscriptions have onError handlers           |
| **Streaming Only**      | Queries only support `.subscribe()` - no snapshot modes                |
| **Single Adapter**      | One collection = one adapter; no merging                               |
| **Reactive**            | Updates in subscribed queries appear immediately                       |
| **Adapter Autonomy**    | Schema, validation, query execution, sync - all adapter's choice       |
| **CTE Protocol**        | Queries compile to JSON CTEs for adapter execution and serialization   |
| **Hidden Complexity**   | Persistence, retry, backoff, deduplication hidden behind interface     |
| **Adapter Owns Schema** | Adapters own schema, validation, versioning, and types completely      |
| **Explicit Cleanup**    | Queries return `unsubscribe()` function; prevent leaks                 |
| **Error Propagation**   | Errors throw from mutations and propagate via onError in subscriptions |

---

## Project Structure

### Core Package (`packages/db/src/`)

```
├── index.ts                 # Main exports
├── types.ts                 # TypeScript interfaces (CTE, SourceAdapter, SingletonSourceAdapter, etc.)
├── collection.ts            # Collection<T> class
├── singleton.ts             # Singleton<T> class
├── createCollection.ts      # Factory functions
├── queryBuilder.ts          # Query builder & CTE compiler
├── cte.ts                   # CTE (Common Table Expression) types and utils
├── d2ts.ts                  # D2TS pipeline for mutation filtering
├── svelte.ts                # Svelte 5 integration
└── memory-adapter.ts        # MemoryAdapter reference implementations (both collection & singleton)
```

**Export structure**:

- `@aicacia/db` — Main API
- `@aicacia/db/svelte` — Svelte 5 integration
- Adapters maintained separately: `@aicacia/db-adapter-indexeddb`, `@aicacia/db-adapter-indexeddb-singleton`, etc.

---

## Dependencies

| Package         | Purpose        | Notes                          |
| --------------- | -------------- | ------------------------------ |
| `eventemitter3` | Event emission | Already in package.json        |
| (none)          | Minimal deps   | Adapters own schema/validation |

---

---

## Core Responsibilities vs Adapter Responsibilities

| Concern                 | Core                                     | Adapter                           |
| ----------------------- | ---------------------------------------- | --------------------------------- |
| **Schema & Validation** | ✗ (adapter owns completely)              | ✓ Schema definition & validation  |
| **Query Building**      | ✓ Fluent API, compile to CTE             | Execute CTE queries               |
| **Query Execution**     | ✗ (adapter executes)                     | ✓ Execute CTE, return results     |
| **Mutation Filtering**  | ✓ D2TS pipeline filters mutation results | Emit raw mutation results         |
| **Subscriptions**       | ✓ Manage subscriptions, coordinate       | Emit updates via callbacks        |
| **Error Handling**      | ✓ Propagate via throw and onError        | Throw errors or call onError      |
| **Persistence**         | ✗ (adapter choice)                       | ✓ Local/remote/hybrid storage     |
| **Schema Versioning**   | ✗ (adapter choice)                       | ✓ Document migration & versioning |
| **Sync/Conflict**       | ✗ (adapter choice)                       | ✓ Any protocol (REST, p2p, CRDT)  |
| **Retry & Network**     | ✗ (adapter choice)                       | ✓ Offline queue, backoff, etc.    |
| **Deduplication**       | ✗ (adapter choice)                       | ✓ Mutation IDs & idempotency      |

---

## Test Scenarios

Core library tests (essential behaviors only):

### Queries & Subscriptions

- **Q1**: Subscription receives document array updates
- **Q2**: `unsubscribe()` stops updates and cleans up memory
- **Q3**: Filter, orderBy, limit operators compose correctly
- **Q4**: Query compiles to valid CTE (JSON)
- **Q5**: CTE passed to adapter's subscribe method
- **Q6**: onError handler receives subscription errors

### Mutations

- **M1**: Mutations throw on error
- **M2**: Adapter receives mutation calls
- **M3**: Multiple simultaneous mutations don't corrupt state
- **M4**: Mutations trigger updates in subscribed queries
- **M5**: D2TS pipeline filters mutation results before emission

### Collections

- **C1**: Collection initializes before adapter is ready
- **C2**: Multiple queries on same collection share data efficiently
- **C3**: Collection cleanup unsubscribes all queries

### Singletons

- **S1**: Singleton `.subscribe()` streams current value when subscribed
- **S2**: Singleton `.set()` replaces document (throws on error)
- **S3**: Singleton `.update()` merges changes (throws on error)
- **S4**: Singleton subscription cleanup stops listening
- **S5**: Value updates trigger all subscriptions
- **S6**: SingletonSourceAdapter receives set/update calls

### Svelte Integration

- **SV1**: Stores auto-subscribe on component mount
- **SV2**: Stores auto-unsubscribe on component destroy
- **SV3**: Multiple components with same query share subscription

### Adapters (Adapter-Specific Tests)

Both collection and singleton adapters must verify their own:

- **Collection**: Query execution (CTE parsing and filtering), CRUD ops, persistence strategy
- **Singleton**: Value streaming, set/update operations, persistence strategy
- Schema versioning and migrations (if applicable)
- Sync protocol and conflict resolution (if applicable)
- Offline queue and retry behavior (if applicable)
- Error recovery and durability guarantees

Example adapter tests:

- **A1** (IndexedDB Collection): Data persists across page reload
- **A2** (IndexedDB Singleton): Settings persist across page reload
- **A3** (Synced Collection): Offline mutations queue and sync when online
- **A4** (Remote Singleton): Values sync to server and back
- **A5** (Memory Adapter): Data correctly filtered and emitted (both models)

### Edge Cases

- **[EG1] Empty collection**: Queries on empty collection return empty array
- **[EG2] All documents deleted**: Collection with all docs deleted behaves correctly
- **[EG3] Very long query chains**: 20+ chained query operators compile to valid CTE
- **[EG4] Rapid mutations**: 1000 mutations in 1 second handled correctly
- **[EG5] CTE serialization**: Complex queries serialize to JSON and back

### Type Safety & Misc Tests

- **[TS1] TypeScript inference**: QueryBuilder methods are fully typed
- **[TS2] No global state**: Two collections are independent
- **[TS3] CTE compilation**: Queries compile to deterministic CTE format
- **[TS4] D2TS compilation**: CTEs compile to D2TS pipelines for mutation filtering

---

## Implementation Scope

Core library (`@aicacia/db`) provides a simple, opinionated API for streaming reactive collections and singletons.

**Core Deliverables**:

- `Collection<T>` and `Singleton<T>` classes
- `SourceAdapter<T>` interface (for collections)
- `SingletonSourceAdapter<T>` interface (for singletons)
- CRUD operations (throw on error)
- Query builder (fluent API)
- CTE compiler (query → JSON)
- D2TS compiler (CTE → pipeline for mutation filtering)
- Subscription management with onError handlers
- MemoryAdapter reference implementations (both collection & singleton variants)
- Svelte 5 integration
- Subscription cleanup

**What Core Does**:

- Query building (fluent API with CTE compilation)
- Query subscription management and coordination
- Error propagation (throw on mutation failure, onError for subscriptions)
- D2TS pipeline compilation (for filtering mutation results)
- Mutation routing and result filtering

**What Adapters Do** (fully owned by adapter):

- Schema definition and validation (appropriate to data model)
- Query execution (collections) or value streaming (singletons)
- Type definitions
- Persistence (local, remote, hybrid)
- Schema versioning and migration
- Sync protocols (REST, WebSocket, p2p)
- Conflict resolution (LWW, CRDT, manual)
- Retry and network handling
- Deduplication and idempotency
- Error recovery

**Reference Adapters** (separate packages):

- `@aicacia/db-adapter-memory` — In-memory testing/demo (collection & singleton variants)
- `@aicacia/db-adapter-indexeddb` — Local IndexedDB (collection)
- `@aicacia/db-adapter-indexeddb-singleton` — Local IndexedDB (singleton)
- `@aicacia/db-adapter-synced` — IndexedDB + REST sync (collection)
- `@aicacia/db-adapter-remote` — Server-backed (collection & singleton variants)

---

## Example Usage

```typescript
import { createCollection, createSingleton } from '@aicacia/db';
import { IndexedDBAdapter } from '@aicacia/db-adapter-indexeddb';
import { recipeSchema, userSettingsSchema } from './schemas';

// Collection with adapter (adapter owns schema)
const recipes = createCollection({
	id: 'recipes',
	source: new IndexedDBAdapter({
		dbName: 'myapp',
		schema: recipeSchema
	}),
	keyOf: (doc) => doc.id
});

// Create (throws on error)
try {
	await recipes.create({ id: '1', name: 'Pasta' });
} catch (error) {
	console.error(error);
}

// Query stream with error handler
const unsubscribe = recipes
	.query()
	.filter((doc) => doc.tags?.includes('quick'))
	.subscribe(
		(docs) => console.log('Recipes:', docs),
		(error) => console.error('Query error:', error)
	);

// Singleton (for non-ID-based data)
const settings = createSingleton({
	id: 'user-settings',
	source: new IndexedDBAdapter({
		dbName: 'myapp',
		schema: userSettingsSchema
	}),
	defaultValue: { theme: 'light' }
});

await settings.update({ theme: 'dark' });
```

**Svelte 5**:

```svelte
<script>
  import { collection } from '@aicacia/db/svelte';
  import { recipes } from './db';

  const activeRecipes = collection(
    recipes.query().filter((r) => r.active)
  );
</script>

{#each $activeRecipes as recipe (recipe.id)}
  <div>{recipe.name}</div>
{/each}
```

---
