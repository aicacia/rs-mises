<script lang="ts">
	import { userSettingsSingleton } from './collections/userSettings.js';
	import { singleton } from '@aicacia/db/singleton.svelte';
	import Recipes from './Recipes.svelte';

	const userSettings = singleton(userSettingsSingleton);

	$effect(() => {
		document.body.dataset.theme = userSettings.data?.theme ?? 'light';
	});
</script>

<svelte:head>
	<title>Mises DB Svelte Example</title>
</svelte:head>

<Recipes />
