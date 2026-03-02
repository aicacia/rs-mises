/**
 * Factory functions for creating collections and singletons
 */

import type { CollectionConfig, ICollection, ISingleton, SingletonConfig } from './types.js';
import { Collection } from './collection.js';
import { Singleton } from './singleton.js';

/**
 * Create a new collection
 */
export function createCollection<T>(config: CollectionConfig<T>): ICollection<T> {
	return new Collection(config);
}

/**
 * Create a new singleton
 */
export function createSingleton<T>(config: SingletonConfig<T>): ISingleton<T> {
	return new Singleton(config);
}
