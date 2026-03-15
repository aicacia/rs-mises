<script lang="ts">
	let { data } = $props();
</script>

<div class="flex grow flex-col items-center justify-center">
	<h1>Welcome, {data.user.preferred_username}!</h1>
</div>
