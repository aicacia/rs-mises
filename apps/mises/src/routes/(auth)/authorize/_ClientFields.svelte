<script lang="ts" module>
	import type { ClientInfo } from './_utils';

	export interface ClientFieldsProps {
		client: Partial<ClientInfo>;
	}
</script>

<script lang="ts">
	import { m } from '$lib/paraglide/messages';

	let { client }: ClientFieldsProps = $props();
</script>

{#if client.scopes?.length}
	<section>
		<h5>{m.authorize_scopes_label()}</h5>
		<ul class="list-inside list-disc space-y-1 text-sm">
			{#each client.scopes as s}
				<li>{s}</li>
			{/each}
		</ul>
	</section>
{/if}

{#if client.redirectUris?.length}
	<section>
		<h5>{m.authorize_redirect_uris_label()}</h5>
		<ul class="list-inside list-disc space-y-1 text-sm">
			{#each client.redirectUris as uri}
				<li>{uri}</li>
			{/each}
		</ul>
	</section>
{/if}

{#if client.postLogoutRedirectUris?.length}
	<section>
		<h5>{m.authorize_post_logout_redirect_uris_label()}</h5>
		<ul class="list-inside list-disc space-y-1 text-sm">
			{#each client.postLogoutRedirectUris as uri}
				<li>{uri}</li>
			{/each}
		</ul>
	</section>
{/if}

{#if client.grantTypes?.length}
	<section>
		<h5>{m.authorize_grant_types_label()}</h5>
		<ul class="list-inside list-disc space-y-1 text-sm">
			{#each client.grantTypes as t}
				<li>{t}</li>
			{/each}
		</ul>
	</section>
{/if}

{#if client.responseTypes?.length}
	<section>
		<h5>{m.authorize_response_types_label()}</h5>
		<ul class="list-inside list-disc space-y-1 text-sm">
			{#each client.responseTypes as r}
				<li>{r}</li>
			{/each}
		</ul>
	</section>
{/if}

{#if client.audience?.length}
	<section>
		<h5>{m.authorize_audience_label()}</h5>
		<ul class="list-inside list-disc space-y-1 text-sm">
			{#each client.audience as a}
				<li>{a}</li>
			{/each}
		</ul>
	</section>
{/if}

{#if client.accessTokenExpiresInSeconds || client.idTokenExpiresInSeconds || client.refreshExpiresInSeconds}
	<section>
		<h5>{m.authorize_token_expiry_label()}</h5>
		<ul class="list-inside list-disc space-y-1 text-sm">
			{#if client.accessTokenExpiresInSeconds}
				<li>{m.authorize_access_token_expires({ seconds: client.accessTokenExpiresInSeconds })}</li>
			{/if}
			{#if client.idTokenExpiresInSeconds}
				<li>{m.authorize_id_token_expires({ seconds: client.idTokenExpiresInSeconds })}</li>
			{/if}
			{#if client.refreshExpiresInSeconds}
				<li>{m.authorize_refresh_token_expires({ seconds: client.refreshExpiresInSeconds })}</li>
			{/if}
		</ul>
	</section>
{/if}

{#if client.policyUri || client.termsOfServiceUri}
	<section>
		<h5>{m.authorize_legal_label()}</h5>
		<ul class="space-y-1 text-sm">
			{#if client.policyUri}
				<li>
					<a href={client.policyUri} target="_blank">{m.authorize_privacy_policy()}</a>
				</li>
			{/if}
			{#if client.termsOfServiceUri}
				<li>
					<a href={client.termsOfServiceUri} target="_blank">{m.authorize_terms_of_service()}</a>
				</li>
			{/if}
		</ul>
	</section>
{/if}
