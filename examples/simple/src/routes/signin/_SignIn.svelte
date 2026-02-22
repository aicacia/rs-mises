<script lang="ts">
	import { getUserManager } from '$lib/common/state/user.svelte';

	async function onSubmit(e: SubmitEvent) {
		e.preventDefault();

		const userManager = await getUserManager();
		await userManager.signinRedirect();
	}
</script>

<form onsubmit={onSubmit} class="flex flex-col">
	<input class="btn primary mt-4" type="submit" value="Sign in" />
</form>
