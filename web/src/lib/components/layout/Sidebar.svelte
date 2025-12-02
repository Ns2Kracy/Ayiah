<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import logo from '$lib/assets/logo.svg';
	import LightSwitch from '$lib/components/LightSwitch.svelte';
	import { Film, Tv, Library } from '@lucide/svelte';

	const navItems = [
		{ label: 'Library', href: '/', icon: Library },
		{ label: 'Movies', href: '/library/movies', icon: Film },
		{ label: 'TV Shows', href: '/library/tv', icon: Tv }
	];
</script>

<div class="flex h-full flex-col gap-2">
	<div class="flex h-14 items-center border-b px-6">
		<a href={resolve('/')} class="flex items-center gap-2 font-semibold">
			<img src={logo} alt="Ayiah Logo" class="h-8 w-8" />
			<span class="text-xl font-bold text-primary-500">Ayiah</span>
		</a>
	</div>
	<div class="flex-1 px-4 py-2">
		<nav class="grid items-start px-2 text-sm font-medium lg:px-4">
			{#each navItems as item (item.href)}
				<a
					href={resolve(item.href as '/')}
					class="flex items-center gap-3 rounded-lg px-3 py-2 transition-all hover:text-primary-500
					{page.url.pathname === resolve(item.href as '/')
						? 'bg-surface-200 text-primary-500 dark:bg-surface-800'
						: 'text-surface-500'}"
				>
					<item.icon class="h-4 w-4" />
					{item.label}
				</a>
			{/each}
		</nav>
	</div>
	<div class="border-t p-4">
		<div class="flex items-center justify-between px-4">
			<span class="text-sm font-medium text-surface-500">Dark Mode</span>
			<LightSwitch />
		</div>
	</div>
</div>
