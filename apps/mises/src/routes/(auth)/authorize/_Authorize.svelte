<script lang="ts" module>
	export interface AuthorizeProps {
		userInfo: UserInfo;
		clientInfo: ClientInfo | null;
		authorizeRequest: AuthorizeRequest;
	}
</script>

<script lang="ts">
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
	import type { AuthorizeRequest, Client, UserInfo } from '$lib/proto/mises';
	import { clientClient, oidcClient } from '$lib/common/util/grpcClient';

	let { userInfo, clientInfo, authorizeRequest }: AuthorizeProps = $props();

	let clientDiff = $state<Partial<ClientInfo> | false>(false);
	let client = $state<Client | null>(null);

	let loadingClient = $state(true);
	$effect(() => {
		loadingClient = true;
		if (authorizeRequest.clientId === 'unknown') {
			console.debug('Client ID is unknown, skipping client info fetch');
			client = null;
			loadingClient = false;
			return;
		}
		console.debug('Fetching client info for clientId', authorizeRequest.clientId);
		clientClient()
			.get({ clientId: authorizeRequest.clientId })
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
		}
		clientDiff = false;
	});

	let loadingUserAllowed = $state(true);
	$effect(() => {
		if (loadingClient) {
			console.debug('Still loading client info, skipping user allowed check');
			return;
		}
		if (!client || clientDiff) {
			console.debug('Client info not available or has updates, skipping user allowed check');
			loadingUserAllowed = false;
			return;
		}
		loadingUserAllowed = true;
		console.debug('Checking if user has already allowed this client and scopes');
		clientClient()
			.isAllowedForUser({
				clientId: authorizeRequest.clientId,
				scope: authorizeRequest.scope?.split(' ') ?? []
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
			console.error('Error resolving authorize request', e);
		} finally {
			loadingAuthorizeRequest = false;
		}
	}
	async function onAllow() {
		try {
			await clientClient().approveForUser({ clientId: authorizeRequest.clientId });
			await onAuthorize();
		} catch (e) {
			console.error('Error approving client for user', e);
		}
	}
	async function onDeny() {
		rejectAuthorizeRequest(authorizeRequest, 'access_denied', m.authorize_access_denied_reason());
	}

	async function onAcceptClientUpdates(clientRegisterRequest: ClientInfo) {
		try {
			loadingClient = true;
			client = await oidcClient().clientRegister(clientRegisterRequest);
		} catch (e) {
			console.error('Error registering client', e);
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
		client={clientInfo}
		{disabled}
		isNew={true}
		onAccept={onAcceptClientUpdates}
		onReject={onRejectClientUpdates}
	/>
{/if}
