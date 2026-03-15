<script lang="ts" module>
	import { resolve } from '$app/paths';
	import type { TokenRequest } from '$lib/proto/mises';

	const AUTHORIZE_PATH = resolve('/(auth)/authorize');
	const REGISTER_PATH = resolve('/(auth)/register');

	function createTokenRequest(url: URL, code: string): TokenRequest {
		return {
			authorizationCode: {
				code,
				clientId: url.searchParams.get('client_id') ?? undefined,
				redirectUri: url.searchParams.get('redirect_uri') ?? undefined,
				codeVerifier: url.searchParams.get('code_verifier') ?? undefined
			}
		};
	}
</script>

<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import type { LayoutProps } from './$types';
	import { getTheme } from '@aicacia/svelte-headless';
	import Notifications from '$lib/common/components/Notifications.svelte';
	import { onOpenUrl } from '@tauri-apps/plugin-deep-link';
	import type { UnlistenFn } from '@tauri-apps/api/event';
	import { goto } from '$app/navigation';
	import { oidcClient } from '$lib/common/util/grpcClient';
	import { handleNativeCallbackRequestUrl } from '@aicacia/oidc-client';
	import { redirectToUrl } from '$lib/common/util/redirectToUrl';
	import { snakeCaseKeys } from '$lib/common/util/snakeCaseKeys';
	import { onMount } from 'svelte';

	const JSON_CONTENT_TYPE = 'application/json;charset=UTF-8';

	async function redirectWithError(
		url: URL,
		error: string,
		errorDescription: string
	): Promise<void> {
		await redirectToUrl(
			await handleNativeCallbackRequestUrl(
				url,
				() =>
					new Response(
						JSON.stringify({
							error,
							error_description: errorDescription
						})
					)
			)
		);
	}

	async function handleNativeTokenRequest(url: URL): Promise<void> {
		const code = url.searchParams.get('code');
		if (!code) {
			await redirectWithError(url, 'invalid_request', 'Missing `code` parameter');
			return;
		}

		try {
			const tokenResponse = await oidcClient().token(createTokenRequest(url, code));
			await redirectToUrl(
				await handleNativeCallbackRequestUrl(
					url,
					() => new Response(JSON.stringify(snakeCaseKeys(tokenResponse)))
				)
			);
		} catch (error) {
			console.error('Error handling token deep link', error);
			await redirectWithError(
				url,
				'server_error',
				error instanceof Error ? error.message : 'Failed to handle token request'
			);
		}
	}

	async function handleNativeOpenIdConfigurationRequest(url: URL): Promise<void> {
		try {
			const openIdConfiguration = await oidcClient().getOpenIdConfiguration({});
			await redirectToUrl(
				await handleNativeCallbackRequestUrl(
					url,
					() => new Response(JSON.stringify(snakeCaseKeys(openIdConfiguration)))
				)
			);
		} catch (error) {
			console.error('Error handling openid-configuration deep link', error);
			await redirectWithError(
				url,
				'server_error',
				error instanceof Error ? error.message : 'Failed to handle openid-configuration request'
			);
		}
	}

	async function handleNativeUserInfoRequest(url: URL): Promise<void> {
		try {
			await redirectToUrl(
				await handleNativeCallbackRequestUrl(url, async () => {
					const userInfo = await oidcClient().getUserInfo({});

					return new Response(JSON.stringify(snakeCaseKeys(userInfo)), {
						headers: {
							'content-type': JSON_CONTENT_TYPE
						}
					});
				})
			);
		} catch (error) {
			console.error('Error handling user-info deep link', error);
			await redirectWithError(
				url,
				'server_error',
				error instanceof Error ? error.message : 'Failed to handle user-info request'
			);
		}
	}

	async function handleDeepLink(urlStrings: string[]): Promise<void> {
		const [urlString] = urlStrings;
		if (!urlString) {
			return;
		}

		const url = new URL(urlString);

		console.debug('Deep link received', url);

		switch (url.pathname) {
			case '/authorize': {
				// eslint-disable-next-line svelte/no-navigation-without-resolve
				await goto(`${AUTHORIZE_PATH}${url.search}`);
				break;
			}
			case '/register': {
				// eslint-disable-next-line svelte/no-navigation-without-resolve
				await goto(`${REGISTER_PATH}${url.search}`);
				break;
			}
			case '/.well-known/openid-configuration': {
				await handleNativeOpenIdConfigurationRequest(url);
				break;
			}
			case '/token': {
				await handleNativeTokenRequest(url);
				break;
			}
			case '/user-info': {
				await handleNativeUserInfoRequest(url);
				break;
			}
			default: {
				console.warn(`Unknown deep link: ${urlString}`);
				break;
			}
		}
	}

	let { children }: LayoutProps = $props();

	$effect(() => {
		if (getTheme() === 'dark') {
			document.body.classList.add('dark');
			return;
		}

		document.body.classList.remove('dark');
	});

	onMount(() => {
		document.body.classList.add('hydrated');

		let onOpenUrlUnlistenFn: UnlistenFn | undefined;

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
