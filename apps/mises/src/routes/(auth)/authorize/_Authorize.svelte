<script lang="ts" module>
	import type { AuthorizeRequest, UserInfo } from '$lib/common/openapi/oidc';

	export interface AuthorizeProps {
		userInfo: UserInfo;
		clientInfo: ClientInfo | null;
		authorizeRequest: AuthorizeRequest;
	}
</script>

<script lang="ts">
	import { oidcApi } from '$lib/common/openapi';
	import type { Client } from '$lib/common/openapi/oidc/models/Client';
	import {
		getClientDiff,
		rejectAuthorizeRequest,
		resolveAuthorizeRequest,
		type ClientInfo
	} from './_utils';
	import { m } from '$lib/paraglide/messages';
	import { LoaderCircle } from '@lucide/svelte';
	import AddClient from './_ClientUpdates.svelte';
	import AuthorizeClient from './_AuthorizeClient.svelte';
	import { handleError } from '$lib/common/errors';

	let { userInfo, clientInfo, authorizeRequest }: AuthorizeProps = $props();

	let clientDiff = $state<Partial<ClientInfo> | false>(false);
	let client = $state<Client | null>(null);

	let loadingClient = $state(true);
	$effect(() => {
		loadingClient = true;
		oidcApi
			.client({ clientId: authorizeRequest.clientId })
			.catch((_e) => {
				return null;
			})
			.then((c) => {
				client = c;
			})
			.finally(() => {
				loadingClient = false;
			});
	});

	$effect(() => {
		if (clientInfo) {
			if (client) {
				clientDiff = getClientDiff(client, clientInfo);
			}
		} else {
			clientDiff = false;
		}
	});

	let loadingUserAllowed = $state(true);
	$effect(() => {
		if (loadingClient) {
			return;
		}
		if (!client || clientDiff) {
			return;
		}
		loadingUserAllowed = true;
		oidcApi
			.isClientAllowedForUser({
				clientId: authorizeRequest.clientId,
				scope: authorizeRequest.scope
			})
			.then(onAuthorize)
			.catch(() => {
				// Not yet approved or scopes changed; fall back to consent screen
			})
			.finally(() => {
				loadingUserAllowed = false;
			});
	});

	let loadingAuthorizeRequest = $state(false);
	async function onAuthorize() {
		loadingAuthorizeRequest = true;
		try {
			await resolveAuthorizeRequest(authorizeRequest);
		} catch (e) {
			handleError(e);
		} finally {
			loadingAuthorizeRequest = false;
		}
	}
	async function onAllow() {
		try {
			await oidcApi.approveClientForUser({ clientId: authorizeRequest.clientId });
			await onAuthorize();
		} catch (e) {
			handleError(e);
		}
	}
	async function onDeny() {
		rejectAuthorizeRequest(authorizeRequest, 'access_denied', m.authorize_access_denied_reason());
	}
	import { oidcClient } from '$lib/common/util/grpcClient';
	import { toGrpcClientRegisterRequest, grpcClientToClient } from './_utils';

	async function onAcceptClientUpdates(clientRegisterRequest: ClientInfo) {
		try {
			loadingClient = true;
			const grpcReq = toGrpcClientRegisterRequest(clientRegisterRequest);
			const grpcResp = await oidcClient().clientRegister(grpcReq);
			client = grpcClientToClient(grpcResp);
		} catch (e) {
			handleError(e);
		} finally {
			loadingClient = false;
		}
	}
	async function onRejectClientUpdates() {
		rejectAuthorizeRequest(
			authorizeRequest,
			'unauthorized_client',
			m.authorize_unauthorized_client_reason()
		);
	}

	let loading = $derived(loadingClient || loadingUserAllowed || loadingAuthorizeRequest);
	let disabled = $derived(loading);
</script>

{#if loading}
	<div class="flex flex-row items-center justify-center">
		<LoaderCircle class="animate-spin" />
	</div>
{:else if client}
	{#if clientDiff}
		<AddClient
			{userInfo}
			client={{
				name: client.name,
				logoUri: client.logoUri,
				...clientDiff
			}}
			isNew={false}
			{disabled}
			onAccept={onAcceptClientUpdates}
			onReject={onRejectClientUpdates}
		/>
	{:else}
		<AuthorizeClient {userInfo} {client} {disabled} {onAllow} {onDeny} />
	{/if}
{:else if clientInfo}
	<AddClient
		{userInfo}
		client={clientInfo}
		{disabled}
		isNew={true}
		onAccept={onAcceptClientUpdates}
		onReject={onRejectClientUpdates}
	/>
{/if}
