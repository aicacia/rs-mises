<script>
	import { onMount } from 'svelte';

	onMount(() => {
		setTimeout(() => {
			if (window.history.length > 1) {
				window.history.back();
			} else {
				window.close();
			}
		}, 1000);
	});
</script>

<div class="flex min-h-screen items-center justify-center">
	<p>Processing native callback...</p>
</div>
