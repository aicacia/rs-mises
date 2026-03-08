<script lang="ts">
	import './layout.css';

	import favicon from '$lib/assets/favicon.svg';
	import type { LayoutProps } from './$types';
	import { resolve } from '$app/paths';
	import { getTheme } from '@aicacia/svelte-headless';
	import Notifications from '$lib/common/components/Notifications.svelte';
	import { onOpenUrl } from '@tauri-apps/plugin-deep-link';
	import type { UnlistenFn } from '@tauri-apps/api/event';
	import { goto } from '$app/navigation';
	import { oidcClient } from '$lib/common/util/grpcClient';
	import { nativeCallbackUrlFromRequestUrl } from '$lib/common/util/nativeCallbackUrlFromRequestUrl';
	import { redirectToUrl } from '$lib/common/util/redirectToUrl';
	import { snakeCaseKeys } from '$lib/common/util/snakeCaseKeys';

	let { children }: LayoutProps = $props();

	$effect(() => {
		if (getTheme() === 'dark') {
			document.body.classList.add('dark');
		} else {
			document.body.classList.remove('dark');
		}
	});

	$effect.pre(() => {
		document.body.classList.add('hydrated');

		let onOpenUrlUnlistenFn: UnlistenFn | undefined;

		const handleDeepLink = async (urlStrings: string[]) => {
			const [urlString] = urlStrings;
			if (!urlString) {
				return;
			}

			const url = new URL(urlString);

			console.debug('Deep link received', url);

			switch (url.pathname) {
				case '/authorize': {
					const authorizePath = resolve('/(auth)/authorize');
					// eslint-disable-next-line svelte/no-navigation-without-resolve
					await goto(`${authorizePath}${url.search}`);
					break;
				}
				case '/register': {
					const registerPath = resolve('/(auth)/register');
					// eslint-disable-next-line svelte/no-navigation-without-resolve
					await goto(`${registerPath}${url.search}`);
					break;
				}
				case '/.well-known/openid-configuration': {
					const openIdConfiguration = await oidcClient().getOpenIdConfiguration({});
					const callbackUrl = nativeCallbackUrlFromRequestUrl(url, snakeCaseKeys(openIdConfiguration));
					await redirectToUrl(callbackUrl);
					break;
				}
				default: {
					console.warn(`Unknown deep link: ${urlString}`);
					break;
				}
			}
		};

		onOpenUrl(handleDeepLink).then((unlisten) => {
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
