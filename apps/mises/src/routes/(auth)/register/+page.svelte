<script lang="ts">
	import { handleNativeCallbackRequest, type NativeRequest } from '@aicacia/oidc-client';
	import { page } from '$app/state';
	import { ClientRegisterRequest } from '$lib/proto/mises.js';
	import AddClient from '$lib/common/components/client/ClientUpdates.svelte';
	import { camelizeKeys } from '$lib/common/util/camelizeKeys';
	import { oidcClient } from '$lib/common/util/grpcClient';
	import { redirectToUrl } from '$lib/common/util/redirectToUrl';
	import { snakeCaseKeys } from '$lib/common/util/snakeCaseKeys';

	let urlRegistration = $derived(page.url.searchParams.get('registration') ?? undefined);
	let urlRedirect = $derived(page.url.searchParams.get('redirect_uri') ?? undefined);
	let urlState = $derived(page.url.searchParams.get('state') ?? undefined);
	let nativeRequest = $derived(
		page.url.searchParams.get('native')
			? (JSON.parse(page.url.searchParams.get('native')) as NativeRequest)
			: undefined
	);

	let clientInfo = $state<ClientRegisterRequest | null>(null);
	let loading = $state(false);

	$effect(() => {
		if (urlRegistration) {
			try {
				clientInfo = camelizeKeys(JSON.parse(urlRegistration)) as ClientRegisterRequest;
			} catch {
				clientInfo = null;
			}
		}
	});

	async function onAccept(ci: ClientRegisterRequest) {
		loading = true;
		try {
			const client = await oidcClient().clientRegister(ci);

			if (nativeRequest) {
				await redirectToUrl(
					await handleNativeCallbackRequest(
						nativeRequest,
						() => new Response(JSON.stringify(snakeCaseKeys(client)))
					)
				);
			} else {
				const u = new URL(urlRedirect!);
				u.searchParams.set('client', JSON.stringify(client));
				if (urlState) {
					u.searchParams.set('state', urlState!);
				}
				await redirectToUrl(u);
			}
		} catch (e) {
			console.error('Error registering client', e);
		} finally {
			loading = false;
		}
	}

	async function onReject() {
		if (nativeRequest) {
			await redirectToUrl(
				await handleNativeCallbackRequest(
					nativeRequest,
					() =>
						new Response(
							JSON.stringify({
								error: 'registration_denied'
							})
						)
				)
			);
		} else {
			const u = new URL(urlRedirect!);
			u.searchParams.set('error', 'registration_denied');
			if (urlState) {
				u.searchParams.set('state', urlState!);
			}
			await redirectToUrl(u);
		}
	}
</script>

<div class="overflow-auto">
	<div class="m-8 flex grow flex-col items-center justify-center">
		<div class="card w-md">
			{#if clientInfo}
				<AddClient client={clientInfo} isNew={true} disabled={loading} {onAccept} {onReject} />
			{:else}
				<section>
					<h5>Invalid registration request</h5>
					<p>Missing or malformed registration information.</p>
					<div class="mt-4 flex flex-row justify-center gap-4">
						<button class="btn secondary" onclick={onReject}>Close</button>
					</div>
				</section>
			{/if}
		</div>
	</div>
</div>
