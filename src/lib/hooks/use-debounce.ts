import { useCallback, useState } from "react";

/**
 * Hook for debouncing values
 * @param delay - Delay in milliseconds
 */
export function useDebounce<T>(delay: number = 300) {
    const [timeoutId, setTimeoutId] = useState<NodeJS.Timeout | null>(null);

    const debounce = useCallback(
        (callback: () => void) => {
            if (timeoutId) {
                clearTimeout(timeoutId);
            }

            const newTimeoutId = setTimeout(() => {
                callback();
                setTimeoutId(null);
            }, delay);

            setTimeoutId(newTimeoutId);
        },
        [timeoutId, delay],
    );

    const cancel = useCallback(() => {
        if (timeoutId) {
            clearTimeout(timeoutId);
            setTimeoutId(null);
        }
    }, [timeoutId]);

    return { debounce, cancel };
}
