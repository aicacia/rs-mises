import type { UnsubscribeFn } from './types.js';
import type { IQueryBuilder } from './queryBuilder.js';

export interface StoreState<T> {
	data: T;
	error: Error | null;
	unsubscribe: UnsubscribeFn | null;
}

/**
 * Create a reactive collection store for use in Svelte components
 * Cleanup is handled automatically via Svelte effect
 *
 * Usage in component:
 * ```
 * import { collection } from '@aicacia/db/svelte';
 * const recipes = collection(recipesCollection.query());
 *
 * // In template:
 * {#each recipes.data as recipe (recipe.id)}
 *   <div>{recipe.name}</div>
 * {/each}
 * ```
 */
export function collection<T>(query: IQueryBuilder<T>): {
	data: T[];
	error: Error | null;
} {
	let data: T[] = $state([]);
	let error: Error | null = $state(null);
	let unsubscribe: UnsubscribeFn | null = null;

	$effect(() => {
		unsubscribe = query.subscribe(
			(docs: T[]) => {
				data = [...docs];
				error = null;
			},
			(err: Error) => {
				error = err;
			}
		);

		return () => {
			if (unsubscribe) {
				unsubscribe();
				unsubscribe = null;
			}
		};
	});

	return {
		get data() {
			return data;
		},
		get error() {
			return error;
		}
	};
}
