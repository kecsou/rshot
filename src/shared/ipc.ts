import { invoke } from '@tauri-apps/api/core';

/** Rectangles are in image pixels of one monitor frame. */
export type Rect = { x: number; y: number; w: number; h: number };
export type WinRect = Rect & { id: number; title: string; app: string };
export type OverlayOptions = {
  timer_secs: number;
  show_thumbnail: boolean;
  remember_selection: boolean;
  show_pointer: boolean;
  screenshots_dir: string;
};
export type OverlayInfo = {
  token: number;
  mode: 'area' | 'window' | 'screen';
  index: number;
  active: boolean;
  width: number;
  height: number;
  windows: WinRect[];
  selection: Rect | null;
  hints: boolean;
  options: OverlayOptions;
};
export type Target = { kind: 'area'; rect: Rect } | { kind: 'window'; id: number; rect: Rect } | { kind: 'screen' };

export const overlayInfo = () => invoke<OverlayInfo | null>('overlay_info');
export const overlayFrame = () => invoke<ArrayBuffer>('overlay_frame');
export const overlayReady = (token: number) => invoke<void>('overlay_ready', { token });
export const overlayActivate = (token: number) => invoke<void>('overlay_activate', { token });
export const overlayCancel = () => invoke<void>('overlay_cancel');
export const overlayCapture = (token: number, target: Target) => invoke<void>('overlay_capture', { token, target });
