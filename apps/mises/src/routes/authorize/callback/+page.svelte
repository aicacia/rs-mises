<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';

	$effect.pre(() => {
		try {
			const hash = window.location.hash || '';
			const params = new URLSearchParams(hash.replace(/^#/, ''));
			const idTokenB64 = params.get('id_token');
			if (idTokenB64) {
				const b64 = idTokenB64.replace(/-/g, '+').replace(/_/g, '/');
				const json = decodeURIComponent(escape(window.atob(b64)));
				const payload = JSON.parse(json);
				localStorage.setItem('mises_id_token', idTokenB64);
				localStorage.setItem('mises_user_sub', payload.sub);
			}
		} catch (e) {
			console.error('authorize callback error', e);
		} finally {
			goto(resolve('/'));
		}
	});
</script>

<div class="p-4">
	<p>Signing in…</p>
</div>
