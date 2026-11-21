# AnyNotes

[Русский](#русский) | [English](#english)

## Русский

**Заметки поверх окон, счётчик и шаблоны ответов для Mac и Windows.**

Отвечаете клиентам? Запишите номер заявки, посчитайте, сколько обращений закрыли, и вставьте типовой ответ одним нажатием. Закрепите AnyNotes поверх браузера или рабочего приложения, чтобы всё это было перед глазами.

**[Скачать AnyNotes для Mac и Windows](https://github.com/odin337/anynotes/releases/latest)**

- **Заметки.** Держите перед глазами номер заявки, список дел или подсказку. Текст сохраняется автоматически.
- **Счётчик.** Считайте заявки, звонки или любые повторяющиеся действия. После сброса число попадает в историю. В ней хранятся последние десять значений.
- **Шаблоны.** Дайте текстам понятные названия. Копируйте их в один клик и меняйте порядок перетаскиванием.

<img src="assets/anynotes-preview.png" alt="AnyNotes: шаблоны ответов для поддержки в компактном окне с вкладками «Заметки» и «Счётчик»" width="420">

### Установка

На Mac с Apple Silicon (macOS 13.3 и новее) откройте DMG и перетащите AnyNotes в «Программы». На Windows x64 запустите `.exe`. При обновлении можно установить новую версию поверх старой или сначала удалить файлы старой версии. Заметки и шаблоны сохраняются в обоих случаях. Если Windows понадобится WebView2, установщик скачает его.

При первом запуске Mac может попросить подтвердить открытие. [Как открыть скачанное приложение на Mac](https://support.apple.com/en-us/102445)

### Частые вопросы

**Как закрепить заметки поверх всех окон?** Нажмите булавку. Окно можно уменьшить до размера стикера, а на Mac оно остаётся на виду и в полноэкранном режиме.

**Как считать обращения, пока я работаю в браузере?** Закрепите AnyNotes. ↑ и → увеличивают число, ↓ и ← уменьшают. Браузер продолжает получать все нажатия. На Mac нажмите кнопку разрешения мониторинга ввода, если она появится.

**Как скопировать шаблон, не переключаясь на AnyNotes?** Нажмите кнопку «Копировать» в строке шаблона. На Mac копирование срабатывает с первого клика, даже если активно другое приложение.

**Где хранятся данные?** Заметки, тексты шаблонов и история счётчика остаются на вашем компьютере.

### Поддержка

Если AnyNotes пригодился, поставьте звезду на GitHub. Нашли ошибку или хотите предложить улучшение? [Напишите в Issues](https://github.com/odin337/anynotes/issues).

## English

**Always-on-top sticky notes, a desktop tally counter, and canned responses for Mac and Windows.**

Work in customer support? Jot down a ticket number, count the cases you've closed, and copy a reply you use all the time. Pin AnyNotes above your browser or help desk to keep it within reach while you work.

**[Download AnyNotes for Mac and Windows](https://github.com/odin337/anynotes/releases/latest)**

- **Notes.** Write down the things you need to remember. Your notes save as you type.
- **Counter.** Count tickets, calls, or anything you do repeatedly. Reset it to start from zero and keep the last ten totals in history.
- **Templates.** Name your saved replies, copy them with one click, and drag them to change the order.

### Installation

On an Apple Silicon Mac (macOS 13.3 or later), open the DMG and drag AnyNotes to Applications. On Windows x64, run the `.exe` installer. You can upgrade in place or remove the old program files first. Your notes and templates stay in either case. The installer downloads WebView2 if needed. The app interface is currently in Russian.

macOS may ask you to confirm the first launch. [How to open a downloaded Mac app](https://support.apple.com/en-us/102445)

### Common questions

**How do I keep sticky notes above other windows?** Click the pin and resize the window to fit the space you have. On Mac, the pinned window also stays above full-screen apps.

**Can I tally support tickets while I work in my browser?** Pin AnyNotes. Press ↑ or → to add one and ↓ or ← to subtract one. The browser still gets the keypresses. On Mac, click the Input Monitoring permission button if it appears.

**How do I copy a template while working in another app?** Click Copy next to the template. On Mac, one click copies the text even if another app has focus.

**Where is my data stored?** Your notes, text templates, and counter history stay on your computer.

### Support

If you use AnyNotes, a GitHub star helps others find it. Found a bug or have an idea? [Open an issue](https://github.com/odin337/anynotes/issues).

## Сборка из исходников / Build from source

Установите Node.js, Rust и [зависимости Tauri](https://v2.tauri.app/start/prerequisites/) для вашей системы.

Install Node.js, Rust, and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform.

```sh
npm ci
npm start
```

`npm run package:mac` создаёт DMG на Mac, `npm run package:win` собирает установщик Windows x64. Результаты находятся в `dist`.

Run `npm run package:mac` on a Mac for the DMG, or `npm run package:win` for the Windows x64 installer. Find both in `dist`.
