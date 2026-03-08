<script lang="ts" module>
	import type { ClientRegisterRequest } from '$lib/proto/mises';

	export interface ClientInfoProps {
		client: Partial<ClientRegisterRequest> & Pick<ClientRegisterRequest, 'logoUri' | 'name'>;
		disabled?: boolean;
		isNew: boolean;
		onAccept: (updates: ClientRegisterRequest) => Promise<void>;
		onReject: () => Promise<void>;
	}
</script>

<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import ClientHeader from './ClientHeader.svelte';
	import ClientFields from './ClientFields.svelte';

	let { client, disabled, isNew, onAccept, onReject }: ClientInfoProps = $props();

	let loading = $state(false);

	async function onAcceptInternal() {
		try {
			loading = true;
			await onAccept(client as ClientRegisterRequest);
		} finally {
			loading = false;
		}
	}
	async function onRejectInternal() {
		try {
			loading = true;
			await onReject();
		} finally {
			loading = false;
		}
	}
</script>

<ClientHeader {client} />

<hr />

{#if isNew}
	<p>{m.authorize_new_client_request()}</p>
{:else}
	<p>{m.authorize_updated_client_request()}</p>
{/if}

<ClientFields {client} />

<hr />

<section>
	<!-- true here until we return the user permissions -->
	{#if true}
		<div class="mt-4 flex flex-row justify-center gap-4">
			<button class="btn secondary" disabled={disabled || loading} onclick={onRejectInternal}
				>{m.client_reject()}</button
			>
			<button class="btn danger" disabled={disabled || loading} onclick={onAcceptInternal}
				>{m.client_accept()}</button
			>
		</div>
	{:else}
		<p>{m.client_not_allowed_to_approve()}</p>
		<div class="mt-4 flex flex-row justify-center gap-4">
			<button class="btn secondary" disabled={disabled || loading} onclick={onRejectInternal}
				>{m.client_close()}</button
			>
		</div>
	{/if}
</section>
