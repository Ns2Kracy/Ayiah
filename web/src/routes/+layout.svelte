<script lang="ts">
	import './layout.css';
	import icon from '$lib/assets/icon.svg';
	import Sidebar from '$lib/components/layout/Sidebar.svelte';
	import Header from '$lib/components/layout/Header.svelte';
	import Footer from '$lib/components/layout/Footer.svelte';

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={icon} />
</svelte:head>

<div class="grid h-screen w-full lg:grid-cols-[280px_1fr]">
	<!-- Desktop Sidebar -->
	<aside class="hidden border-r bg-surface-100/50 lg:block dark:bg-surface-900/50">
		<Sidebar />
	</aside>

	<!-- Main Content -->
	<div class="flex flex-col">
		<Header />
		<main class="flex-1 overflow-y-auto bg-surface-50 p-4 lg:p-6 dark:bg-surface-950">
			{@render children()}
		</main>
		<Footer />
	</div>
</div>
