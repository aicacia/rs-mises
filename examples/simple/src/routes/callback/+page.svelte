<script context="module">
	export const prerender = false;
	export const ssr = false;
</script>

<script lang="ts">
	import { onMount } from 'svelte';
	import { getOidcClient } from '$lib/common/state/user.svelte';
	import { page } from '$app/state';

	let callbackError = '';
	let callbackErrorDescription = '';

	onMount(async () => {
		const searchParams = new URLSearchParams(page.url.searchParams);
		const hashParams = new URLSearchParams(page.url.hash.replace(/^#/, ''));

		const error = searchParams.get('error') ?? hashParams.get('error');
		const errorDescription =
			searchParams.get('error_description') ?? hashParams.get('error_description') ?? '';

		if (error) {
			callbackError = error;
			callbackErrorDescription = errorDescription;
			return;
		}

		try {
			await getOidcClient().handleSigninCallback();
		} catch (err) {
			callbackError = 'callback_error';
			callbackErrorDescription = err instanceof Error ? err.message : String(err);
		}
	});
</script>

<div class="flex min-h-screen items-center justify-center">
	{#if callbackError}
		<div class="max-w-xl space-y-2 px-6 text-center">
			<p class="font-semibold">Callback failed</p>
			<p class="wrap-break-word">{callbackError}</p>
			{#if callbackErrorDescription}
				<p class="text-sm wrap-break-word opacity-80">{callbackErrorDescription}</p>
			{/if}
		</div>
	{:else}
		<p>Processing callback...</p>
	{/if}
</div>
