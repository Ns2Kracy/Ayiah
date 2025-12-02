<script lang="ts">
	import { useRequest } from 'alova/client';
	import { libraryApi } from '$lib/api/methods/library';
	import MediaCard from '$lib/components/MediaCard.svelte';
	import { Search, ArrowUpDown } from '@lucide/svelte';

	let searchQuery = $state('');
	let sortBy = $state('added');
	let sortOrder = $state('desc');

	const {
		loading,
		data,
		send: fetchItems
	} = useRequest(
		() =>
			libraryApi.getMovies({
				search: searchQuery || undefined,
				sort: sortBy,
				order: sortOrder
			}),
		{
			immediate: true
		}
	);

	let items = $derived($data?.data?.items || []);

	function handleSearch() {
		fetchItems();
	}

	function toggleSortOrder() {
		sortOrder = sortOrder === 'asc' ? 'desc' : 'asc';
		fetchItems();
	}
</script>

<div class="flex flex-col gap-6">
	<div class="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
		<h1 class="text-2xl font-bold text-surface-900 dark:text-surface-50">Movies</h1>

		<div class="flex flex-col gap-2 sm:flex-row sm:items-center">
			<!-- Search -->
			<div class="relative">
				<Search class="absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 text-surface-400" />
				<input
					type="text"
					placeholder="Search movies..."
					bind:value={searchQuery}
					onkeydown={(e) => e.key === 'Enter' && handleSearch()}
					class="input h-10 w-full rounded-lg border-surface-200 bg-surface-50 pl-9 text-sm placeholder:text-surface-400 focus:border-primary-500 focus:ring-primary-500 sm:w-64 dark:border-surface-800 dark:bg-surface-900"
				/>
			</div>

			<!-- Sort -->
			<div class="flex items-center gap-2">
				<select
					bind:value={sortBy}
					onchange={fetchItems}
					class="select h-10 rounded-lg border-surface-200 bg-surface-50 text-sm dark:border-surface-800 dark:bg-surface-900"
				>
					<option value="added">Date Added</option>
					<option value="title">Title</option>
					<option value="year">Year</option>
					<option value="rating">Rating</option>
				</select>
				<button
					class="variant-ghost-surface btn-icon h-10 w-10 rounded-lg"
					onclick={toggleSortOrder}
					aria-label="Toggle sort order"
				>
					<ArrowUpDown class="h-4 w-4" />
				</button>
			</div>
		</div>
	</div>

	{#if $loading}
		<div class="flex h-64 items-center justify-center">
			<div
				class="h-8 w-8 animate-spin rounded-full border-4 border-primary-500 border-t-transparent"
			></div>
		</div>
	{:else if items.length === 0}
		<div class="flex h-64 flex-col items-center justify-center gap-2 text-surface-400">
			<Search class="h-12 w-12 opacity-50" />
			<p>No movies found</p>
		</div>
	{:else}
		<div class="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
			{#each items as item (item.id)}
				<MediaCard {item} />
			{/each}
		</div>
	{/if}
</div>
