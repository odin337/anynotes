import { enableTemplateSorting } from './template-sort.js';

const { invoke } = window.__TAURI__.core;
const api = {
  session: () => invoke('session'),
  apply: (action) => invoke('apply', { action }),
  pin: (pinned) => invoke('pin', { pinned }),
  copy: (id) => invoke('copy', { id }),
  authorizeKeys: () => invoke('authorize_keys'),
};
const get = (id) => document.getElementById(id);
const tabs = [...document.querySelectorAll('.tab')];
const add = get('add');
const pin = get('pin');
const keysAccess = get('keys-access');
const noteList = get('note-list');
const notePicker = get('note-select');
const paper = get('paper');
const noteTitle = get('note-title');
const noteText = get('note-text');
const noteStatus = get('note-status');
const palette = get('palette');
const templateEditor = get('template-editor');
const templateTitle = get('template-title');
const templateText = get('template-text');
const deleteDialog = get('delete-dialog');
const toast = get('toast');
const templateList = get('template-list');
enableTemplateSorting(templateList, (id, target, after) => change({ type: 'template:move', id, target, after }));
const formattedDate = new Intl.DateTimeFormat('ru-RU', {
  day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit',
});

const noteDrafts = new Map();
let data;
let activePanel = 'notes';
let selectedNoteId;
let noteListSignature;
let historySignature;
let templatesSignature;
let editingTemplate;
let deleting;
let toastTimer;
let isMac = false;

function node(tag, className, text) {
  const element = document.createElement(tag);
  if (className) element.className = className;
  if (text !== undefined) element.textContent = text;
  return element;
}

function icon(name) {
  const element = node('i', 'ph ' + name);
  element.setAttribute('aria-hidden', 'true');
  return element;
}

function describeButton(button, label) {
  button.title = label;
  button.setAttribute('aria-label', label);
}

function notify(message, failed = false) {
  clearTimeout(toastTimer);
  toast.firstElementChild.className = failed ? 'ph warning' : 'ph check';
  toast.lastElementChild.textContent = message;
  toast.classList.toggle('error', failed);
  toast.hidden = false;
  toastTimer = setTimeout(() => { toast.hidden = true; }, 2700);
}

function selectPanel(id) {
  activePanel = id;
  for (const tab of tabs) {
    const current = tab.id === 'tab-' + id;
    tab.setAttribute('aria-selected', String(current));
    tab.tabIndex = current ? 0 : -1;
    get(tab.getAttribute('aria-controls')).hidden = !current;
  }
  add.hidden = id === 'counter';
  const label = id === 'templates' ? 'Новый шаблон' : 'Новая заметка';
  describeButton(add, label);
}

function noteWithDrafts(note) {
  const drafts = noteDrafts.get(note.id);
  if (!drafts) return note;
  return {
    ...note,
    title: drafts.get('title')?.value ?? note.title,
    text: drafts.get('text')?.value ?? note.text,
    color: drafts.get('color')?.value ?? note.color,
  };
}

function noteName(note) {
  return note.title.trim() || 'Без названия';
}

function makeNoteItem(note) {
  const button = node('button', 'note-item');
  button.type = 'button';
  button.dataset.id = note.id;
  button.setAttribute('aria-current', String(note.id === selectedNoteId));

  const badge = node('span', 'note-badge');
  badge.dataset.color = note.color;
  const meta = node('span', 'note-meta');
  meta.append(node('strong', null, note.title), node('small', null, note.preview));
  button.append(badge, meta, icon('next'));
  return button;
}

function renderNotes() {
  const notes = data.notes.map(noteWithDrafts);
  const current = notes.find((note) => note.id === selectedNoteId) ?? notes[0];
  selectedNoteId = current?.id;
  get('notes').classList.toggle('has-multiple', notes.length > 1);
  get('notes-empty').hidden = Boolean(current);
  paper.hidden = !current;
  paper.previousElementSibling.hidden = !current;

  const entries = notes.map((note) => ({
    id: note.id,
    title: noteName(note),
    color: note.color,
    preview: note.text.trimStart().split('\n', 1)[0].slice(0, 100) || 'Пустая заметка',
  }));
  const signature = JSON.stringify([selectedNoteId, entries]);
  if (noteListSignature !== signature) {
    const focusedId = noteList.contains(document.activeElement) ? document.activeElement.dataset.id : null;
    const items = entries.map(makeNoteItem);
    noteList.replaceChildren(...items);
    items.find((item) => item.dataset.id === focusedId)?.focus();
    notePicker.replaceChildren(...entries.map(({ title, id }) => new Option(title, id)));
    notePicker.value = selectedNoteId ?? '';
    noteListSignature = signature;
  }

  if (!current) return;

  if (noteTitle.value !== current.title) noteTitle.value = current.title;
  if (noteText.value !== current.text) noteText.value = current.text;
  paper.dataset.color = current.color;
  for (const button of palette.children) {
    button.setAttribute('aria-pressed', String(button.dataset.color === current.color));
  }

  const drafts = [...(noteDrafts.get(current.id)?.values() ?? [])];
  const failed = drafts.some((draft) => draft.failed);
  const pending = drafts.some((draft) => !draft.failed);
  const status = failed ? 'Ошибка сохранения' : pending ? 'Сохраняется…' : 'Сохранено';
  noteStatus.classList.toggle('error', failed);
  noteStatus.firstElementChild.className = failed ? 'ph warning' : pending ? 'ph history' : 'ph check';
  if (noteStatus.lastElementChild.textContent !== status) noteStatus.lastElementChild.textContent = status;
}

function renderCounter() {
  const { value, history } = data.counter;
  const output = get('counter-value');
  const text = String(value).replace('-', '−');
  if (output.textContent !== text) output.textContent = text;
  output.classList.toggle('long', text.length > 5 && text.length <= 11);
  output.classList.toggle('huge', text.length > 11);

  const signature = JSON.stringify(history);
  if (signature === historySignature) return;
  historySignature = signature;
  const list = get('history-list');
  list.hidden = !history.length;
  get('history-empty').hidden = Boolean(history.length);
  list.replaceChildren(...history.map(({ value: number, at }) => {
    const item = node('li');
    const date = new Date(at);
    const time = node('time', null, formattedDate.format(date));
    time.dateTime = date.toISOString();
    item.append(node('strong', null, String(number).replace('-', '−')), time);
    return item;
  }));
}

function templateAction(action, iconName, label, className = 'icon-button') {
  const button = node('button', className);
  button.type = 'button';
  button.dataset.action = action;
  describeButton(button, label);
  button.append(icon(iconName));
  return button;
}

function makeTemplateCard(template) {
  const article = node('article', 'template-card');
  article.dataset.id = template.id;
  const description = node('div', 'template-description');
  description.append(node('h2', null, template.title), node('p', null, template.text.slice(0, 500)));

  const actions = node('div', 'template-actions');
  const handle = templateAction('drag', 'grip', 'Переместить: ' + template.title, 'icon-button drag-handle');
  handle.title = 'Перетащите вверх или вниз';
  handle.setAttribute('aria-keyshortcuts', 'ArrowUp ArrowDown');
  handle.disabled = data.templates.length < 2;
  const copy = templateAction('copy', 'copy', 'Копировать: ' + template.title, 'button copy-button');
  copy.append(node('span', null, 'Копировать'));
  actions.append(
    copy,
    templateAction('edit', 'edit', 'Редактировать шаблон'),
    templateAction('delete', 'trash', 'Удалить шаблон', 'icon-button delete-button'),
  );
  article.append(handle, description, actions);
  return article;
}

function renderTemplates() {
  const signature = JSON.stringify(data.templates);
  if (signature === templatesSignature) return;
  templatesSignature = signature;
  templateList.hidden = !data.templates.length;
  get('templates-empty').hidden = Boolean(data.templates.length);
  templateList.replaceChildren(...data.templates.map(makeTemplateCard));
}

function receive(snapshot) {
  if (data && snapshot.revision < data.revision) return;
  data = snapshot;
  renderNotes();
  renderCounter();
  renderTemplates();
}

async function change(action) {
  try {
    const response = await api.apply(action);
    receive(response.data);
    return true;
  } catch {
    notify('Не удалось сохранить изменения', true);
    return false;
  }
}

async function editNote(field, value) {
  const id = selectedNoteId;
  if (!id) return;
  const drafts = noteDrafts.get(id) ?? new Map();
  const draft = { value, failed: false };
  drafts.set(field, draft);
  noteDrafts.set(id, drafts);
  renderNotes();

  const saved = await change({ type: 'note:edit', id, field, value });
  if (drafts.get(field) === draft) {
    if (saved) drafts.delete(field);
    else draft.failed = true;
  }
  if (!drafts.size) noteDrafts.delete(id);
  renderNotes();
}

async function addNote() {
  try {
    const response = await api.apply({ type: 'note:add' });
    selectedNoteId = response.result;
    receive(response.data);
    renderNotes();
    if (activePanel === 'notes') noteTitle.focus();
  } catch {
    notify('Не удалось создать заметку', true);
  }
}

function openTemplateEditor(template) {
  editingTemplate = template?.id;
  get('template-heading').textContent = template ? 'Редактировать шаблон' : 'Новый шаблон';
  templateTitle.value = template?.title ?? '';
  templateText.value = template?.text ?? '';
  templateTitle.setCustomValidity('');
  templateText.setCustomValidity('');
  templateEditor.showModal();
}

function askDelete(type, entry) {
  deleting = { type: type + ':delete', id: entry.id };
  get('delete-heading').textContent = type === 'note' ? 'Удалить заметку?' : 'Удалить шаблон?';
  get('delete-label').textContent = '«' + noteName(entry) + '»';
  deleteDialog.showModal();
}

function setDialogBusy(dialog, busy) {
  dialog.querySelector('button[type="submit"]').disabled = busy;
  for (const button of dialog.querySelectorAll('[data-close]')) button.disabled = busy;
}

function renderPin(pinned, keysReady) {
  pin.setAttribute('aria-pressed', String(pinned));
  pin.title = pinned ? 'Открепить окно' : 'Закрепить поверх других окон';
  keysAccess.hidden = !isMac || !pinned || keysReady;
}

tabs.forEach((tab, index) => {
  tab.addEventListener('click', () => selectPanel(tab.id.slice(4)));
  tab.addEventListener('keydown', (event) => {
    const last = tabs.length - 1;
    const target = {
      ArrowLeft: index === 0 ? last : index - 1,
      ArrowRight: index === last ? 0 : index + 1,
      Home: 0,
      End: last,
    }[event.key];
    if (target === undefined) return;
    event.preventDefault();
    tabs[target].click();
    tabs[target].focus();
  });
});

pin.addEventListener('click', async () => {
  pin.disabled = true;
  try {
    const status = await api.pin(pin.getAttribute('aria-pressed') !== 'true');
    renderPin(status.pinned, status.keysReady);
    if (status.pinned && !status.keysReady && !isMac) {
      notify('Стрелки в других приложениях недоступны', true);
    }
  } catch {
    notify('Не удалось изменить закрепление', true);
  } finally {
    pin.disabled = false;
  }
});

keysAccess.addEventListener('click', async () => {
  keysAccess.disabled = true;
  try {
    const ready = await api.authorizeKeys();
    renderPin(pin.getAttribute('aria-pressed') === 'true', ready);
    if (!ready) notify('Разрешите AnyNotes мониторинг ввода в настройках macOS, затем перезапустите приложение.');
  } catch {
    notify('Не удалось проверить разрешение', true);
  } finally {
    keysAccess.disabled = false;
  }
});
window.addEventListener('focus', async () => {
  if (keysAccess.hidden) return;
  try {
    const status = await api.session();
    renderPin(status.pinned, status.keysReady);
  } catch {
    // Keep the authorization button available for a retry.
  }
});

add.addEventListener('click', () => {
  if (activePanel === 'notes') addNote();
  if (activePanel === 'templates') openTemplateEditor();
});
noteList.addEventListener('click', (event) => {
  const item = event.target.closest('.note-item');
  if (!item) return;
  selectedNoteId = item.dataset.id;
  renderNotes();
});
notePicker.addEventListener('change', () => {
  selectedNoteId = notePicker.value;
  renderNotes();
});
noteTitle.addEventListener('input', () => editNote('title', noteTitle.value));
noteText.addEventListener('input', () => editNote('text', noteText.value));
palette.addEventListener('click', (event) => {
  const color = event.target.dataset.color;
  if (color && color !== paper.dataset.color) editNote('color', color);
});
get('note-delete').addEventListener('click', () => {
  const note = data.notes.find((entry) => entry.id === selectedNoteId);
  if (note) askDelete('note', noteWithDrafts(note));
});

get('decrement').addEventListener('click', () => change({ type: 'counter:step', step: -1 }));
get('increment').addEventListener('click', () => change({ type: 'counter:step', step: 1 }));
get('reset').addEventListener('click', () => change({ type: 'counter:reset' }));

document.addEventListener('keydown', (event) => {
  if ((activePanel !== 'counter' && pin.getAttribute('aria-pressed') !== 'true')
    || event.defaultPrevented || event.isComposing
    || event.ctrlKey || event.metaKey || event.altKey || event.shiftKey
    || event.target?.closest?.('input, textarea, select, [contenteditable], dialog, .history-list, .drag-handle')) return;
  if (event.key === 'ArrowUp') get('increment').click();
  if (event.key === 'ArrowDown') get('decrement').click();
});

templateList.addEventListener('click', async (event) => {
  const button = event.target.closest('button[data-action]');
  if (!button) return;
  const id = button.closest('article').dataset.id;
  const template = data.templates.find((entry) => entry.id === id);
  if (!template) return;
  switch (button.dataset.action) {
    case 'copy':
      try {
        await api.copy(id);
        notify('Скопировано');
      } catch {
        notify('Не удалось скопировать шаблон', true);
      }
      break;
    case 'edit':
      openTemplateEditor(template);
      break;
    case 'delete':
      askDelete('template', template);
      break;
  }
});

for (const dialog of [templateEditor, deleteDialog]) {
  for (const button of dialog.querySelectorAll('[data-close]')) {
    button.addEventListener('click', () => dialog.close());
  }
  dialog.addEventListener('cancel', (event) => {
    if (dialog.querySelector('button[type="submit"]').disabled) event.preventDefault();
  });
}
for (const field of [templateTitle, templateText]) {
  field.addEventListener('input', () => field.setCustomValidity(''));
}
get('template-form').addEventListener('submit', async (event) => {
  event.preventDefault();
  if (!templateTitle.value.trim()) templateTitle.setCustomValidity('Введите название шаблона.');
  if (!templateText.value.trim()) templateText.setCustomValidity('Введите текст для копирования.');
  if (!event.currentTarget.reportValidity()) return;
  setDialogBusy(templateEditor, true);
  const saved = await change({
    type: 'template:save', id: editingTemplate, title: templateTitle.value, text: templateText.value,
  });
  setDialogBusy(templateEditor, false);
  if (saved) {
    templateEditor.close();
    notify('Шаблон сохранён');
  }
});
get('delete-form').addEventListener('submit', async (event) => {
  event.preventDefault();
  setDialogBusy(deleteDialog, true);
  const saved = await change(deleting);
  setDialogBusy(deleteDialog, false);
  if (saved) {
    if (deleting.type === 'note:delete') noteDrafts.delete(deleting.id);
    deleteDialog.close();
  }
});

async function start() {
  try {
    await window.__TAURI__.event.listen('counter-changed', ({ payload }) => receive(payload));
    const session = await api.session();
    isMac = session.platform === 'macos';
    renderPin(session.pinned, session.keysReady);
    receive(session.data);
    selectPanel('notes');
    get('app').hidden = false;
    pin.disabled = false;
  } catch {
    notify('Не удалось загрузить AnyNotes', true);
  }
}

start();
