import type { ISingleton } from './types.js';

/**
 * Create a reactive singleton store for use in Svelte components
 * Cleanup is handled automatically via Svelte effect
 *
 * Usage in component:
 * ```
 * import { singleton } from '@aicacia/db/svelte';
 * const settings = singleton(settingsSingleton);
 *
 * // In template:
 * <div>{settings.data?.theme}</div>
 * ```
 */
export function singleton<T>(source: ISingleton<T>): {
	data: T | undefined;
	error: Error | null;
} {
	let data: T | undefined = $state(undefined);
	let error: Error | null = $state(null);
	let unsubscribe: (() => void) | null = null;

	$effect(() => {
		// Subscribe to singleton updates
		unsubscribe = source.subscribe(
			(value: T | undefined) => {
				data = value;
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
