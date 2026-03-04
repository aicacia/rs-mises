/**
 * Singleton<T> - at-most-one document collection for non-ID-based data
 */

import type { AdapterStatus, SingletonSourceAdapter, UnsubscribeFn } from './types.js';
import { toError } from './utils.js';

/**
 * Singleton configuration
 */
export interface SingletonConfig<T> {
	/** Unique singleton identifier */
	id: string;

	/** Source adapter for this singleton */
	source: SingletonSourceAdapter<T>;

	/** Default value if not yet set */
	defaultValue?: T;
}

/**
 * Singleton interface
 */
export interface ISingleton<T> {
	readonly id: string;

	/**
	 * Subscribe to singleton value
	 */
	subscribe(
		onUpdate: (value: T | undefined) => void,
		onError?: (error: Error) => void
	): UnsubscribeFn;

	/**
	 * Replace entire singleton value
	 * @throws Error if set fails
	 */
	set(doc: T): Promise<void>;

	/**
	 * Merge changes into singleton value
	 * @throws Error if update fails
	 */
	update(changes: Partial<T>): Promise<void>;

	/**
	 * Get current adapter status
	 */
	getStatus(): AdapterStatus;
}

interface SingletonSubscription<T> {
	onUpdate: (value: T | undefined) => void;
	onError: (error: Error) => void;
}

/**
 * Singleton - represents a singleton value (at most one document)
 */
export class Singleton<T> implements ISingleton<T> {
	readonly id: string;
	private _source: SingletonSourceAdapter<T>;
	private _subscriptions: Set<SingletonSubscription<T>> = new Set();
	private _adapterUnsubscribe: UnsubscribeFn | null = null;
	private _cache: T | undefined;
	private _defaultValue: T | undefined;
	private _adapterSubscribed: boolean = false;

	constructor(config: SingletonConfig<T>) {
		this.id = config.id;
		this._source = config.source;
		this._cache = config.defaultValue;
		this._defaultValue = config.defaultValue;
	}

	subscribe(
		onUpdate: (value: T | undefined) => void,
		onError?: (error: Error) => void
	): UnsubscribeFn {
		const subscription: SingletonSubscription<T> = {
			onUpdate,
			onError:
				onError ||
				((error: Error) => {
					throw error;
				})
		};

		try {
			onUpdate(this._cache);
		} catch (error) {
			subscription.onError(toError(error));
		}

		this._subscriptions.add(subscription);

		if (this._subscriptions.size === 1) {
			this._startAdapterSubscription();
		}

		return () => {
			this._subscriptions.delete(subscription);

			if (this._subscriptions.size === 0) {
				this._stopAdapterSubscription();
			}
		};
	}

	async set(doc: T): Promise<void> {
		await this._source.set(doc);
	}

	async update(changes: Partial<T>): Promise<void> {
		await this._source.update(changes);
	}

	getStatus(): AdapterStatus {
		return this._source.getStatus();
	}

	private _startAdapterSubscription(): void {
		if (this._adapterSubscribed) {
			return;
		}

		this._adapterSubscribed = true;
		this._adapterUnsubscribe = this._source.subscribe(
			(value) => this._handleAdapterUpdate(value),
			(error) => this._handleAdapterError(error)
		);
	}

	private _stopAdapterSubscription(): void {
		if (this._adapterUnsubscribe) {
			this._adapterUnsubscribe();
			this._adapterUnsubscribe = null;
		}
		this._adapterSubscribed = false;
	}

	private _handleAdapterUpdate(value: T | undefined): void {
		const effectiveValue =
			value === undefined && this._defaultValue !== undefined ? this._defaultValue : value;

		if (this._cache !== effectiveValue) {
			this._cache = effectiveValue;

			for (const subscription of this._subscriptions) {
				try {
					subscription.onUpdate(this._cache);
				} catch (error) {
					this._handleAdapterError(toError(error));
				}
			}
		}
	}

	private _handleAdapterError(error: Error): void {
		for (const subscription of this._subscriptions) {
			try {
				subscription.onError(error);
			} catch {
				// Silently ignore errors from error handlers to prevent infinite loops
			}
		}
	}
}

export function createSingleton<T>(config: SingletonConfig<T>): ISingleton<T> {
	return new Singleton(config);
}
