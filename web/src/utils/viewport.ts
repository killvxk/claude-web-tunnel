/**
 * Viewport utilities for mobile adaptation
 * Handles virtual keyboard and viewport height issues on mobile devices
 */

// Set CSS variable for actual viewport height
function setViewportHeight() {
  // Use visualViewport if available (better for virtual keyboard handling)
  const vh = window.visualViewport?.height ?? window.innerHeight;
  document.documentElement.style.setProperty('--viewport-height', `${vh}px`);
}

// Debounce helper
function debounce<T extends (...args: unknown[]) => void>(fn: T, ms: number): T {
  let timeoutId: ReturnType<typeof setTimeout>;
  return ((...args: unknown[]) => {
    clearTimeout(timeoutId);
    timeoutId = setTimeout(() => fn(...args), ms);
  }) as T;
}

/**
 * Setup viewport height handler
 * Call this function once during app initialization
 */
export function setupViewportHandler(): () => void {
  // Initial set
  setViewportHeight();

  // Debounced handler for resize events
  const debouncedSetHeight = debounce(setViewportHeight, 100);

  // Listen to visualViewport events if available
  if (window.visualViewport) {
    window.visualViewport.addEventListener('resize', debouncedSetHeight);
    window.visualViewport.addEventListener('scroll', debouncedSetHeight);
  }

  // Fallback to window resize
  window.addEventListener('resize', debouncedSetHeight);
  window.addEventListener('orientationchange', () => {
    // Delay on orientation change to get correct values
    setTimeout(setViewportHeight, 100);
  });

  // Return cleanup function
  return () => {
    if (window.visualViewport) {
      window.visualViewport.removeEventListener('resize', debouncedSetHeight);
      window.visualViewport.removeEventListener('scroll', debouncedSetHeight);
    }
    window.removeEventListener('resize', debouncedSetHeight);
    window.removeEventListener('orientationchange', setViewportHeight);
  };
}

/**
 * Check if the current device is likely a mobile device
 */
export function isMobileDevice(): boolean {
  return /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(
    navigator.userAgent
  );
}

/**
 * Check if the current device is likely a touch device
 */
export function isTouchDevice(): boolean {
  return 'ontouchstart' in window || navigator.maxTouchPoints > 0;
}
