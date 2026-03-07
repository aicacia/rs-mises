import type {
	AdapterStatus,
	SingletonSourceAdapter,
	SourceAdapter,
	UnsubscribeFn
} from './types.js';
import type { CTE } from './cte.js';
import { applyCTE } from './filterEngine.js';
import { toError } from './utils.js';

interface SubscriptionEntry<T> {
	cte: CTE<T>;
	onUpdate: (docs: T[]) => void;
	onError: (error: Error) => void;
}

/** In-memory collection adapter intended for tests and demos. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export class MemoryAdapter<T extends Record<string, any>> implements SourceAdapter<T> {
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
		cte: CTE<T>,
		onUpdate: (docs: T[]) => void,
		onError: (error: Error) => void
	): UnsubscribeFn {
		const entry: SubscriptionEntry<T> = {
			cte,
			onUpdate,
			onError
		};

		this._subscriptions.push(entry);

		try {
			const allDocs = Array.from(this._documents.values());
			const results = applyCTE(cte, allDocs);
			onUpdate(results);
		} catch (error) {
			onError(toError(error));
		}

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

	/** Return all stored documents. */
	getAllDocuments(): T[] {
		return Array.from(this._documents.values());
	}

	/** Remove all stored documents. */
	clear(): void {
		this._documents.clear();
		this._notifySubscribers();
	}

	private _notifySubscribers(): void {
		const allDocs = Array.from(this._documents.values());

		for (const entry of this._subscriptions) {
			try {
				const results = applyCTE(entry.cte, allDocs);
				entry.onUpdate(results);
			} catch (error) {
				entry.onError(toError(error));
			}
		}
	}
}

interface SingletonSubscriptionEntry<T> {
	onUpdate: (value: T | undefined) => void;
	onError: (error: Error) => void;
}

/** In-memory singleton adapter intended for tests and demos. */
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

		try {
			onUpdate(this._value);
		} catch (error) {
			onError(toError(error));
		}

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

	/** Return the current value. */
	getValue(): T | undefined {
		return this._value;
	}

	/** Clear the current value. */
	clear(): void {
		this._value = undefined;
		this._notifySubscribers();
	}

	private _notifySubscribers(): void {
		for (const entry of this._subscriptions) {
			try {
				entry.onUpdate(this._value);
			} catch (error) {
				entry.onError(toError(error));
			}
		}
	}
}
