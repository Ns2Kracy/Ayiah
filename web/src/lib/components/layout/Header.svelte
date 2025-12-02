<script lang="ts">
	import { resolve } from '$app/paths';
	import { Settings, User } from '@lucide/svelte';
</script>

<header
	class="sticky top-0 z-10 flex h-14 items-center gap-4 border-b bg-surface-100/50 px-6 backdrop-blur-md dark:bg-surface-900/50"
>
	<span class="font-bold lg:hidden">Ayiah</span>

	<div class="ml-auto flex items-center gap-2">
		<a href={resolve('/settings')} class="variant-ghost-surface btn-icon btn-icon-sm">
			<Settings class="h-5 w-5" />
		</a>
		<button class="variant-ghost-surface btn-icon btn-icon-sm">
			<User class="h-5 w-5" />
		</button>
	</div>
</header>
