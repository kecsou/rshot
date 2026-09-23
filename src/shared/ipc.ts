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
  mic: string | null;
};
export type OverlayInfo = {
  token: number;
  mode: 'area' | 'window' | 'screen' | 'recarea' | 'recscreen';
  index: number;
  active: boolean;
  width: number;
  height: number;
  windows: WinRect[];
  selection: Rect | null;
  hints: boolean;
  options: OverlayOptions;
  /** Why the record modes are disabled (no ffmpeg, or a recording is running); null: available. */
  record_off: string | null;
};
export type Target = { kind: 'area'; rect: Rect } | { kind: 'window'; id: number; rect: Rect } | { kind: 'screen' };

export const overlayInfo = () => invoke<OverlayInfo | null>('overlay_info');
export const overlayFrame = () => invoke<ArrayBuffer>('overlay_frame');
export const overlayReady = (token: number) => invoke<void>('overlay_ready', { token });
export const overlayActivate = (token: number) => invoke<void>('overlay_activate', { token });
export const overlayCancel = () => invoke<void>('overlay_cancel');
export const overlayCapture = (token: number, target: Target) => invoke<void>('overlay_capture', { token, target });
/** Records `rect` (null: the whole monitor) after a 3 s countdown. */
export const overlayRecord = (token: number, rect: Rect | null) => invoke<void>('overlay_record', { token, rect });
export const setOverlayOptions = (options: OverlayOptions) => invoke<void>('set_overlay_options', { options });
export const pickFolder = () => invoke<string | null>('pick_folder');

export type Thumb = { path: string; display: string; copied: boolean; kind: 'image' | 'video' };
export const thumbnailInfo = () => invoke<Thumb | null>('thumbnail_info');
export const readCapture = (path: string) => invoke<ArrayBuffer>('read_capture', { path });
export const revealCapture = (path: string) => invoke<void>('reveal_capture', { path });
export const deleteCapture = (path: string) => invoke<void>('delete_capture', { path });
export const retryCopy = (path: string) => invoke<void>('retry_copy', { path });
export const dismissThumbnail = () => invoke<void>('close_window'); // this card only
/** A recording's duration in seconds (NaN if unknown) and first frame. Raw bytes: an f64 (LE), then the PNG. */
export const videoPoster = async (path: string) => {
  const b = await invoke<ArrayBuffer>('video_poster', { path });
  return { duration: new DataView(b).getFloat64(0, true), png: b.slice(8) };
};

export const countdownInfo = () => invoke<number>('countdown_info');
export const countdownDone = () => invoke<void>('countdown_done');
export const countdownCancel = () => invoke<void>('countdown_cancel');

export type ClipboardMode = 'path-and-image' | 'path-only';
export type Shortcuts = { area: string; screen: string; window: string; record: string };
export type Settings = {
  launch_at_login: boolean;
  screenshots_dir: string;
  clipboard_mode: ClipboardMode;
  show_thumbnail: boolean;
  shutter_sound: boolean;
  takeover: boolean;
  shortcuts: Shortcuts;
  takeover_error: string | null;
  manual: [string, string][];
  recordings_dir: string;
  mic: string | null;
  fps: number;
};
export type BoolSetting = 'launch_at_login' | 'show_thumbnail' | 'shutter_sound' | 'takeover';
export const getSettings = () => invoke<Settings>('get_settings');
export const setSettings = (settings: Settings) => invoke<Settings>('set_settings', { settings });
export const onboardingChoice = (accept: boolean) => invoke<void>('onboarding_choice', { accept });
export const openConfig = () => invoke<void>('open_config');
export const closeWindow = () => invoke<void>('close_window');

/** `stream`: a video editor's loopback URL for its file (null for an image). */
export type EditorInfo = { path: string; display: string; name: string; stream: string | null };
export const openEditor = (path: string) => invoke<void>('open_editor', { path });
export const editorInfo = () => invoke<EditorInfo>('editor_info');
/** Sends the PNG as a raw IPC body (no JSON/base64 round-trip). False = written, but not copied. */
export const saveImage = (png: Uint8Array) => invoke<boolean>('save_image', png);
export const editorDelete = () => invoke<void>('editor_delete');
export const copyPath = (path: string) => invoke<void>('copy_path', { path });

export type Mic = { id: string; label: string };
export const listMics = () => invoke<Mic[]>('list_mics');

/** `level`: the mic's dBFS (-100..0), null without a mic. Null info: nothing is recording. */
export type RecordingInfo = { elapsed_ms: number; mic: boolean; level: number | null };
export const recordingInfo = () => invoke<RecordingInfo | null>('recording_info');
export const recordingStop = () => invoke<void>('recording_stop');
export const recordingDiscard = () => invoke<void>('recording_discard');
/** Keeps [start, end] s of the open recording, saved in place. False = trimmed, but not copied. */
export const trimVideo = (start: number, end: number, mute: boolean) => invoke<boolean>('trim_video', { start, end, mute });
