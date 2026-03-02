/**
 * Singleton<T> - at-most-one document collection for non-ID-based data
 */

import type {
	AdapterStatus,
	ISingleton,
	SingletonConfig,
	SingletonSourceAdapter,
	UnsubscribeFn
} from './types.js';

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
	private _currentValue: T | undefined;

	constructor(config: SingletonConfig<T>) {
		this.id = config.id;
		this._source = config.source;
		this._currentValue = config.defaultValue;
	}

	subscribe(
		onUpdate: (value: T | undefined) => void,
		onError?: (error: Error) => void
	): UnsubscribeFn {
		const subscription: SingletonSubscription<T> = {
			onUpdate,
			onError: onError || (() => {})
		};

		this._subscriptions.add(subscription);

		// If this is the first subscription, start listening to adapter
		if (this._subscriptions.size === 1) {
			this._startAdapterSubscription();
		}

		// Send current value immediately
		try {
			onUpdate(this._currentValue);
		} catch (error) {
			onError?.(error instanceof Error ? error : new Error(String(error)));
		}

		// Return unsubscribe function
		return () => {
			this._subscriptions.delete(subscription);

			// If no more subscriptions, stop listening to adapter
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
	}

	private _handleAdapterUpdate(value: T | undefined): void {
		this._currentValue = value;

		// Notify all subscribers
		for (const subscription of this._subscriptions) {
			try {
				subscription.onUpdate(this._currentValue);
			} catch (error) {
				this._handleAdapterError(error instanceof Error ? error : new Error(String(error)));
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
