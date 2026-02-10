<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';

	onMount(() => {
		try {
			const hash = window.location.hash || '';
			const params = new URLSearchParams(hash.replace(/^#/, ''));
			const idTokenB64 = params.get('id_token');
			if (idTokenB64) {
				// decode base64url
				const b64 = idTokenB64.replace(/-/g, '+').replace(/_/g, '/');
				const json = decodeURIComponent(escape(window.atob(b64)));
				const payload = JSON.parse(json);
				// store minimal session info (dev only)
				localStorage.setItem('mises_id_token', idTokenB64);
				localStorage.setItem('mises_user_sub', payload.sub);
			}
		} catch (e) {
			console.error('authorize callback error', e);
		} finally {
			goto('/');
		}
	});
</script>

<div class="p-4">
	<p>Signing in…</p>
</div>
