/**
 * D2TS (Doc2TypeScript) pipeline compilation for mutation result filtering
 */

import type { D2TSPipeline, QueryPredicate } from './types.js';

/**
 * Compile predicates into a D2TS mutation filter pipeline
 * This allows filtering mutation results before emitting to subscribers
 *
 * @internal This is used internally for query compilation
 */
export function compileMutationFilterPipeline<T>(predicates: QueryPredicate<T>[]): D2TSPipeline {
	const operations = predicates.map((_, index) => ({
		type: 'predicate_filter',
		id: `filter_${index}`
	}));

	return {
		operations
	};
}

/**
 * Apply a D2TS pipeline to a document
 * Evaluates all predicates in the pipeline
 */
export function applyMutationFilterPipeline<T>(doc: T, predicates: QueryPredicate<T>[]): boolean {
	return predicates.every((predicate) => {
		try {
			return predicate(doc);
		} catch {
			// If predicate throws, consider it non-matching
			return false;
		}
	});
}

/**
 * Filter an array of documents using a set of predicates
 */
export function filterDocumentsByPredicates<T>(docs: T[], predicates: QueryPredicate<T>[]): T[] {
	if (predicates.length === 0) return docs;

	return docs.filter((doc) => applyMutationFilterPipeline(doc, predicates));
}
