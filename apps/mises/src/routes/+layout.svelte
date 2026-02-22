<script lang="ts">
	import './layout.css';

	import favicon from '$lib/assets/favicon.svg';
	import { onMount } from 'svelte';
	import type { LayoutProps } from './$types';
	import { resolve } from '$app/paths';
	import { getTheme } from '@aicacia/svelte-headless';
	import Notifications from '$lib/common/components/Notifications.svelte';
	import { onOpenUrl } from '@tauri-apps/plugin-deep-link';
	import type { UnlistenFn } from '@tauri-apps/api/event';
	import { goto } from '$app/navigation';

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

		let onOpenUrlUnlistenFn: UnlistenFn | undefined;
		onOpenUrl(async (urlStrings) => {
			if (urlStrings.length > 0) {
				const urlString = urlStrings[0];
				const url = new URL(urlString);

				switch (url.hostname) {
					case 'authorize': {
						await goto(resolve('/(auth)/authorize') + url.search);
						break;
					}
					default: {
						console.warn(`Unknown deep link: ${urlString}`);
						break;
					}
				}
			}
		}).then((unlisten) => {
			onOpenUrlUnlistenFn = unlisten;
		});

		return () => {
			if (onOpenUrlUnlistenFn) {
				onOpenUrlUnlistenFn();
			}
		};
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<link rel="manifest" crossorigin="use-credentials" href={resolve('/manifest.json')} />
</svelte:head>

{@render children()}
<Notifications />
