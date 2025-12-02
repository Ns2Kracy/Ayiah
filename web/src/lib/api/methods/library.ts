import { alovaInstance } from '../index';
import type {
    ApiResponse,
    LibraryResponse,
    LibraryQuery,
    MediaItemWithMetadata,
    SearchResult,
    IdentifyRequest,
    BatchRefreshRequest,
    BatchRefreshResponse
} from '../types';

export const libraryApi = {
    // Get all items
    getAllItems: (params?: LibraryQuery) =>
        alovaInstance.Get<ApiResponse<LibraryResponse>>('/api/library', { params }),

    // Get movies
    getMovies: (params?: LibraryQuery) =>
        alovaInstance.Get<ApiResponse<LibraryResponse>>('/api/library/movies', { params }),

    // Get TV shows
    getTvShows: (params?: LibraryQuery) =>
        alovaInstance.Get<ApiResponse<LibraryResponse>>('/api/library/tv', { params }),

    // Get media item by ID
    getItem: (id: number) =>
        alovaInstance.Get<ApiResponse<MediaItemWithMetadata>>(`/api/library/items/${id}`),

    // Refresh metadata
    refreshMetadata: (id: number) =>
        alovaInstance.Post<ApiResponse<string>>(`/api/library/items/${id}/refresh`),

    // Batch refresh
    batchRefresh: (req: BatchRefreshRequest) =>
        alovaInstance.Post<ApiResponse<BatchRefreshResponse>>('/api/library/batch/refresh', req),

    // Identify item
    identifyItem: (id: number, req: IdentifyRequest) =>
        alovaInstance.Post<ApiResponse<string>>(`/api/library/items/${id}/identify`, req),

    // Get identify candidates
    getCandidates: (id: number) =>
        alovaInstance.Get<ApiResponse<SearchResult[]>>(`/api/library/items/${id}/candidates`)
};
