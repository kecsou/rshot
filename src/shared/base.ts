// WebKit's default context menu (Reload, Inspect Element…) has no place in a shipped app.
if (import.meta.env.PROD) document.addEventListener('contextmenu', (e) => e.preventDefault());
