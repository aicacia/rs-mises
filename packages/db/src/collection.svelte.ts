import type { IQueryBuilder, UnsubscribeFn } from './types.js';

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
		// Subscribe to query updates
		unsubscribe = query.subscribe(
			(docs: T[]) => {
				// Trigger reactivity by creating a new array reference
				data = [...docs];
				error = null;
			},
			(err: Error) => {
				error = err;
			}
		);

		// Return cleanup function
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
