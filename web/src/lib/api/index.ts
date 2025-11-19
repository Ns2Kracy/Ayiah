import { createAlova } from 'alova';
import SvelteHook from 'alova/svelte';
import adapterFetch from 'alova/fetch';

export const alovaInstance = createAlova({
    requestAdapter: adapterFetch(),
    statesHook: SvelteHook
});
