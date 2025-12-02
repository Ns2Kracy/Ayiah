<script lang="ts">
	import type { MediaItemWithMetadata } from '$lib/api/types';
	import { Film, Tv } from '@lucide/svelte';
	import { resolve } from '$app/paths';

	let { item } = $props<{ item: MediaItemWithMetadata }>();

	let posterUrl = $derived(
		item.metadata?.poster_path
			? `https://image.tmdb.org/t/p/w342${item.metadata.poster_path}`
			: null
	);

	let year = $derived(
		item.metadata?.release_date ? new Date(item.metadata.release_date).getFullYear() : null
	);
</script>

<a
	href={resolve(`/library/items/${item.id}`)}
	class="group relative flex flex-col gap-2 rounded-lg bg-surface-100 p-2 transition-all hover:bg-surface-200 dark:bg-surface-900 dark:hover:bg-surface-800"
>
	<div
		class="relative aspect-2/3 w-full overflow-hidden rounded-md bg-surface-200 dark:bg-surface-800"
	>
		{#if posterUrl}
			<img
				src={posterUrl}
				alt={item.title}
				class="h-full w-full object-cover transition-transform duration-300 group-hover:scale-105"
				loading="lazy"
			/>
		{:else}
			<div class="flex h-full w-full flex-col items-center justify-center gap-2 text-surface-400">
				{#if item.media_type === 'movie'}
					<Film class="h-12 w-12" />
				{:else}
					<Tv class="h-12 w-12" />
				{/if}
				<span class="text-xs">No Poster</span>
			</div>
		{/if}

		<!-- Overlay for rating -->
		{#if item.metadata?.vote_average}
			<div
				class="absolute top-2 right-2 rounded bg-black/60 px-1.5 py-0.5 text-xs font-bold text-white backdrop-blur-sm"
			>
				{item.metadata.vote_average.toFixed(1)}
			</div>
		{/if}
	</div>

	<div class="flex flex-col gap-0.5 px-1">
		<h3
			class="truncate text-sm font-medium text-surface-900 dark:text-surface-50"
			title={item.title}
		>
			{item.title}
		</h3>
		<div class="flex items-center justify-between text-xs text-surface-500">
			<span>{year || 'Unknown'}</span>
			<span class="capitalize">{item.media_type}</span>
		</div>
	</div>
</a>
