<script lang="ts">
	import './layout.css';

	import favicon from '$lib/assets/favicon.svg';
	import { onMount } from 'svelte';
	import type { LayoutProps } from './$types';
	import { resolve } from '$app/paths';
	import { getTheme } from '@aicacia/svelte-headless';
	import Notifications from '$lib/common/components/Notifications.svelte';

	let { children }: LayoutProps = $props();

	$effect(() => {
		if (getTheme() === 'dark') {
			document.body.classList.add('dark');
		} else {
			document.body.classList.remove('dark');
		}
	});

	onMount(() => {
		document.body.classList.add('hydrated');
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<link rel="manifest" crossorigin="use-credentials" href={resolve('/manifest.json')} />
</svelte:head>

{@render children()}
<Notifications />
