import { Sound } from '@/types';

// Pointer travel before a press on a sound turns into a drag (so plain clicks still play).
const DRAG_THRESHOLD = 6;
const HOVER_CLASS = 'vx-drop-hover';

// Drag a sound onto any element carrying `data-tag-drop="<tag>"`. Pointer events are used because
// native HTML5 drag-and-drop is intercepted by the webview's file-drop handling.
export function pressSound(e: MouseEvent, sound: Sound, onDrop: (sound: Sound, tag: string) => void): void {
  if (e.button !== 0) return;
  const startX = e.clientX;
  const startY = e.clientY;
  let ghost: HTMLElement | null = null;
  let hover: HTMLElement | null = null;

  const targetAt = (x: number, y: number): HTMLElement | null => {
    const el = document.elementFromPoint(x, y);
    return el ? (el.closest('[data-tag-drop]') as HTMLElement | null) : null;
  };

  const onMove = (ev: MouseEvent) => {
    if (!ghost) {
      if (Math.hypot(ev.clientX - startX, ev.clientY - startY) < DRAG_THRESHOLD) return;
      ghost = document.createElement('div');
      ghost.className = 'vx-drag-ghost';
      ghost.textContent = sound.name;
      document.body.appendChild(ghost);
      document.body.classList.add('vx-dragging-sound');
    }
    ghost.style.left = `${ev.clientX + 12}px`;
    ghost.style.top = `${ev.clientY + 12}px`;
    const target = targetAt(ev.clientX, ev.clientY);
    if (target !== hover) {
      hover?.classList.remove(HOVER_CLASS);
      target?.classList.add(HOVER_CLASS);
      hover = target;
    }
  };

  const onUp = (ev: MouseEvent) => {
    window.removeEventListener('mousemove', onMove);
    window.removeEventListener('mouseup', onUp);
    if (!ghost) return;
    const target = targetAt(ev.clientX, ev.clientY);
    ghost.remove();
    hover?.classList.remove(HOVER_CLASS);
    document.body.classList.remove('vx-dragging-sound');
    // swallow the click that follows the release so the sound doesn't start playing
    window.addEventListener('click', swallow, true);
    setTimeout(() => window.removeEventListener('click', swallow, true), 0);
    const tag = target?.getAttribute('data-tag-drop');
    if (tag) onDrop(sound, tag);
  };

  window.addEventListener('mousemove', onMove);
  window.addEventListener('mouseup', onUp);
}

function swallow(ev: Event): void {
  ev.stopPropagation();
}
