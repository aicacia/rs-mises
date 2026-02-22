<script lang="ts">
	import { getUserManager } from '$lib/common/state/user.svelte';

	let { data } = $props();

	async function onSignOut() {
		const userManager = await getUserManager();
		userManager.signoutRedirect();
	}
</script>

<div class="flex grow flex-col items-center justify-center">
	<h1>Welcome, {data.user.profile.preferred_username}!</h1>
</div>
