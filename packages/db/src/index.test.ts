/**
 * Basic tests for @aicacia/db core library
 */

import test from 'tape';
import { createCollection } from './createCollection.js';
import { MemoryAdapter } from './memory-adapter.js';
import { createCTE, evaluateCTE } from './cte.js';

// Test data interface
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

	const unsub = adapter.subscribe(cte, (docs) => {
		updates.push([...docs]);
	});

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

	const unsub = adapter.subscribe(cte, (docs) => {
		updates.push([...docs]);
	});

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
		.filter((doc) => doc.status === 'active')
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
		.filter((doc) => doc.status === 'active')
		.filter((doc) => doc.prepTime < 40)
		.subscribe((docs) => {
			updates.push([...docs]);
		});

	t.equal(updates.length, 1, 'Should receive one update');
	t.equal(updates[0].length, 2, 'Should have two matching documents');
	t.equal(
		updates[0].every((d) => d.prepTime < 40),
		true,
		'All should have prepTime < 40'
	);

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

	// Create
	await collection.create({
		id: '1',
		name: 'Pasta',
		status: 'active',
		tags: ['quick'],
		prepTime: 20
	});

	t.equal(updates.length, 2, 'Should receive update after create');
	t.equal(updates[1].length, 1, 'Should have one document');

	// Update
	await collection.update('1', { status: 'archived' });

	t.equal(updates.length, 3, 'Should receive update after update');
	t.equal(updates[2][0].status, 'archived', 'Should be archived');

	// Delete
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

	// After unsubscribe, adapter mutations shouldn't trigger updates
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
		.filter((doc) => doc.status === 'active')
		.orderBy('prepTime', 'asc')
		.limit(10);

	const cte = queryBuilder.toCTE();

	t.equal(cte.version, '1.0', 'CTE should have version');
	t.equal(cte.limit, 10, 'CTE should have limit');
	t.equal(cte.orderBy?.length, 1, 'CTE should have one orderBy clause');

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

	const query = collection.query().filter((doc) => doc.status === 'active');

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
