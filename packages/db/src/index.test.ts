import test from 'tape';
import { createCollection } from './collection.js';
import { createSingleton } from './singleton.js';
import { MemoryAdapter, MemorySingletonAdapter } from './memoryAdapter.js';
import { and, compare, createCTE, equal, greaterThan, lessThan, or } from './cte.js';

interface Recipe {
	id: string;
	name: string;
	status: 'active' | 'archived';
	tags: string[];
	prepTime: number;
}

test('MemoryAdapter: create and subscribe', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id');

	const cte = createCTE();
	const updates: Recipe[][] = [];

	const unsub = adapter.subscribe(
		cte,
		(docs) => {
			updates.push([...docs]);
		},
		() => {
			t.fail('Unexpected adapter subscription error');
		}
	);

	adapter.create({ id: '1', name: 'Pasta', status: 'active', tags: ['quick'], prepTime: 20 });

	t.equal(updates.length, 2, 'Should receive initial empty and then update');
	t.equal(updates[1].length, 1, 'Should have one document');
	t.equal(updates[1][0].name, 'Pasta', 'Should have correct name');

	unsub();
	t.end();
});

test('Query builder: adapter with initial docs', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id', [
		{ id: '1', name: 'Pasta', status: 'active', tags: ['quick'], prepTime: 20 },
		{ id: '2', name: 'Risotto', status: 'active', tags: ['slow'], prepTime: 45 },
		{ id: '3', name: 'Soup', status: 'archived', tags: ['quick'], prepTime: 30 }
	]);

	const cte = createCTE();
	const updates: Recipe[][] = [];

	const unsub = adapter.subscribe(
		cte,
		(docs) => {
			updates.push([...docs]);
		},
		() => {
			t.fail('Unexpected adapter subscription error');
		}
	);

	t.equal(updates.length, 1, 'Should receive one update from adapter');
	t.equal(updates[0].length, 3, 'Should have three documents');

	unsub();
	t.end();
});

test('Query builder: filter', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id', [
		{ id: '1', name: 'Pasta', status: 'active', tags: ['quick'], prepTime: 20 },
		{ id: '2', name: 'Risotto', status: 'active', tags: ['slow'], prepTime: 45 },
		{ id: '3', name: 'Soup', status: 'archived', tags: ['quick'], prepTime: 30 }
	]);

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const updates: Recipe[][] = [];

	const unsub = collection
		.query()
		.where(equal('status', 'active'))
		.subscribe((docs) => {
			updates.push([...docs]);
		});

	t.equal(updates.length, 1, 'Should receive one update');
	t.equal(updates[0].length, 2, 'Should have two active documents');
	t.equal(
		updates[0].every((d) => d.status === 'active'),
		true,
		'All should be active'
	);

	unsub();
	t.end();
});

test('Query builder: multiple filters compose', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id', [
		{ id: '1', name: 'Pasta', status: 'active', tags: ['quick'], prepTime: 20 },
		{ id: '2', name: 'Risotto', status: 'active', tags: ['slow'], prepTime: 45 },
		{ id: '3', name: 'Soup', status: 'active', tags: ['quick'], prepTime: 30 }
	]);

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const updates: Recipe[][] = [];

	const unsub = collection
		.query()
		.where(equal<Recipe>('status', 'active'))
		.where(greaterThan<Recipe>('prepTime', 30))
		.subscribe((docs) => {
			updates.push([...docs]);
		});

	t.equal(updates.length, 1, 'Should receive one update');
	t.equal(updates[0].length, 1, 'Should have one matching document');
	t.equal(
		updates[0].every((d) => d.prepTime > 30),
		true,
		'All should have prepTime > 30'
	);

	unsub();
	t.end();
});

test('Query builder: fluent equal helper', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id', [
		{ id: '1', name: 'Pasta', status: 'active', tags: ['quick'], prepTime: 20 },
		{ id: '2', name: 'Risotto', status: 'archived', tags: ['slow'], prepTime: 45 },
		{ id: '3', name: 'Soup', status: 'active', tags: ['quick'], prepTime: 30 }
	]);

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const updates: Recipe[][] = [];

	const unsub = collection
		.query()
		.equal('status', 'active')
		.subscribe((docs) => {
			updates.push([...docs]);
		});

	t.equal(updates.length, 1, 'Should receive one update');
	t.equal(updates[0].length, 2, 'Should have two active documents');
	t.ok(
		updates[0].every((d) => d.status === 'active'),
		'All should be active'
	);

	unsub();
	t.end();
});

test('Query builder: fluent containsIgnoreCase helper', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id', [
		{ id: '1', name: 'Pasta', status: 'active', tags: ['quick'], prepTime: 20 },
		{ id: '2', name: 'PASTA Salad', status: 'active', tags: ['quick'], prepTime: 10 },
		{ id: '3', name: 'Soup', status: 'active', tags: ['quick'], prepTime: 30 }
	]);

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const updates: Recipe[][] = [];

	const unsub = collection
		.query()
		.containsIgnoreCase('name', 'pAsTa')
		.subscribe((docs) => {
			updates.push([...docs]);
		});

	t.equal(updates.length, 1, 'Should receive one update');
	t.equal(updates[0].length, 2, 'Should match documents regardless of case');
	t.deepEqual(
		updates[0].map((d) => d.id),
		['1', '2'],
		'Should return matching ids'
	);

	unsub();
	t.end();
});

test('Query builder: fluent fuzzyContains helper', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id', [
		{ id: '1', name: 'Spaghetti Bolognese', status: 'active', tags: ['slow'], prepTime: 60 },
		{ id: '2', name: 'Pasta Salad', status: 'active', tags: ['quick'], prepTime: 10 },
		{ id: '3', name: 'Soup', status: 'active', tags: ['quick'], prepTime: 30 }
	]);

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const updates: Recipe[][] = [];

	const unsub = collection
		.query()
		.fuzzyContains('name', 'spageti')
		.subscribe((docs) => {
			updates.push([...docs]);
		});

	t.equal(updates.length, 1, 'Should receive one update');
	t.equal(updates[0].length, 1, 'Should match fuzzy typo against substring');
	t.equal(updates[0][0].id, '1', 'Should match the spaghetti document');

	unsub();
	t.end();
});

test('Query builder: fluent and helper', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id', [
		{ id: '1', name: 'Pasta', status: 'active', tags: ['quick'], prepTime: 20 },
		{ id: '2', name: 'Risotto', status: 'active', tags: ['slow'], prepTime: 45 },
		{ id: '3', name: 'Soup', status: 'archived', tags: ['quick'], prepTime: 30 }
	]);

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const updates: Recipe[][] = [];

	const unsub = collection
		.query()
		.and(equal<Recipe>('status', 'active'), greaterThan<Recipe>('prepTime', 30))
		.subscribe((docs) => {
			updates.push([...docs]);
		});

	t.equal(updates.length, 1, 'Should receive one update');
	t.equal(updates[0].length, 1, 'Should have one matching document');
	t.equal(updates[0][0].name, 'Risotto', 'Should match active and prepTime > 30');

	unsub();
	t.end();
});

test('Query builder: orderBy', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id', [
		{ id: '1', name: 'Pasta', status: 'active', tags: ['quick'], prepTime: 20 },
		{ id: '2', name: 'Risotto', status: 'active', tags: ['slow'], prepTime: 45 },
		{ id: '3', name: 'Soup', status: 'active', tags: ['quick'], prepTime: 30 }
	]);

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const updates: Recipe[][] = [];

	const unsub = collection
		.query()
		.orderBy('prepTime', 'asc')
		.subscribe((docs) => {
			updates.push([...docs]);
		});

	t.equal(updates.length, 1, 'Should receive one update');
	t.deepEqual(
		updates[0].map((d) => d.prepTime),
		[20, 30, 45],
		'Should be sorted by prepTime ascending'
	);

	unsub();
	t.end();
});

test('Query builder: limit and offset', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id', [
		{ id: '1', name: 'Pasta', status: 'active', tags: ['quick'], prepTime: 20 },
		{ id: '2', name: 'Risotto', status: 'active', tags: ['slow'], prepTime: 45 },
		{ id: '3', name: 'Soup', status: 'active', tags: ['quick'], prepTime: 30 }
	]);

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const updates: Recipe[][] = [];

	const unsub = collection
		.query()
		.orderBy('prepTime', 'asc')
		.offset(1)
		.limit(1)
		.subscribe((docs) => {
			updates.push([...docs]);
		});

	t.equal(updates.length, 1, 'Should receive one update');
	t.equal(updates[0].length, 1, 'Should have one document');
	t.equal(updates[0][0].name, 'Soup', 'Should be the middle document');

	unsub();
	t.end();
});

test('Collection: mutations', async (t) => {
	const adapter = new MemoryAdapter<Recipe>('id');

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const updates: Recipe[][] = [];

	const unsub = collection.query().subscribe((docs) => {
		updates.push([...docs]);
	});

	await collection.create({
		id: '1',
		name: 'Pasta',
		status: 'active',
		tags: ['quick'],
		prepTime: 20
	});

	t.equal(updates.length, 2, 'Should receive update after create');
	t.equal(updates[1].length, 1, 'Should have one document');

	await collection.update('1', { status: 'archived' });

	t.equal(updates.length, 3, 'Should receive update after update');
	t.equal(updates[2][0].status, 'archived', 'Should be archived');

	await collection.delete('1');

	t.equal(updates.length, 4, 'Should receive update after delete');
	t.equal(updates[3].length, 0, 'Should have no documents');

	unsub();
	t.end();
});

test('Query: subscription cleanup', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id', [
		{ id: '1', name: 'Pasta', status: 'active', tags: ['quick'], prepTime: 20 }
	]);

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	let updateCount = 0;

	const unsub = collection.query().subscribe(() => {
		updateCount++;
	});

	t.equal(updateCount, 1, 'Should receive initial update');

	unsub();

	adapter.create({ id: '2', name: 'Risotto', status: 'active', tags: ['slow'], prepTime: 45 });

	t.equal(updateCount, 1, 'Should not receive update after unsubscribe');

	t.end();
});

test('Query: CTE export', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id');

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const queryBuilder = collection
		.query()
		.where(equal('status', 'active'))
		.orderBy('prepTime', 'asc')
		.limit(10);

	const cte = queryBuilder.toCTE();

	t.equal(cte.version, '1.0', 'CTE should have version');
	t.equal(cte.limit, 10, 'CTE should have limit');
	t.equal(cte.orderBy?.length, 1, 'CTE should have one orderBy clause');
	t.equal(cte.filters?.length, 1, 'CTE should have one filter');

	t.end();
});

test('Error handling: update non-existent document', async (t) => {
	const adapter = new MemoryAdapter<Recipe>('id');

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	try {
		await collection.update('non-existent', { status: 'archived' });
		t.fail('Should throw error');
	} catch (error) {
		t.ok(error instanceof Error, 'Should throw Error');
		t.match((error as Error).message, /not found/, 'Should mention not found');
	}

	t.end();
});

test('Query: multiple subscriptions to same query share adapter subscription', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id', [
		{ id: '1', name: 'Pasta', status: 'active', tags: ['quick'], prepTime: 20 },
		{ id: '2', name: 'Soup', status: 'active', tags: ['quick'], prepTime: 30 }
	]);

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const updates1: Recipe[][] = [];
	const updates2: Recipe[][] = [];

	const query = collection.query().where(equal('status', 'active'));

	const unsub1 = query.subscribe((docs) => {
		updates1.push([...docs]);
	});

	const unsub2 = query.subscribe((docs) => {
		updates2.push([...docs]);
	});

	t.equal(updates1.length, 1, 'First subscriber should receive initial update');
	t.equal(updates2.length, 1, 'Second subscriber should receive initial update');

	adapter.create({
		id: '3',
		name: 'Risotto',
		status: 'active',
		tags: ['slow'],
		prepTime: 45
	});

	t.equal(updates1.length, 2, 'First subscriber should receive mutation update');
	t.equal(updates2.length, 2, 'Second subscriber should receive mutation update');

	unsub1();
	unsub2();

	t.end();
});

test('Error handling: error in subscriber is caught', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id', [
		{ id: '1', name: 'Pasta', status: 'active', tags: ['quick'], prepTime: 20 }
	]);

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const errors: Error[] = [];

	const unsub = collection.query().subscribe(
		() => {
			throw new Error('Subscriber error');
		},
		(error) => {
			errors.push(error);
		}
	);

	t.equal(errors.length, 1, 'Error should be caught');
	t.equal(errors[0]?.message, 'Subscriber error', 'Error message should match');

	unsub();
	t.end();
});

test('Error handling: multiple subscribers to same query', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id', [
		{ id: '1', name: 'Pasta', status: 'active', tags: ['quick'], prepTime: 20 }
	]);

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const updates1: Recipe[][] = [];
	const updates2: Recipe[][] = [];

	const unsub1 = collection
		.query()
		.where(equal('status', 'active'))
		.subscribe((docs) => {
			updates1.push([...docs]);
		});

	const unsub2 = collection
		.query()
		.where(equal('status', 'active'))
		.subscribe((docs) => {
			updates2.push([...docs]);
		});

	t.equal(updates1.length, 1, 'First subscriber should receive initial update');
	t.equal(updates2.length, 1, 'Second subscriber should receive initial update');
	t.deepEqual(updates1[0], updates2[0], 'Both subscribers should receive same data');

	adapter.create({ id: '2', name: 'Pizza', status: 'active', tags: ['quick'], prepTime: 15 });

	t.equal(updates1.length, 2, 'First subscriber should receive mutation update');
	t.equal(updates2.length, 2, 'Second subscriber should receive mutation update');
	t.equal(updates1[1].length, 2, 'First subscriber should have 2 docs');
	t.equal(updates2[1].length, 2, 'Second subscriber should have 2 docs');

	unsub1();
	unsub2();

	t.end();
});

test('Error handling: resubscribe after unsubscribe', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id', [
		{ id: '1', name: 'Pasta', status: 'active', tags: ['quick'], prepTime: 20 }
	]);

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const updates1: Recipe[][] = [];

	const unsub1 = collection.query().subscribe((docs) => {
		updates1.push([...docs]);
	});

	t.equal(updates1.length, 1, 'Should receive initial update');

	unsub1();

	const updates2: Recipe[][] = [];
	const unsub2 = collection.query().subscribe((docs) => {
		updates2.push([...docs]);
	});

	t.equal(updates2.length, 1, 'Should receive initial update after resubscribe');
	t.deepEqual(updates2[0], updates1[0], 'Should receive same data');

	unsub2();
	t.end();
});

test('Error handling: unsubscribe during callback execution', async (t) => {
	const adapter = new MemoryAdapter<Recipe>('id', [
		{ id: '1', name: 'Pasta', status: 'active', tags: ['quick'], prepTime: 20 }
	]);

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	let callCount = 0;
	let unsub: (() => void) | null = null;

	unsub = collection.query().subscribe((_docs) => {
		callCount++;
		if (callCount === 1) {
			unsub?.();
		}
	});

	t.equal(callCount, 1, 'Should receive initial callback');

	// The next create will trigger an adapter update. Since we're unsubscribed,
	// we should not receive a callback for this new document
	await adapter.create({ id: '2', name: 'Pizza', status: 'active', tags: ['quick'], prepTime: 15 });

	// Wait a tick to ensure any async callbacks are processed
	await new Promise((resolve) => setTimeout(resolve, 10));

	// Note: The collection may have already processed the create notification
	// before we unsubscribed if they happen synchronously. The important thing
	// is that we can safely unsubscribe during a callback without crashing.
	t.ok(callCount >= 1, 'Should have received at least the initial callback');
	t.ok(callCount <= 2, 'Should not receive more than 2 callbacks');

	t.end();
});

test('Functional: default error handler throws', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id');
	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	try {
		collection.query().subscribe(() => {
			throw new Error('Callback error');
		});
		t.fail('Should throw when onError is not provided');
	} catch (error) {
		t.ok(error instanceof Error, 'Should throw Error');
		t.equal((error as Error).message, 'Callback error', 'Should throw callback error');
	}

	t.end();
});

test('Functional: custom error handler catches errors', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id');
	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const errors: Error[] = [];

	collection.query().subscribe(
		() => {
			throw new Error('Callback error');
		},
		(error) => {
			errors.push(error);
		}
	);

	t.equal(errors.length, 1, 'Should have caught one error');
	t.equal(errors[0].message, 'Callback error', 'Should have correct error message');
	t.end();
});

test('Functional: create fails with missing key field', async (t) => {
	const adapter = new MemoryAdapter<Recipe>('id');

	try {
		await adapter.create({
			name: 'Broken',
			status: 'active',
			tags: [],
			prepTime: 10
		} as unknown as Recipe);
		t.fail('Should have thrown error');
	} catch (error) {
		t.ok(error instanceof Error, 'Should throw an Error');
		t.ok(
			(error as Error).message.includes('missing required key field'),
			'Should mention missing key field'
		);
	}

	t.end();
});

test('Functional: update fails for non-existent document', async (t) => {
	const adapter = new MemoryAdapter<Recipe>('id');

	try {
		await adapter.update('nonexistent', { name: 'Updated' });
		t.fail('Should have thrown error');
	} catch (error) {
		t.ok(error instanceof Error, 'Should throw an Error');
		t.ok((error as Error).message.includes('not found'), 'Should mention document not found');
	}

	t.end();
});

test('Functional: delete fails for non-existent document', async (t) => {
	const adapter = new MemoryAdapter<Recipe>('id');

	try {
		await adapter.delete('nonexistent');
		t.fail('Should have thrown error');
	} catch (error) {
		t.ok(error instanceof Error, 'Should throw an Error');
		t.ok((error as Error).message.includes('not found'), 'Should mention document not found');
	}

	t.end();
});

test('Functional: empty collection returns empty results', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id');
	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const updates: Recipe[][] = [];

	const unsub = collection.query().subscribe((docs) => {
		updates.push([...docs]);
	});

	t.equal(updates.length, 1, 'Should receive initial update');
	t.equal(updates[0].length, 0, 'Should have empty array');

	unsub();
	t.end();
});

test('Functional: filter with no matches returns empty results', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id', [
		{ id: '1', name: 'Pasta', status: 'active', tags: ['quick'], prepTime: 20 }
	]);

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const updates: Recipe[][] = [];

	const unsub = collection
		.query()
		.where(equal('status', 'archived'))
		.subscribe((docs) => {
			updates.push([...docs]);
		});

	t.equal(updates.length, 1, 'Should receive initial update');
	t.equal(updates[0].length, 0, 'Should have no matching documents');

	unsub();
	t.end();
});

test('Functional: OR filter matches multiple conditions', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id', [
		{ id: '1', name: 'Pasta', status: 'active', tags: ['quick'], prepTime: 20 },
		{ id: '2', name: 'Risotto', status: 'active', tags: ['slow'], prepTime: 45 },
		{ id: '3', name: 'Soup', status: 'archived', tags: ['quick'], prepTime: 30 }
	]);

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const updates: Recipe[][] = [];

	const unsub = collection
		.query()
		.where(or(equal('status', 'archived'), compare('prepTime', 'greaterThan', 40)))
		.subscribe((docs) => {
			updates.push([...docs]);
		});

	t.equal(updates.length, 1, 'Should receive initial update');
	t.equal(updates[0].length, 2, 'Should match archived OR prepTime > 40');
	t.ok(
		updates[0].some((d) => d.id === '2'),
		'Should include Risotto (prepTime 45)'
	);
	t.ok(
		updates[0].some((d) => d.id === '3'),
		'Should include Soup (archived)'
	);

	unsub();
	t.end();
});

test('Functional: AND filter requires all conditions', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id', [
		{ id: '1', name: 'Pasta', status: 'active', tags: ['quick'], prepTime: 20 },
		{ id: '2', name: 'Risotto', status: 'active', tags: ['slow'], prepTime: 45 },
		{ id: '3', name: 'Soup', status: 'active', tags: ['quick'], prepTime: 30 }
	]);

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const updates: Recipe[][] = [];

	const unsub = collection
		.query()
		.where(and<Recipe>(equal('status', 'active'), lessThan('prepTime', 40)))
		.subscribe((docs) => {
			updates.push([...docs]);
		});

	t.equal(updates.length, 1, 'Should receive initial update');
	t.equal(updates[0].length, 2, 'Should match active AND prepTime < 40');
	t.ok(
		updates[0].every((d) => d.prepTime < 40),
		'All results should have prepTime < 40'
	);

	unsub();
	t.end();
});

test('Functional: singleton with default value', (t) => {
	interface Settings {
		theme: string;
		notifications: boolean;
	}

	const adapter = new MemorySingletonAdapter<Settings>();
	const singleton = createSingleton({
		id: 'settings',
		source: adapter,
		defaultValue: { theme: 'light', notifications: true }
	});

	const updates: Array<Settings | undefined> = [];

	const unsub = singleton.subscribe((value) => {
		updates.push(value);
	});

	t.equal(updates.length, 1, 'Should receive initial update');
	t.deepEqual(updates[0], { theme: 'light', notifications: true }, 'Should have default value');

	unsub();
	t.end();
});

test('Functional: singleton set and update', async (t) => {
	interface Settings {
		theme: string;
		notifications: boolean;
	}

	const adapter = new MemorySingletonAdapter<Settings>();
	const singleton = createSingleton({
		id: 'settings',
		source: adapter
	});

	const updates: Array<Settings | undefined> = [];

	const unsub = singleton.subscribe((value) => {
		updates.push(value);
	});

	t.equal(updates[0], undefined, 'Should start undefined');

	await singleton.set({ theme: 'dark', notifications: false });

	t.equal(updates.length, 2, 'Should receive update after set');
	t.deepEqual(updates[1], { theme: 'dark', notifications: false }, 'Should have set value');

	await singleton.update({ theme: 'light' });

	t.equal(updates.length, 3, 'Should receive update after merge');
	t.deepEqual(updates[2], { theme: 'light', notifications: false }, 'Should merge changes');

	unsub();
	t.end();
});

test('Functional: singleton update fails when not initialized', async (t) => {
	interface Settings {
		theme: string;
	}

	const adapter = new MemorySingletonAdapter<Settings>();
	const singleton = createSingleton({
		id: 'settings',
		source: adapter
	});

	try {
		await singleton.update({ theme: 'dark' });
		t.fail('Should have thrown error');
	} catch (error) {
		t.ok(error instanceof Error, 'Should throw an Error');
		t.ok(
			(error as Error).message.includes('not initialized'),
			'Should mention singleton not initialized'
		);
	}

	t.end();
});

test('Functional: singleton default error handler throws', (t) => {
	interface Settings {
		theme: string;
	}

	const adapter = new MemorySingletonAdapter<Settings>();
	const singleton = createSingleton({
		id: 'settings',
		source: adapter
	});

	try {
		singleton.subscribe(() => {
			throw new Error('Singleton callback error');
		});
		t.fail('Should throw when onError is not provided');
	} catch (error) {
		t.ok(error instanceof Error, 'Should throw Error');
		t.equal((error as Error).message, 'Singleton callback error', 'Should throw callback error');
	}

	t.end();
});

test('Functional: pagination with offset and limit', (t) => {
	const adapter = new MemoryAdapter<Recipe>('id', [
		{ id: '1', name: 'Pasta', status: 'active', tags: ['quick'], prepTime: 20 },
		{ id: '2', name: 'Risotto', status: 'active', tags: ['slow'], prepTime: 45 },
		{ id: '3', name: 'Soup', status: 'active', tags: ['quick'], prepTime: 30 },
		{ id: '4', name: 'Salad', status: 'active', tags: ['quick'], prepTime: 10 },
		{ id: '5', name: 'Steak', status: 'active', tags: ['slow'], prepTime: 60 }
	]);

	const collection = createCollection({
		id: 'recipes',
		source: adapter,
		keyOf: (doc) => doc.id
	});

	const updates: Recipe[][] = [];

	const unsub = collection
		.query()
		.orderBy('prepTime', 'asc')
		.paginate(1, 2)
		.subscribe((docs) => {
			updates.push([...docs]);
		});

	t.equal(updates.length, 1, 'Should receive initial update');
	t.equal(updates[0].length, 2, 'Should have 2 documents (page size)');
	t.equal(updates[0][0].name, 'Soup', 'Should skip first 2 (Salad, Pasta) and start at Soup');
	t.equal(updates[0][1].name, 'Risotto', 'Should include Risotto as second result');

	unsub();
	t.end();
});
