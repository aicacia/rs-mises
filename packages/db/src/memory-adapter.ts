/**
 * MemoryAdapter - Reference implementation using in-memory storage
 * Good for testing and demos; no persistence
 */

import type {
	AdapterStatus,
	CTE,
	SingletonSourceAdapter,
	SourceAdapter,
	UnsubscribeFn
} from './types.js';
import { evaluateCTE } from './cte.js';

interface SubscriptionEntry<T> {
	cte: CTE;
	onUpdate: (docs: T[]) => void;
	onError: (error: Error) => void;
}

/**
 * MemoryAdapter - stores documents in memory
 */
export class MemoryAdapter<T extends Record<string, unknown>> implements SourceAdapter<T> {
	private _documents: Map<string, T> = new Map();
	private _subscriptions: SubscriptionEntry<T>[] = [];
	private _keyField: string;
	private _status: AdapterStatus = { state: 'idle' };

	constructor(keyField: string = 'id', initialDocs?: T[]) {
		this._keyField = keyField;
		if (initialDocs) {
			for (const doc of initialDocs) {
				const key = String(doc[keyField]);
				this._documents.set(key, doc);
			}
		}
	}

	subscribe(
		cte: CTE,
		onUpdate: (docs: T[]) => void,
		onError: (error: Error) => void
	): UnsubscribeFn {
		const entry: SubscriptionEntry<T> = {
			cte,
			onUpdate,
			onError
		};

		this._subscriptions.push(entry);

		// Immediately send current matching results
		try {
			const allDocs = Array.from(this._documents.values());
			const results = evaluateCTE(cte, allDocs);
			onUpdate(results);
		} catch (error) {
			onError(error instanceof Error ? error : new Error(String(error)));
		}

		// Return unsubscribe function
		return () => {
			const index = this._subscriptions.indexOf(entry);
			if (index >= 0) {
				this._subscriptions.splice(index, 1);
			}
		};
	}

	async create(doc: T): Promise<void> {
		const key = String(doc[this._keyField]);
		if (!key || key === 'undefined') {
			throw new Error(`Document missing required key field "${this._keyField}"`);
		}
		this._documents.set(key, doc);
		this._notifySubscribers();
	}

	async update(id: string, changes: Partial<T>): Promise<void> {
		const doc = this._documents.get(id);
		if (!doc) {
			throw new Error(`Document with id "${id}" not found`);
		}
		const updated = { ...doc, ...changes };
		this._documents.set(id, updated);
		this._notifySubscribers();
	}

	async delete(id: string): Promise<void> {
		if (!this._documents.has(id)) {
			throw new Error(`Document with id "${id}" not found`);
		}
		this._documents.delete(id);
		this._notifySubscribers();
	}

	getStatus(): AdapterStatus {
		return this._status;
	}

	/**
	 * Get all documents (for testing)
	 */
	getAllDocuments(): T[] {
		return Array.from(this._documents.values());
	}

	/**
	 * Clear all documents (for testing)
	 */
	clear(): void {
		this._documents.clear();
		this._notifySubscribers();
	}

	private _notifySubscribers(): void {
		const allDocs = Array.from(this._documents.values());

		for (const entry of this._subscriptions) {
			try {
				const results = evaluateCTE(entry.cte, allDocs);
				entry.onUpdate(results);
			} catch (error) {
				entry.onError(error instanceof Error ? error : new Error(String(error)));
			}
		}
	}
}

interface SingletonSubscriptionEntry<T> {
	onUpdate: (value: T | undefined) => void;
	onError: (error: Error) => void;
}

/**
 * MemorySingletonAdapter - in-memory singleton storage
 * Stores at most one document for non-ID-based data (settings, config, etc.)
 */
export class MemorySingletonAdapter<T> implements SingletonSourceAdapter<T> {
	private _value: T | undefined;
	private _subscriptions: SingletonSubscriptionEntry<T>[] = [];
	private _status: AdapterStatus = { state: 'idle' };

	constructor(initialValue?: T) {
		this._value = initialValue;
	}

	subscribe(
		onUpdate: (value: T | undefined) => void,
		onError: (error: Error) => void
	): UnsubscribeFn {
		const entry: SingletonSubscriptionEntry<T> = {
			onUpdate,
			onError
		};

		this._subscriptions.push(entry);

		// Immediately send current value
		try {
			onUpdate(this._value);
		} catch (error) {
			onError(error instanceof Error ? error : new Error(String(error)));
		}

		// Return unsubscribe function
		return () => {
			const index = this._subscriptions.indexOf(entry);
			if (index >= 0) {
				this._subscriptions.splice(index, 1);
			}
		};
	}

	async set(doc: T): Promise<void> {
		this._value = doc;
		this._notifySubscribers();
	}

	async update(changes: Partial<T>): Promise<void> {
		if (this._value === undefined) {
			throw new Error('Singleton is not initialized; cannot update');
		}
		this._value = { ...this._value, ...changes };
		this._notifySubscribers();
	}

	getStatus(): AdapterStatus {
		return this._status;
	}

	/**
	 * Get current value (for testing)
	 */
	getValue(): T | undefined {
		return this._value;
	}

	/**
	 * Clear the value (for testing)
	 */
	clear(): void {
		this._value = undefined;
		this._notifySubscribers();
	}

	private _notifySubscribers(): void {
		for (const entry of this._subscriptions) {
			try {
				entry.onUpdate(this._value);
			} catch (error) {
				entry.onError(error instanceof Error ? error : new Error(String(error)));
			}
		}
	}
}
