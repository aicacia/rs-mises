<script lang="ts">
	import './layout.css';
	import github from 'svelte-highlight/styles/github';

	const { children } = $props();
</script>

<svelte:head>
	<!-- eslint-disable-next-line svelte/no-at-html-tags -->
	{@html github}
</svelte:head>

<div class="overflow-x-hidden overflow-y-auto">
	<div class="container mx-auto p-4">
		{@render children()}
	</div>
</div>
