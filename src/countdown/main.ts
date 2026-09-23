import '../shared/base';
import '../shared/glass.css';
import * as ipc from '../shared/ipc';

const secs = await ipc.countdownInfo();
const num = document.querySelector<HTMLElement>('#num')!;
const arc = document.querySelector<SVGCircleElement>('#arc')!;
const C = 2 * Math.PI * 54;
arc.style.strokeDasharray = `${C}`;
arc.style.strokeDashoffset = '0';
let left = secs;
num.textContent = String(left);
requestAnimationFrame(() => {
  arc.style.transition = `stroke-dashoffset ${secs}s linear`;
  arc.style.strokeDashoffset = `${C}`;
});
const timer = setInterval(() => {
  left -= 1;
  if (left > 0) num.textContent = String(left);
  else {
    clearInterval(timer);
    document.body.hidden = true;
    void ipc.countdownDone();
  }
}, 1000);
const cancel = () => {
  clearInterval(timer);
  void ipc.countdownCancel();
};
addEventListener('keydown', (e) => e.key === 'Escape' && cancel());
addEventListener('mousedown', cancel);
