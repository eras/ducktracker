// useThrottle.ts
import { useState, useEffect, useRef } from "react";

/**
 * A custom hook to throttle a value.
 * @param value The value to throttle.
 * @param limit The time in milliseconds to wait before a new update.
 * @returns The throttled value.
 */
function useThrottle<T>(value: T, limit: number): T {
  const [throttledValue, setThrottledValue] = useState<T>(value);
  const lastRan = useRef<number>(Date.now());

  useEffect(() => {
    // Set a timer to update the throttled value.
    const handler = setTimeout(
      () => {
        if (Date.now() - lastRan.current >= limit) {
          setThrottledValue(value);
          lastRan.current = Date.now();
        }
      },
      limit - (Date.now() - lastRan.current),
    );

    // Cleanup function to clear the timer if the component unmounts.
    return () => {
      clearTimeout(handler);
    };
  }, [value, limit]);

  return throttledValue;
}

export default useThrottle;
