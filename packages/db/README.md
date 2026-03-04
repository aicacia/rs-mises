# Aicacia DB

**Offline-first**, **eventually-consistent** local database for JavaScript and TypeScript applications with real-time streaming queries and pluggable persistence.

## Features

- ✅ **Streaming queries** — Reactive subscriptions that emit results as data changes
- ✅ **Multiple data models** — Collections (ID-based documents) and Singletons (at-most-one value)
- ✅ **Pluggable adapters** — Custom persistence layers (memory, IndexedDB, remote, etc.)
- ✅ **CTE query protocol** — Serializable Common Table Expressions for query composition
- ✅ **Svelte integration** — Built-in reactive stores with automatic cleanup
- ✅ **Type-safe** — Full TypeScript support with inferred types
- ✅ **Error handling** — Mutations throw on error; subscriptions support error callbacks
- 🔄 **Simple API** — Fluent query builder, straightforward CRUD operations

## Quick Start

### Installation

```bash
npm install @aicacia/db
```

### Basic Usage

#### Collections (Multiple Documents)

```typescript
import { createCollection, MemoryAdapter, createEqualityFilter } from '@aicacia/db';

// Define your schema
interface Recipe {
	id: string;
	name: string;
	status: 'active' | 'archived';
	prepTime: number;
}

// Create adapter and collection
const adapter = new MemoryAdapter<Recipe>();
const recipes = createCollection({
	id: 'recipes',
	source: adapter,
	keyOf: (doc) => doc.id
});

// Create documents
await recipes.create({
	id: '1',
	name: 'Pasta',
	status: 'active',
	prepTime: 20
});

// Query with streaming results
const unsubscribe = recipes
	.query()
	.where(createEqualityFilter('status', 'active'))
	.orderBy('name', 'asc')
	.subscribe(
		(docs) => console.log('Updated:', docs),
		(error) => console.error('Error:', error)
	);

// Update document
await recipes.update('1', { status: 'archived' });

// Delete document
await recipes.delete('1');

// Cleanup
unsubscribe();
```

#### Singletons (At-Most-One Value)

```typescript
import { createSingleton, MemorySingletonAdapter } from '@aicacia/db';

// Define schema
interface UserSettings {
	theme: 'light' | 'dark';
	language: string;
	notifications: boolean;
}

// Create singleton
const adapter = new MemorySingletonAdapter<UserSettings>();
const settings = createSingleton({
	id: 'user-settings',
	source: adapter,
	defaultValue: { theme: 'light', language: 'en', notifications: true }
});

// Subscribe to changes
const unsubscribe = settings.subscribe(
	(value) => console.log('Settings:', value),
	(error) => console.error('Error:', error)
);

// Update entire value
await settings.set({ theme: 'dark', language: 'es', notifications: false });

// Merge changes
await settings.update({ theme: 'light' });

// Cleanup
unsubscribe();
```

### Svelte Integration

```svelte
<script>
  import { collection, singleton } from '@aicacia/db/svelte';
  import { createEqualityFilter } from '@aicacia/db';
  import { recipes, settings } from './stores.ts';

  const recipesList = collection(recipes.query().where(createEqualityFilter('status', 'active')));
  const userSettings = singleton(settings);
</script>

<div>
  <!-- Auto-subscribed, reactive -->
  {#each recipesList.data as recipe (recipe.id)}
    <div>{recipe.name} ({recipe.prepTime} min)</div>
  {/each}

  {#if recipesList.error}
    <p class="error">{recipesList.error.message}</p>
  {/if}

  <div>Theme: {userSettings.data?.theme}</div>
</div>
```

## Architecture

```
┌─────────────────────────────────┐
│  Application Layer              │
│  Svelte/React/Vue Components    │
└────────────┬────────────────────┘
             │
┌────────────▼────────────────────┐
│  Core (@aicacia/db)             │
│  • Query subscriptions           │
│  • Streaming & event emission    │
│  • Query compilation (CTE)       │
│  • Error propagation            │
└────────────┬────────────────────┘
             │
┌────────────▼────────────────────┐
│  Source Adapter                 │
│  • Persistence layer            │
│  • Query execution              │
│  • Schema & validation          │
│  • Sync & conflicts (optional)  │
└─────────────────────────────────┘

Available Adapters:
  • MemoryAdapter - in-memory (testing/demo)
  • MemorySingletonAdapter - in-memory singletons
  (IndexedDB, Remote adapters in progress)
```

## Query API

### Build Queries

```typescript
import { createEqualityFilter, createComparisonFilter } from '@aicacia/db';

const query = collection
	.query()
	.where(createEqualityFilter('status', 'active'))
	.where(createComparisonFilter('prepTime', 'lessThan', 30))
	.orderBy('name', 'asc')
	.limit(10)
	.offset(0)
	.with('recentRecipes', (q) =>
		q.where(createComparisonFilter('createdAt', 'greaterThan', Date.now() - 86400000))
	);
```

#### Available Methods

- `.where(filter)` — Add CTE filter condition (use `createEqualityFilter`, `createComparisonFilter`, etc.)
- `.orderBy(field, direction?)` — Sort by field ('asc' or 'desc')
- `.limit(n)` — Limit results to n documents
- `.offset(n)` — Skip first n results
- `.with(name, fn)` — Define named CTE subquery
- `.subscribe(onUpdate, onError?)` — Subscribe to results (streaming)
- `.toCTE()` — Export as serializable CTE (for adapters)

#### Filter Creators

```typescript
import {
	createEqualityFilter,
	createComparisonFilter,
	createAndFilter,
	createOrFilter
} from '@aicacia/db';

// Equality
createEqualityFilter('status', 'active');

// Comparisons
createComparisonFilter('prepTime', 'lessThan', 30);
createComparisonFilter('rating', 'greaterThanOrEqual', 4);

// Logical operators
createAndFilter(
	createEqualityFilter('status', 'active'),
	createComparisonFilter('prepTime', 'lessThan', 30)
);

createOrFilter(
	createEqualityFilter('category', 'dessert'),
	createEqualityFilter('category', 'appetizer')
);
```

### Subscribe to Results

```typescript
// Subscribe with both callbacks
const unsubscribe = query.subscribe(
	(docs) => {
		// Called whenever results change
		console.log('Results:', docs);
	},
	(error) => {
		// Called on subscription errors
		console.error('Subscription failed:', error);
	}
);

// Or just success callback
query.subscribe((docs) => console.log(docs));

// Cleanup when done
unsubscribe();
```

## Collections API

### Creating Collections

```typescript
interface DocumentType {
	id: string;
	// ... your fields
}

const collection = createCollection<DocumentType>({
	id: 'unique-collection-id',
	source: adapter, // SourceAdapter<DocumentType>
	keyOf: (doc) => doc.id // Extract unique ID
});
```

### CRUD Operations

All mutations throw on error:

```typescript
// Create
try {
	await collection.create({
		id: '123',
		name: 'Example'
	});
} catch (error) {
	console.error('Failed to create:', error.message);
}

// Update (partially)
await collection.update('123', { name: 'Updated' });

// Delete
await collection.delete('123');

// Query
collection.query().subscribe((docs) => {
	console.log('All documents:', docs);
});

// Get adapter status
const status = collection.getStatus();
console.log(status.state); // 'idle', 'syncing', 'offline', 'error'
```

## Singletons API

### Creating Singletons

```typescript
interface SingletonType {
	field: string;
	// ... your fields
}

const singleton = createSingleton<SingletonType>({
	id: 'unique-singleton-id',
	source: adapter, // SingletonSourceAdapter<SingletonType>
	defaultValue: { field: 'default' } // Optional
});
```

### Operations

```typescript
// Subscribe to value
const unsubscribe = singleton.subscribe(
	(value) => console.log('Value:', value), // T | undefined
	(error) => console.error('Error:', error)
);

// Replace entire value
await singleton.set({ field: 'new value' });

// Merge changes
await singleton.update({ field: 'updated' });

// Get adapter status
singleton.getStatus();
```

## Adapters

Adapters own all persistence, schema, validation, sync, and conflict resolution details.

### MemoryAdapter (Collections)

In-memory storage for testing and demos:

```typescript
import { MemoryAdapter, createCollection } from '@aicacia/db';

interface Document {
	id: string;
	name: string;
}

// Empty adapter
const adapter = new MemoryAdapter<Document>();

// Or with initial data
const adapter = new MemoryAdapter<Document>('id', [{ id: '1', name: 'Initial Doc' }]);

const collection = createCollection({
	id: 'docs',
	source: adapter,
	keyOf: (doc) => doc.id
});
```

### MemorySingletonAdapter (Singletons)

In-memory storage for singleton values:

```typescript
import { MemorySingletonAdapter, createSingleton } from '@aicacia/db';

interface Settings {
	theme: string;
}

const adapter = new MemorySingletonAdapter<Settings>();
const singleton = createSingleton({
	id: 'settings',
	source: adapter
});
```

### Custom Adapters

Implement `SourceAdapter<T>` or `SingletonSourceAdapter<T>`:

```typescript
import type { SourceAdapter, AdapterStatus } from '@aicacia/db';

export class MyAdapter<T> implements SourceAdapter<T> {
	subscribe(cte, onUpdate, onError) {
		// Execute CTE and emit results
		// Return unsubscribe function
	}

	async create(doc: T): Promise<void> {
		// Persist document
	}

	async update(id: string, changes: Partial<T>): Promise<void> {
		// Update document
	}

	async delete(id: string): Promise<void> {
		// Delete document
	}

	getStatus(): AdapterStatus {
		return { state: 'idle' };
	}
}
```

## Svelte Integration

Export `@aicacia/db/svelte` provides reactive store helpers:

```typescript
// collection.svelte.ts
export function collection<T>(query: IQueryBuilder<T>): { data: T[]; error: Error | null };

// singleton.svelte.ts
export function singleton<T>(source: ISingleton<T>): { data: T | undefined; error: Error | null };
```

Both automatically:

- Subscribe on mount
- Unsubscribe on unmount
- Update on data changes
- Provide error state

## Status

### Implemented ✅

- Core types and interfaces
- Collection and Singleton classes
- Query builder with filter, orderBy, limit, offset, with, subscribe
- CTE serialization protocol
- D2TS mutation filtering
- MemoryAdapter and MemorySingletonAdapter
- Svelte reactive stores
- Comprehensive tests

### In Progress 🔄

- IndexedDB adapter
- Remote adapter
- Additional examples
- Documentation & TypeDoc

## License

MIT OR Apache-2.0
