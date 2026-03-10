<script lang="ts">
	import {  signin } from '$lib/common/state/user.svelte';

	async function onSubmit(e: SubmitEvent) {
		e.preventDefault();
		await signin();
	}
</script>

<form onsubmit={onSubmit} class="flex flex-col">
	<input class="btn primary" type="submit" value="Sign in" />
</form>
