<script lang="ts">
	import { needsRegistration, signin, startRegistration } from '$lib/common/state/user.svelte';

	async function onSubmit(e: SubmitEvent) {
		e.preventDefault();
		if (needsRegistration()) {
			await startRegistration();
			return;
		}
		await signin(false);
	}
</script>

<form onsubmit={onSubmit} class="flex flex-col">
	<input class="btn primary" type="submit" value="Sign in" />
</form>
