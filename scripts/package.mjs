import { execFileSync } from 'node:child_process';
import { copyFile, mkdir, readFile, readdir } from 'node:fs/promises';
import { delimiter, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const out = join(root, 'dist');
const { version } = JSON.parse(await readFile(join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'));
const [target, requestedArch] = process.argv.slice(2);
const cli = join(root, 'node_modules', '@tauri-apps', 'cli', 'tauri.js');
await mkdir(out, { recursive: true });

function build(triple, bundle, extra = [], env = process.env) {
  execFileSync(process.execPath, [
    cli, 'build', '--target', triple, '--bundles', bundle, '--ci', ...extra, '--', '--locked',
  ], { cwd: root, stdio: 'inherit', env });
  return join(root, 'src-tauri', 'target', triple, 'release', 'bundle', bundle === 'app' ? 'macos' : bundle);
}

if (target === 'mac' && process.platform === 'darwin') {
  const arch = requestedArch ?? process.arch;
  const triple = { arm64: 'aarch64-apple-darwin', x64: 'x86_64-apple-darwin' }[arch];
  if (!triple) throw new Error('Expected macOS arm64 or x64.');
  const bundle = build(triple, 'app');
  const image = join(out, `AnyNotes-${version}-macOS-${arch}.dmg`);
  execFileSync('uvx', [
    '--from', 'dmgbuild==2.0.0', 'dmgbuild',
    '--settings', join(root, 'scripts', 'dmg-settings.py'),
    '-D', `app=${join(bundle, 'AnyNotes.app')}`,
    '-D', `art=${join(root, 'assets', 'dmg-background.png')}`,
    'AnyNotes', image,
  ], { stdio: 'inherit' });
  console.log(image);
} else if (target === 'win') {
  const arch = requestedArch ?? 'x64';
  const triple = { x64: 'x86_64-pc-windows-msvc', arm64: 'aarch64-pc-windows-msvc' }[arch];
  if (!triple) throw new Error('Expected Windows x64 or arm64.');
  const cross = process.platform !== 'win32';
  const path = process.platform === 'darwin'
    ? ['/opt/homebrew/opt/llvm/bin', '/opt/homebrew/opt/lld/bin', '/usr/local/opt/llvm/bin', '/usr/local/opt/lld/bin', process.env.PATH].join(delimiter)
    : process.env.PATH;
  const bundle = build(triple, 'nsis', cross ? ['--runner', 'cargo-xwin'] : [], { ...process.env, PATH: path });
  const installer = (await readdir(bundle)).find((name) => name.endsWith('-setup.exe') && name.includes(version));
  if (!installer) throw new Error('Windows installer was not generated.');
  const output = join(out, `AnyNotes-${version}-Windows-${arch}-Setup.exe`);
  await copyFile(join(bundle, installer), output);
  console.log(output);
} else {
  throw new Error('Use package:mac on macOS or package:win with the Windows build tools.');
}
