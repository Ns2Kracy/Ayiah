<script lang="ts">
	import { useRequest, useWatcher } from 'alova/client';
	import { page } from '$app/state';
	import { libraryApi } from '$lib/api/methods/library';
	import type { SearchResult } from '$lib/api/types';
	import { ArrowLeft, RefreshCw, Search, Clock, Calendar, Star } from '@lucide/svelte';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';

	let itemId = $derived(Number(page.params.id));

	// Main item data
	const {
		loading,
		data,
		send: fetchItem
	} = useWatcher(() => libraryApi.getItem(itemId), [() => itemId], {
		immediate: true
	});

	let item = $derived($data?.data || null);

	// Actions
	const { loading: refreshing, send: refreshMetadata } = useRequest(
		() => libraryApi.refreshMetadata(itemId),
		{ immediate: false }
	);

	const {
		loading: identifyLoading,
		data: candidatesData,
		send: loadCandidates
	} = useRequest(() => libraryApi.getCandidates(itemId), { immediate: false });

	let identifyCandidates = $derived($candidatesData?.data || []);

	const { send: identifyItem } = useRequest(
		(candidate: SearchResult) =>
			libraryApi.identifyItem(itemId, {
				provider: candidate.provider,
				provider_id: candidate.id,
				type: candidate.media_type
			}),
		{ immediate: false }
	);

	// Identify state
	let showIdentify = $state(false);

	let backdropUrl = $derived(
		item?.metadata?.backdrop_path
			? `https://image.tmdb.org/t/p/w1280${item.metadata.backdrop_path}`
			: null
	);

	let posterUrl = $derived(
		item?.metadata?.poster_path
			? `https://image.tmdb.org/t/p/w500${item.metadata.poster_path}`
			: null
	);

	async function handleRefresh() {
		try {
			await refreshMetadata();
			fetchItem();
		} catch (e) {
			console.error('Failed to refresh metadata:', e);
		}
	}

	async function handleIdentify(candidate: SearchResult) {
		try {
			await identifyItem(candidate);
			showIdentify = false;
			fetchItem();
		} catch (e) {
			console.error('Failed to identify item:', e);
		}
	}

	function openIdentify() {
		showIdentify = true;
		loadCandidates();
	}

	function formatRuntime(minutes?: number) {
		if (!minutes) return '';
		const h = Math.floor(minutes / 60);
		const m = minutes % 60;
		return `${h}h ${m}m`;
	}

	function parseGenres(genres?: string) {
		if (!genres) return [];
		try {
			return JSON.parse(genres) as string[];
		} catch {
			return [];
		}
	}

	onMount(() => {
		fetchItem();
	});
</script>

<div class="relative min-h-full">
	{#if $loading}
		<div class="flex h-64 items-center justify-center">
			<div
				class="h-8 w-8 animate-spin rounded-full border-4 border-primary-500 border-t-transparent"
			></div>
		</div>
	{:else if item}
		<!-- Backdrop -->
		<div class="mask-image-gradient-b absolute inset-0 -z-10 h-[50vh] overflow-hidden opacity-20">
			{#if backdropUrl}
				<img src={backdropUrl} alt="" class="h-full w-full object-cover blur-sm" />
			{:else}
				<div class="h-full w-full bg-surface-200 dark:bg-surface-800"></div>
			{/if}
			<div
				class="absolute inset-0 bg-linear-to-b from-transparent to-surface-50 dark:to-surface-950"
			></div>
		</div>

		<div class="mx-auto max-w-6xl space-y-8 pb-10">
			<!-- Header / Back -->
			<div>
				<a href={resolve('/')} class="variant-ghost-surface btn gap-2 btn-sm">
					<ArrowLeft class="h-4 w-4" />
					Back to Library
				</a>
			</div>

			<div class="flex flex-col gap-8 md:flex-row">
				<!-- Poster -->
				<div class="w-48 shrink-0 md:w-72">
					<div
						class="aspect-2/3 overflow-hidden rounded-xl bg-surface-200 shadow-xl dark:bg-surface-800"
					>
						{#if posterUrl}
							<img src={posterUrl} alt={item.title} class="h-full w-full object-cover" />
						{:else}
							<div class="flex h-full w-full items-center justify-center text-surface-400">
								<span class="text-sm">No Poster</span>
							</div>
						{/if}
					</div>
				</div>

				<!-- Info -->
				<div class="flex-1 space-y-6">
					<div>
						<h1 class="text-3xl font-bold md:text-4xl">{item.title}</h1>
						<div
							class="mt-2 flex flex-wrap items-center gap-4 text-sm text-surface-600 dark:text-surface-300"
						>
							{#if item.metadata?.release_date}
								<div class="flex items-center gap-1.5">
									<Calendar class="h-4 w-4" />
									{new Date(item.metadata.release_date).getFullYear()}
								</div>
							{/if}
							{#if item.metadata?.runtime}
								<div class="flex items-center gap-1.5">
									<Clock class="h-4 w-4" />
									{formatRuntime(item.metadata.runtime)}
								</div>
							{/if}
							{#if item.metadata?.vote_average}
								<div class="flex items-center gap-1.5">
									<Star class="h-4 w-4 fill-yellow-500 text-yellow-500" />
									{item.metadata.vote_average.toFixed(1)}
								</div>
							{/if}
							<span class="variant-soft badge capitalize">{item.media_type}</span>
						</div>
					</div>

					<!-- Genres -->
					{#if item.metadata?.genres}
						<div class="flex flex-wrap gap-2">
							{#each parseGenres(item.metadata.genres) as genre (genre)}
								<span class="variant-outline-surface badge">{genre}</span>
							{/each}
						</div>
					{/if}

					<!-- Overview -->
					{#if item.metadata?.overview}
						<div class="prose max-w-none dark:prose-invert">
							<h3 class="text-lg font-semibold">Overview</h3>
							<p>{item.metadata.overview}</p>
						</div>
					{/if}

					<!-- File Info -->
					<div class="rounded-lg bg-surface-100/50 p-4 text-sm dark:bg-surface-900/50">
						<h3 class="mb-2 font-semibold">File Information</h3>
						<div class="grid gap-2 sm:grid-cols-2">
							<div>
								<span class="text-surface-500">Path:</span>
								<span class="break-all">{item.file_path}</span>
							</div>
							<div>
								<span class="text-surface-500">Size:</span>
								{(item.file_size / 1024 / 1024 / 1024).toFixed(2)} GB
							</div>
						</div>
					</div>

					<!-- Actions -->
					<div class="flex gap-2">
						<button
							class="variant-filled-primary btn gap-2"
							onclick={handleRefresh}
							disabled={$refreshing}
						>
							<RefreshCw class="h-4 w-4 {$refreshing ? 'animate-spin' : ''}" />
							{$refreshing ? 'Refreshing...' : 'Refresh Metadata'}
						</button>
						<button class="variant-ghost-surface btn gap-2" onclick={openIdentify}>
							<Search class="h-4 w-4" />
							Identify
						</button>
					</div>
				</div>
			</div>
		</div>
	{:else}
		<div class="p-8 text-center">Item not found</div>
	{/if}

	<!-- Identify Modal -->
	{#if showIdentify}
		<div
			class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4 backdrop-blur-sm"
		>
			<div
				class="flex max-h-[80vh] w-full max-w-2xl flex-col rounded-xl bg-surface-50 shadow-2xl dark:bg-surface-900"
			>
				<div
					class="flex items-center justify-between border-b border-surface-200 p-4 dark:border-surface-800"
				>
					<h2 class="text-xl font-bold">Identify Item</h2>
					<button class="btn-icon btn-icon-sm" onclick={() => (showIdentify = false)}>✕</button>
				</div>

				<div class="flex-1 overflow-y-auto p-4">
					{#if $identifyLoading && identifyCandidates.length === 0}
						<div class="flex justify-center p-8">
							<div
								class="h-8 w-8 animate-spin rounded-full border-4 border-primary-500 border-t-transparent"
							></div>
						</div>
					{:else if identifyCandidates.length > 0}
						<div class="grid gap-2">
							{#each identifyCandidates as candidate (candidate.id)}
								<button
									class="flex items-start gap-3 rounded-lg p-2 text-left transition-colors hover:bg-surface-200 dark:hover:bg-surface-800"
									onclick={() => handleIdentify(candidate)}
								>
									{#if candidate.poster}
										<img
											src={`https://image.tmdb.org/t/p/w92${candidate.poster}`}
											alt=""
											class="h-24 w-16 rounded bg-surface-300 object-cover"
										/>
									{:else}
										<div class="flex h-24 w-16 items-center justify-center rounded bg-surface-300">
											?
										</div>
									{/if}
									<div>
										<div class="font-bold">{candidate.title}</div>
										<div class="text-sm text-surface-500">
											{candidate.year} • {candidate.media_type}
										</div>
										<div class="mt-1 line-clamp-2 text-xs text-surface-600 dark:text-surface-400">
											{candidate.overview}
										</div>
									</div>
								</button>
							{/each}
						</div>
					{:else}
						<p class="text-center text-surface-500">No candidates found.</p>
					{/if}
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.mask-image-gradient-b {
		mask-image: linear-gradient(to bottom, black 0%, transparent 100%);
	}
</style>
