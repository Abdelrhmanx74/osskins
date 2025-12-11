/**
 * Image caching system for champion icons and skin splashes
 * Uses browser's native image caching with optimized preloading
 */

class ImageCacheManager {
    private cache: Map<string, HTMLImageElement> = new Map();
    private loadingPromises: Map<string, Promise<void>> = new Map();

    /**
     * Preload an image and store it in cache
     */
    preload(src: string): Promise<void> {
        // Return existing promise if already loading
        if (this.loadingPromises.has(src)) {
            return this.loadingPromises.get(src)!;
        }

        // Return immediately if already cached
        if (this.cache.has(src)) {
            return Promise.resolve();
        }

        const promise = new Promise<void>((resolve, reject) => {
            const img = new Image();
            img.onload = () => {
                this.cache.set(src, img);
                this.loadingPromises.delete(src);
                resolve();
            };
            img.onerror = () => {
                this.loadingPromises.delete(src);
                reject(new Error(`Failed to load image: ${src}`));
            };
            img.src = src;
        });

        this.loadingPromises.set(src, promise);
        return promise;
    }

    /**
     * Preload multiple images in parallel
     */
    async preloadBatch(sources: string[]): Promise<void> {
        await Promise.allSettled(sources.map((src) => this.preload(src)));
    }

    /**
     * Check if an image is cached
     */
    has(src: string): boolean {
        return this.cache.has(src);
    }

    /**
     * Get a cached image
     */
    get(src: string): HTMLImageElement | undefined {
        return this.cache.get(src);
    }

    /**
     * Clear specific images from cache
     */
    clear(sources?: string[]): void {
        if (sources) {
            sources.forEach((src) => this.cache.delete(src));
        } else {
            this.cache.clear();
        }
    }

    /**
     * Get cache size
     */
    size(): number {
        return this.cache.size;
    }
}

// Export singleton instance
export const imageCache = new ImageCacheManager();
