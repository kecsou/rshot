export const SPRITE = `<svg xmlns="http://www.w3.org/2000/svg" style="display:none">
<symbol id="i-x" viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18"/></symbol>
<symbol id="i-screen" viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/></symbol>
<symbol id="i-window" viewBox="0 0 24 24"><rect x="3" y="5" width="18" height="14" rx="2"/><path d="M3 9h18"/></symbol>
<symbol id="i-area" viewBox="0 0 24 24"><path d="M4 8V5a1 1 0 0 1 1-1h3M16 4h3a1 1 0 0 1 1 1v3M20 16v3a1 1 0 0 1-1 1h-3M8 20H5a1 1 0 0 1-1-1v-3"/></symbol>
<symbol id="i-down" viewBox="0 0 24 24"><path d="M6 9l6 6 6-6"/></symbol>
<symbol id="i-folder" viewBox="0 0 24 24"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/></symbol>
<symbol id="i-trash" viewBox="0 0 24 24"><path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/></symbol>
<symbol id="i-check" viewBox="0 0 24 24"><path d="M5 12.5l4.5 4.5L19 7.5"/></symbol>
<symbol id="i-logo" viewBox="0 0 24 24"><path d="M4 8V5a1 1 0 0 1 1-1h3M16 4h3a1 1 0 0 1 1 1v3M20 16v3a1 1 0 0 1-1 1h-3M8 20H5a1 1 0 0 1-1-1v-3"/><circle cx="12" cy="12" r="3"/></symbol>
<symbol id="t-select" viewBox="0 0 24 24"><path d="M6 3l12 8-5.5 1.3L9.5 18z"/></symbol>
<symbol id="t-crop" viewBox="0 0 24 24"><path d="M6 2v14a2 2 0 0 0 2 2h14M2 6h14a2 2 0 0 1 2 2v14"/></symbol>
<symbol id="t-arrow" viewBox="0 0 24 24"><path d="M5 19L19 5M10 5h9v9"/></symbol>
<symbol id="t-rect" viewBox="0 0 24 24"><rect x="4" y="5" width="16" height="14" rx="2"/></symbol>
<symbol id="t-ellipse" viewBox="0 0 24 24"><ellipse cx="12" cy="12" rx="9" ry="7"/></symbol>
<symbol id="t-line" viewBox="0 0 24 24"><path d="M5 19L19 5"/></symbol>
<symbol id="t-pen" viewBox="0 0 24 24"><path d="M3 18c2.5-5 4.5-9 6.5-9s1 6 3.5 6 3-7 5-7 2 3 3 4"/></symbol>
<symbol id="t-hl" viewBox="0 0 24 24"><path d="M14.5 4.5l5 5L11 18H6v-5z"/><path d="M3 21h9" stroke-width="3"/></symbol>
<symbol id="t-text" viewBox="0 0 24 24"><path d="M5 7V5h14v2M12 5v14M9 19h6"/></symbol>
<symbol id="t-num" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/><path d="M10.5 9.5L12.5 8v8"/></symbol>
<symbol id="t-blur" viewBox="0 0 24 24"><rect x="4" y="4" width="5" height="5" rx="1"/><rect x="15" y="4" width="5" height="5" rx="1" fill="currentColor"/><rect x="9.5" y="9.5" width="5" height="5" rx="1" fill="currentColor"/><rect x="4" y="15" width="5" height="5" rx="1" fill="currentColor"/><rect x="15" y="15" width="5" height="5" rx="1"/></symbol>
<symbol id="i-undo" viewBox="0 0 24 24"><path d="M9 14L4 9l5-5M4 9h10.5a5.5 5.5 0 0 1 0 11H11"/></symbol>
<symbol id="i-redo" viewBox="0 0 24 24"><path d="M15 14l5-5-5-5M20 9H9.5a5.5 5.5 0 0 0 0 11H13"/></symbol>
<symbol id="i-copy" viewBox="0 0 24 24"><rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/></symbol>
<symbol id="i-minus" viewBox="0 0 24 24"><path d="M6 12h12"/></symbol>
<symbol id="i-plus" viewBox="0 0 24 24"><path d="M12 6v12M6 12h12"/></symbol>
<symbol id="i-pen" viewBox="0 0 24 24"><path d="M4 20h4L19 9l-4-4L4 16z"/></symbol>
<symbol id="e-one" viewBox="0 0 24 24"><path d="M4 12h14M13 7l5 5-5 5"/></symbol>
<symbol id="e-two" viewBox="0 0 24 24"><path d="M4 12h16M8 8l-4 4 4 4M16 8l4 4-4 4"/></symbol>
<symbol id="e-none" viewBox="0 0 24 24"><path d="M4 12h16"/></symbol>
</svg>`;

/** Inserts the sprite and fills every `[data-icon]` element with its icon. */
export function mountIcons(): void {
  document.body.insertAdjacentHTML('afterbegin', SPRITE);
  document.querySelectorAll<HTMLElement>('[data-icon]').forEach((el) => {
    el.insertAdjacentHTML('afterbegin', `<svg class="ic" aria-hidden="true"><use href="#${el.dataset.icon}"/></svg>`);
  });
}
