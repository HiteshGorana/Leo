// Leo Link - Content Script
// Future use for more complex DOM interaction/extraction

console.log("Leo Link: Content script active");

// Optional: Inform background script when a page is fully loaded/stable
window.addEventListener('load', () => {
    // chrome.runtime.sendMessage({ type: "page_loaded", url: window.location.href });
});
