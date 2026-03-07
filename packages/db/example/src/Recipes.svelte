<script lang="ts">
	import { recipesCollection } from './collections/recipes.js';
	import { collection } from '@aicacia/db/svelte';

	let search = $state('');
	let query = $state(recipesCollection.query().orderBy('updatedAt', 'desc'));

	$effect(() => {
		query = search
			? recipesCollection.query().contains('title', search).orderBy('updatedAt', 'desc')
			: recipesCollection.query().orderBy('updatedAt', 'desc');
	});

	const recipes = $derived(collection(query));
</script>

<h1>Recipes</h1>

<div>
	<input bind:value={search} placeholder="Search recipes..." />
</div>

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
		<hr />
	{/each}
</ul>
