import * as v from 'valibot';
import { createCollection, MemoryAdapter } from '@aicacia/db';

export const metricUnitSchema = v.picklist(['g', 'kg', 'ml', 'l']);

export const quantitySchema = v.object({
	value: v.number(),
	unit: metricUnitSchema
});

export const itemSchema = v.object({
	name: v.string(),
	description: v.string()
});

export const ingredientSchema = v.object({
	item: itemSchema,
	quantity: quantitySchema
});

export const recipeSchema = v.object({
	id: v.string(),
	title: v.string(),
	description: v.string(),
	ingredients: v.array(ingredientSchema),
	instructions: v.array(v.string()),
	updatedAt: v.date(),
	createdAt: v.date()
});

export type Recipe = v.InferOutput<typeof recipeSchema>;

export const recipesCollection = createCollection({
	id: 'recipes',
	source: new MemoryAdapter<Recipe>('id'),
	keyOf: (doc) => doc.id
});
