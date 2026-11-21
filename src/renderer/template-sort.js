export function enableTemplateSorting(list, move) {
  let drag;
  let animation = 0;

  function clearTarget() {
    drag.target?.classList.remove('drop-before', 'drop-after');
    drag.target = null;
  }

  function updateTarget() {
    animation = 0;
    if (!drag) return;
    clearTarget();
    const bounds = list.getBoundingClientRect();
    if (drag.x < bounds.left - 12 || drag.x > bounds.right + 12) return;

    const edge = 22;
    const speed = drag.y < bounds.top + edge ? -7 : drag.y > bounds.bottom - edge ? 7 : 0;
    const oldScroll = list.scrollTop;
    list.scrollTop += speed;
    if (list.scrollTop !== oldScroll) animation = requestAnimationFrame(updateTarget);

    const y = Math.max(bounds.top, Math.min(bounds.bottom, drag.y));
    const choices = Array.from(list.children).filter((row) => row !== drag.row);
    drag.target = choices.find((row) => {
      const rect = row.getBoundingClientRect();
      return y < rect.top + rect.height / 2;
    });
    drag.after = !drag.target;
    drag.target ??= choices.at(-1);
    drag.target?.classList.add(drag.after ? 'drop-after' : 'drop-before');
  }

  function finish(event, dropped) {
    if (!drag || event.pointerId !== drag.pointer) return;
    const { row, target, after, handle, pointer } = drag;
    cancelAnimationFrame(animation);
    animation = 0;
    clearTarget();
    drag = null;
    row.classList.remove('dragging');
    if (handle.hasPointerCapture(pointer)) handle.releasePointerCapture(pointer);
    if (dropped && target) move(row.dataset.id, target.dataset.id, after);
  }

  list.addEventListener('pointerdown', (event) => {
    const handle = event.target.closest('.drag-handle');
    if (!handle || handle.disabled || drag || event.button !== 0) return;
    drag = {
      row: handle.closest('article'), handle, pointer: event.pointerId,
      start: event.clientY, y: event.clientY, x: event.clientX, moved: false,
    };
    handle.setPointerCapture(event.pointerId);
  });
  list.addEventListener('pointermove', (event) => {
    if (!drag || event.pointerId !== drag.pointer) return;
    if (!drag.moved && Math.abs(event.clientY - drag.start) < 4) return;
    drag.moved = true;
    drag.x = event.clientX;
    drag.y = event.clientY;
    drag.row.classList.add('dragging');
    if (!animation) updateTarget();
  });
  list.addEventListener('pointerup', (event) => finish(event, true));
  list.addEventListener('pointercancel', (event) => finish(event, false));
  list.addEventListener('lostpointercapture', (event) => finish(event, false));
  list.addEventListener('keydown', async (event) => {
    const handle = event.target.closest('.drag-handle');
    if (!handle || event.ctrlKey || event.altKey || event.metaKey || event.shiftKey) return;
    if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') return;
    const row = handle.closest('article');
    const after = event.key === 'ArrowDown';
    const target = after ? row.nextElementSibling : row.previousElementSibling;
    if (!target) return;
    event.preventDefault();
    const id = row.dataset.id;
    if (await move(id, target.dataset.id, after)) {
      Array.from(list.children).find((item) => item.dataset.id === id)?.querySelector('.drag-handle').focus();
    }
  });
}
