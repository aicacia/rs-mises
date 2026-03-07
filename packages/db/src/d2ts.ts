/**
 * Apply CTE filters to documents
 *
 * D2TS pipeline for differential computation - uses shared filter engine
 */

import {
	D2,
	MessageType,
	filter,
	orderBy,
	output,
	type IStreamBuilder,
	type KeyValue,
	type Message
} from '@electric-sql/d2ts';
import type { CTE } from './cte.js';
import { applyFilter } from './filterEngine.js';
import { createItemSortFunction } from './utils.js';

export type KeyedChange<T> = [KeyValue<string, T>, number];

export interface IncrementalQuery<T> {
	getResults(): T[];
	applyChanges(changes: KeyedChange<T>[]): T[];
}

interface MaterializedDoc<T> {
	doc: T;
	multiplicity: number;
}

function toResultArray<T>(materialized: Map<string, MaterializedDoc<T>>, cte: CTE<T>): T[] {
	const results: T[] = [];

	for (const { doc, multiplicity } of materialized.values()) {
		const copies = Math.max(0, multiplicity);
		for (let index = 0; index < copies; index++) {
			results.push(doc);
		}
	}

	if (cte.orderBy && cte.orderBy.length > 0) {
		results.sort(createItemSortFunction(cte.orderBy));
	}

	if (cte.offset !== undefined && cte.offset > 0) {
		results.splice(0, cte.offset);
	}

	if (cte.limit !== undefined && cte.limit > 0 && results.length > cte.limit) {
		results.length = cte.limit;
	}

	return results;
}

export function createIncrementalQuery<T>(cte: CTE<T>): IncrementalQuery<T> {
	const graph = new D2({ initialFrontier: 0 });
	const input = graph.newInput<KeyValue<string, T>>();

	let stream: IStreamBuilder<KeyValue<string, T>> = input;
	if (cte.filters && cte.filters.length > 0) {
		stream = stream.pipe(
			filter(([_, doc]) => {
				return cte.filters!.every((entry) => applyFilter(entry, doc));
			})
		);
	}

	if (cte.orderBy && cte.orderBy.length > 0) {
		stream = stream.pipe(
			orderBy<KeyValue<string, T>, T>((doc) => doc, {
				comparator: createItemSortFunction(cte.orderBy!)
			})
		);
	}

	const materialized = new Map<string, MaterializedDoc<T>>();
	let hasPendingResultDelta = false;

	stream.pipe(
		output((message: Message<KeyValue<string, T>>) => {
			if (message.type !== MessageType.DATA) {
				return;
			}

			for (const [[key, doc], multiplicity] of message.data.collection.getInner()) {
				const existing = materialized.get(key);
				const existingMultiplicity = existing?.multiplicity ?? 0;
				const nextMultiplicity = existingMultiplicity + multiplicity;

				if (nextMultiplicity <= 0) {
					if (existing) {
						materialized.delete(key);
						hasPendingResultDelta = true;
					}
					continue;
				}

				if (!existing || existing.doc !== doc || existingMultiplicity !== nextMultiplicity) {
					materialized.set(key, {
						doc,
						multiplicity: nextMultiplicity
					});
					hasPendingResultDelta = true;
				}
			}
		})
	);

	graph.finalize();

	let version = 1;
	let cachedResults: T[] = [];

	const computeResults = (): T[] => {
		if (cte.orderBy && cte.orderBy.length > 0) {
			cachedResults = toResultArray(materialized, cte);
			return cachedResults;
		}

		cachedResults = toResultArray(materialized, {
			...cte,
			orderBy: undefined
		});
		return cachedResults;
	};

	return {
		getResults(): T[] {
			return cachedResults;
		},

		applyChanges(changes: KeyedChange<T>[]): T[] {
			if (changes.length === 0) {
				return cachedResults;
			}

			hasPendingResultDelta = false;
			input.sendData(version, changes);
			input.sendFrontier(version + 1);
			version += 1;
			graph.run();

			if (!hasPendingResultDelta) {
				return cachedResults;
			}

			return computeResults();
		}
	};
}
