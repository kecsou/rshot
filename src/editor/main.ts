import '../shared/base';
import '../shared/glass.css';
import './editor.css';
import { mountIcons } from '../shared/icons';
import * as ipc from '../shared/ipc';

// Minimal round-trip editor; Task 4 replaces this file.
mountIcons();
const info = await ipc.editorInfo();
const img = await createImageBitmap(new Blob([await ipc.readCapture(info.path)], { type: 'image/png' }));
const canvas = document.querySelector<HTMLCanvasElement>('#view')!;
canvas.width = img.width;
canvas.height = img.height;
canvas.style.width = `${Math.min(img.width, 900)}px`;
canvas.getContext('2d')!.drawImage(img, 0, 0);
document.querySelector('#title')!.textContent = info.name;
document.querySelector('#path')!.textContent = info.display;
document.querySelector('#done')!.addEventListener('click', async () => {
  const blob = await new Promise<Blob>((ok) => canvas.toBlob((b) => ok(b!), 'image/png'));
  await ipc.saveImage(new Uint8Array(await blob.arrayBuffer()));
  await ipc.closeWindow();
});
document.querySelector('#close')!.addEventListener('click', () => void ipc.closeWindow());
