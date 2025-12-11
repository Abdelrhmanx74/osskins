import { useState, useEffect } from "react";
import { imageCache } from "../imageCache";

/**
 * Hook for using images with caching support
 */
export function useCachedImage(src: string | undefined) {
    const [loaded, setLoaded] = useState(false);
    const [error, setError] = useState(false);

    useEffect(() => {
        if (!src) {
            setLoaded(false);
            return;
        }

        // Check if already cached
        if (imageCache.has(src)) {
            setLoaded(true);
            return;
        }

        // Start preloading
        setLoaded(false);
        setError(false);

        imageCache
            .preload(src)
            .then(() => setLoaded(true))
            .catch(() => setError(true));
    }, [src]);

    return { loaded, error };
}

/**
 * Hook for preloading images in the background
 */
export function useImagePreloader(sources: string[]) {
    useEffect(() => {
        if (sources.length === 0) return;

        // Preload in background without blocking
        void imageCache.preloadBatch(sources);
    }, [sources]);
}
