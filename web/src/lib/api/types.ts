export interface ApiResponse<T> {
	code: number;
	message: string;
	data?: T;
}

export type MediaType = 'movie' | 'tv' | 'comic' | 'book';

export interface MediaItem {
	id: number;
	library_folder_id: number;
	media_type: MediaType;
	title: string;
	file_path: string;
	file_size: number;
	added_at: string;
	updated_at: string;
}

export interface VideoMetadata {
	id: number;
	media_item_id: number;
	tmdb_id?: number;
	tvdb_id?: number;
	imdb_id?: string;
	overview?: string;
	poster_path?: string;
	backdrop_path?: string;
	release_date?: string;
	runtime?: number;
	vote_average?: number;
	vote_count?: number;
	genres?: string; // JSON string
	created_at: string;
	updated_at: string;
}

export interface MediaItemWithMetadata extends MediaItem {
	metadata?: VideoMetadata;
}

export interface LibraryResponse {
	items: MediaItemWithMetadata[];
	total: number;
}

export interface LibraryQuery {
	page?: number;
	limit?: number;
	sort?: string;
	order?: string;
	search?: string;
}

export interface BatchRefreshRequest {
	ids: number[];
}

export interface BatchRefreshError {
	id: number;
	error: string;
}

export interface BatchRefreshResponse {
	success: number[];
	failed: BatchRefreshError[];
}

export interface IdentifyRequest {
	provider: string;
	provider_id: string;
	type: string;
}

export interface SearchResult {
	id: string;
	title: string;
	year?: number;
	media_type: string;
	poster?: string;
	overview?: string;
	provider: string;
}
