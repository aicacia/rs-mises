<script lang="ts">
	import { page } from '$app/state';
	import { m } from '$lib/paraglide/messages';
	import { ClientRegisterRequest } from '$lib/proto/mises.js';
	import AddClient from '../authorize/_ClientUpdates.svelte';
	import { oidcClient } from '$lib/common/util/grpcClient';
	import { isTauri } from '@tauri-apps/api/core';
	import { openUrl } from '@tauri-apps/plugin-opener';

	let urlRegistration = $derived(page.url.searchParams.get('registration') ?? undefined);
	let urlRedirect = $derived(page.url.searchParams.get('redirect_uri') ?? undefined);
	let urlState = $derived(page.url.searchParams.get('state') ?? undefined);

	let clientInfo = $state<ClientRegisterRequest | null>(null);
	let loading = $state(false);

	$effect(() => {
		if (urlRegistration) {
			try {
				clientInfo = JSON.parse(urlRegistration) as ClientRegisterRequest;
			} catch {
				clientInfo = null;
			}
		}
	});

	async function onAccept(ci: ClientRegisterRequest) {
		loading = true;
		try {
			const client = await oidcClient().clientRegister(ci);
			if (urlRedirect) {
				const u = new URL(urlRedirect!);
				u.searchParams.append('client_id', client.clientId || client.id);
				if (urlState) u.searchParams.append('state', urlState!);
				if (isTauri()) await openUrl(u.toString());
				else window.location.href = u.toString();
			}
		} catch (e) {
			console.error('Error registering client', e);
		} finally {
			loading = false;
		}
	}

	function onReject() {
		if (urlRedirect) {
			const u = new URL(urlRedirect!);
			u.searchParams.append('error', 'registration_denied');
			if (urlState) u.searchParams.append('state', urlState!);
			if (isTauri()) openUrl(u.toString());
			else window.location.href = u.toString();
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
