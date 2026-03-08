<script>
	import { onMount } from 'svelte';

	onMount(() => {
		setTimeout(() => {
			if (window.history.length > 1) {
				window.history.back();
			} else {
				window.location.href = '/';
			}
		}, 1000);
	});
</script>

<div class="flex min-h-screen items-center justify-center">
	<p>Processing callback...</p>
</div>
