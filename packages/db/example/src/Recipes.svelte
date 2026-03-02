<script lang="ts">
	import { recipesCollection } from './collections/recipes.js';
	import { collection } from '@aicacia/db/collection.svelte';

	const recipes = collection(recipesCollection.query().orderBy('updatedAt', 'desc'));
</script>

<h1>Recipes</h1>

<ul>
	{#each recipes.data as recipe}
		<li>{recipe.title}</li>
		<p>{recipe.description}</p>
		<hr />
		<ul>
			{#each recipe.ingredients as ingredient}
				<li>{ingredient.quantity.value} {ingredient.quantity.unit} {ingredient.item.name}</li>
			{/each}
		</ul>
		<ol>
			{#each recipe.instructions as instruction}
				<li>{instruction}</li>
			{/each}
		</ol>
	{/each}
</ul>
