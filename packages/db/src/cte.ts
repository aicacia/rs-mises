export interface CTE {
	version: '1.0';
	name?: string;
	columns?: string[];
	filters?: CTEFilter[];
	orderBy?: CTEOrderBy[];
	limit?: number;
	offset?: number;
	ctes?: Record<string, CTE>;
}

export type CTEFilter = CTEComparisonFilter | CTELogicalFilter;

export interface CTEComparisonFilter {
	type: 'comparison';
	operator:
		| 'equal'
		| 'notEqual'
		| 'greaterThan'
		| 'lessThan'
		| 'greaterThanOrEqual'
		| 'lessThanOrEqual'
		| 'in'
		| 'contains'
		| 'includes';
	field: string;
	value: unknown;
}

export interface CTELogicalFilter {
	type: 'logical';
	operator: 'and' | 'or';
	filters: CTEFilter[];
}

export interface CTEOrderBy {
	field: string;
	direction: 'asc' | 'desc';
}

export function createCTE(): CTE {
	return {
		version: '1.0'
	};
}

export function createEqualityFilter(field: string, value: unknown): CTEComparisonFilter {
	return {
		type: 'comparison',
		operator: 'equal',
		field,
		value
	};
}

export function createComparisonFilter(
	field: string,
	operator:
		| 'equal'
		| 'notEqual'
		| 'greaterThan'
		| 'lessThan'
		| 'greaterThanOrEqual'
		| 'lessThanOrEqual'
		| 'in'
		| 'contains'
		| 'includes',
	value: unknown
): CTEComparisonFilter {
	return {
		type: 'comparison',
		operator,
		field,
		value
	};
}

export function createAndFilter(...filters: CTEFilter[]): CTELogicalFilter {
	return {
		type: 'logical',
		operator: 'and',
		filters
	};
}

export function createOrFilter(...filters: CTEFilter[]): CTELogicalFilter {
	return {
		type: 'logical',
		operator: 'or',
		filters
	};
}

export function addNamedCTE(parent: CTE, name: string, child: CTE): void {
	if (!parent.ctes) {
		parent.ctes = {};
	}
	parent.ctes[name] = child;
}
