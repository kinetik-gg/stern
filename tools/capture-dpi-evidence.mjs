import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { arch, platform, tmpdir } from 'node:os';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const SCALES = [
  { value: 1, label: '1.00x', width: 640, height: 360 },
  { value: 1.25, label: '1.25x', width: 800, height: 450 },
  { value: 1.5, label: '1.50x', width: 960, height: 540 },
  { value: 2, label: '2.00x', width: 1280, height: 720 },
];
const TOKENS = {
  'surface.application': '#111111', 'surface.panel_raised': '#181818',
  'surface.control': '#181818', 'surface.sunken': '#0b0b0b',
  'surface.control_hover': '#1c1c1c', 'surface.control_pressed': '#2a2a2a',
  'border.subtle': '#222222', 'border.default': '#2a2a2a',
  'border.strong': '#3d3d3d', 'accent.subtle': '#0b2a3f',
  'accent.default': '#0c8ce9', 'accent.focus': '#4db2ff',
};
const HTML = `<!doctype html><html><head><meta charset="utf-8"><style>
html,body{margin:0;width:640px;height:360px;overflow:hidden;zoom:1;background:${TOKENS['surface.application']}}
svg{display:block;width:640px;height:360px;overflow:hidden}
</style></head><body><svg xmlns="http://www.w3.org/2000/svg" width="640" height="360" viewBox="0 0 640 360">
<rect width="640" height="360" fill="${TOKENS['surface.application']}"/>
<rect x="40" y="32" width="560" height="296" fill="${TOKENS['surface.panel_raised']}" stroke="${TOKENS['border.default']}" stroke-width="1"/>
<rect x="72" y="76" width="96" height="64" fill="${TOKENS['surface.control']}" stroke="${TOKENS['border.strong']}" stroke-width="1"/>
<rect x="168" y="76" width="96" height="64" fill="${TOKENS['accent.default']}" stroke="${TOKENS['border.strong']}" stroke-width="1"/>
<rect x="296" y="76" width="272" height="64" fill="${TOKENS['surface.sunken']}" stroke="${TOKENS['border.subtle']}" stroke-width="1"/>
<line x1="72" y1="180" x2="568" y2="180" stroke="${TOKENS['border.default']}" stroke-width="1"/>
<line x1="72" y1="220" x2="568" y2="220" stroke="${TOKENS['accent.focus']}" stroke-width="2"/>
<rect x="72" y="260" width="160" height="36" fill="${TOKENS['surface.control_hover']}"/>
<rect x="232" y="260" width="168" height="36" fill="${TOKENS['surface.control_pressed']}"/>
<rect x="400" y="260" width="168" height="36" fill="${TOKENS['accent.subtle']}"/>
</svg></body></html>`;

class Cdp {
  constructor(url) { this.url = url; this.next = 1; this.pending = new Map(); }
  async open() {
    this.socket = new WebSocket(this.url);
    await new Promise((ok, fail) => {
      this.socket.addEventListener('open', ok, { once: true });
      this.socket.addEventListener('error', fail, { once: true });
    });
    this.socket.addEventListener('message', event => {
      const message = JSON.parse(String(event.data));
      if (!message.id) return;
      const pending = this.pending.get(message.id);
      if (!pending) return;
      this.pending.delete(message.id);
      message.error ? pending.reject(new Error(JSON.stringify(message.error))) : pending.resolve(message.result);
    });
  }
  call(method, params = {}) {
    const id = this.next++;
    return new Promise((resolveCall, reject) => {
      this.pending.set(id, { resolve: resolveCall, reject });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }
  close() { this.socket?.close(); }
}

function args() {
  const [command, ...rest] = process.argv.slice(2);
  const options = { command };
  for (let index = 0; index < rest.length; index += 2) {
    if (!rest[index].startsWith('--') || rest[index + 1] === undefined) throw new Error(`invalid argument ${rest[index]}`);
    options[rest[index].slice(2)] = rest[index + 1];
  }
  if (!['capture', 'verify'].includes(command) || !options.output) throw new Error('usage: capture|verify --output <dir> [--chrome <exe>] [--require-review pending_human|approved]');
  return options;
}

async function launchChrome(executable) {
  const profile = mkdtempSync(join(tmpdir(), 'stern-dpi-009-'));
  const command = ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--hide-scrollbars', 'about:blank'];
  const process = spawn(executable, command, { stdio: ['ignore', 'pipe', 'pipe'] });
  let output = '';
  const endpoint = await new Promise((resolveEndpoint, reject) => {
    const timer = setTimeout(() => reject(new Error(`Chrome CDP timeout: ${output}`)), 20000);
    const inspect = data => {
      output += String(data);
      const match = output.match(/DevTools listening on (ws:\/\/[^\s]+)/);
      if (match) { clearTimeout(timer); resolveEndpoint(match[1]); }
    };
    process.stdout.on('data', inspect); process.stderr.on('data', inspect);
    process.once('exit', code => reject(new Error(`Chrome exited before CDP (${code}): ${output}`)));
  });
  return { process, profile, endpoint, command };
}

async function shutdownChrome(chrome, browser) {
  try {
    await withTimeout(browser.call('Browser.close'), 3000, 'Chrome graceful close');
  } catch {
    chrome.process.kill();
  }
  try {
    await waitForExit(chrome.process, 5000);
  } catch (error) {
    if (!chrome.process.kill() && chrome.process.exitCode === null && chrome.process.signalCode === null) throw error;
    await waitForExit(chrome.process, 5000);
  } finally {
    browser.close();
  }
  rmSync(chrome.profile, { recursive: true, force: false });
}

function waitForExit(process, timeout) {
  if (process.exitCode !== null || process.signalCode !== null) return Promise.resolve();
  return withTimeout(new Promise(resolveExit => process.once('exit', resolveExit)), timeout, 'Chrome process exit');
}

function withTimeout(promise, timeout, label) {
  return new Promise((resolvePromise, reject) => {
    const timer = setTimeout(() => reject(new Error(`${label} timed out`)), timeout);
    promise.then(
      value => { clearTimeout(timer); resolvePromise(value); },
      error => { clearTimeout(timer); reject(error); },
    );
  });
}

async function captureBrowser(output, executable) {
  const chrome = await launchChrome(executable);
  const endpoint = new URL(chrome.endpoint);
  const browser = new Cdp(chrome.endpoint);
  await browser.open();
  const version = await browser.call('Browser.getVersion');
  const captures = [];
  try {
    for (const scale of SCALES) {
      const target = await fetch(`http://${endpoint.host}/json/new?about:blank`, { method: 'PUT' }).then(response => response.json());
      const cdp = new Cdp(target.webSocketDebuggerUrl);
      await cdp.open();
      try {
        await cdp.call('Page.enable');
        await cdp.call('Emulation.setDeviceMetricsOverride', {
          width: 640, height: 360, mobile: false, scale: 1, deviceScaleFactor: scale.value,
        });
        await cdp.call('Page.navigate', { url: `data:text/html;base64,${Buffer.from(HTML).toString('base64')}` });
        let observed;
        for (let attempt = 0; attempt < 100; attempt++) {
          observed = await cdp.call('Runtime.evaluate', { returnByValue: true, expression: `(() => ({ready:document.readyState,dpr:devicePixelRatio,innerWidth,innerHeight,rootZoom:getComputedStyle(document.documentElement).zoom,bodyZoom:getComputedStyle(document.body).zoom,scrollWidth:document.documentElement.scrollWidth,scrollHeight:document.documentElement.scrollHeight,bodyScrollWidth:document.body.scrollWidth,bodyScrollHeight:document.body.scrollHeight}))()` });
          if (observed.result.value.ready === 'complete') break;
          await new Promise(done => setTimeout(done, 20));
        }
        observed = observed.result.value;
        assert(observed.ready === 'complete', 'document not ready');
        assert(observed.dpr === scale.value, `wrong DPR at ${scale.label}`);
        assert(observed.innerWidth === 640 && observed.innerHeight === 360, `wrong viewport at ${scale.label}`);
        assert(observed.rootZoom === '1' && observed.bodyZoom === '1', `wrong zoom at ${scale.label}`);
        assert(observed.scrollWidth === 640 && observed.scrollHeight === 360 && observed.bodyScrollWidth === 640 && observed.bodyScrollHeight === 360, `overflow at ${scale.label}`);
        const shot = await cdp.call('Page.captureScreenshot', { format: 'png', fromSurface: true, captureBeyondViewport: false });
        const path = join(output, 'browser', `${scale.label}.png`);
        mkdirSync(dirname(path), { recursive: true });
        writeFileSync(path, Buffer.from(shot.data, 'base64'));
        const dimensions = pngDimensions(readFileSync(path));
        assert(dimensions.width === scale.width && dimensions.height === scale.height, `wrong browser PNG dimensions at ${scale.label}`);
        captures.push({ scale: scale.value, label: scale.label, cdp_parameters: { width: 640, height: 360, mobile: false, scale: 1, deviceScaleFactor: scale.value }, observed });
      } finally { cdp.close(); }
    }
  } finally {
    await shutdownChrome(chrome, browser);
  }
  return { product: version.product, protocol_version: version.protocolVersion, revision: version.revision, user_agent: version.userAgent, executable, command: [executable, ...chrome.command], captures };
}

function captureVello(output) {
  const target = join(output, 'vello');
  mkdirSync(target, { recursive: true });
  const command = ['+1.92.0', 'run', '-p', 'stern-vello', '--example', 'capture_dpi_evidence', '--all-features', '--', '--output', target];
  const result = spawnSync('cargo', command, { cwd: ROOT, env: { ...process.env, WGPU_BACKEND: 'dx12' }, encoding: 'utf8', windowsHide: true });
  if (result.status !== 0) throw new Error(`Vello capture failed (${result.status})\n${result.stdout}\n${result.stderr}`);
  const line = result.stdout.split(/\r?\n/).find(value => value.startsWith('STERN_VELLO_METADATA='));
  if (!line) throw new Error(`missing Vello metadata\n${result.stdout}`);
  const metadata = JSON.parse(line.slice('STERN_VELLO_METADATA='.length));
  assert(metadata.backend === 'Dx12', `wrong Vello backend ${metadata.backend}`);
  return { ...metadata, command: ['WGPU_BACKEND=dx12', 'cargo', ...command] };
}

function git(...arguments_) {
  const result = spawnSync('git', arguments_, { cwd: ROOT, encoding: 'utf8', windowsHide: true });
  if (result.status !== 0) throw new Error(result.stderr);
  return result.stdout.trim();
}

function artifact(output, renderer, scale) {
  const path = join(output, renderer, `${scale.label}.png`);
  const bytes = readFileSync(path);
  const dimensions = pngDimensions(bytes);
  assert(dimensions.width === scale.width && dimensions.height === scale.height, `wrong ${renderer} dimensions at ${scale.label}`);
  assert(bytes.length <= 1024 * 1024, `${renderer}/${scale.label} exceeds 1 MiB`);
  return { renderer, scale: scale.value, path: relative(ROOT, path).split(sep).join('/'), mime: 'image/png', logical_size: [640, 360], physical_size: [dimensions.width, dimensions.height], byte_length: bytes.length, sha256: sha256(bytes), observations: 'deterministic capture checks passed; pending human visual review' };
}

async function capture(options) {
  if (!options.chrome) throw new Error('--chrome is required for capture');
  assert(git('status', '--porcelain') === '', 'capture source must be clean');
  const output = resolve(ROOT, options.output);
  const chrome = await captureBrowser(output, resolve(options.chrome));
  const vello = captureVello(output);
  const artifacts = SCALES.flatMap(scale => [artifact(output, 'browser', scale), artifact(output, 'vello', scale)]);
  assert(artifacts.reduce((sum, item) => sum + item.byte_length, 0) <= 4 * 1024 * 1024, 'PNG aggregate exceeds 4 MiB');
  const review = {
    schema_version: '1.0', requirement_id: 'STERN-DPI-009', issue: 653,
    stern_version: '1.0.0-rc.2.dev', specification_sha256: 'f1d489f6f28b613c0bcfa4490b7855da341457ee20c66c892dc37ebff2d024ed',
    source: { commit: git('rev-parse', 'HEAD'), tree: git('rev-parse', 'HEAD^{tree}') },
    capture_utc: new Date().toISOString(), host: { os: platform(), architecture: arch() },
    scene: { logical_size: [640, 360], tokens: TOKENS, named_geometry: { panel: [40, 32, 560, 296], shared_edge: { x: 168, y1: 76, y2: 140 }, control_swatch: [72, 76, 96, 64], accent_swatch: [168, 76, 96, 64], sunken_swatch: [296, 76, 272, 64] }, strokes: [{ name: 'hairline', logical_width: 1, y: 180 }, { name: 'emphasis', logical_width: 2, y: 220 }], expected_physical: SCALES.map(({ value, width, height }) => ({ scale: value, width, height, parity: Number.isInteger(width) && Number.isInteger(height) })), overflow_free: true },
    chrome, vello, artifacts,
    comparison: { policy: 'independent renderer review of named assertions', cross_renderer_pixel_equality: false, limitations: { browser: 'Chromium SVG anti-aliasing and rasterization are renderer-specific.', vello: 'Vello Area AA and GPU rasterization are renderer-specific.' } },
    review: { status: 'pending_human', disposition: 'partial', reviewer: null, reviewed_utc: null, approval_reference: null, artifact_verdicts: [], assertion_results: [], overall: null },
  };
  const reviewPath = join(output, 'review.json');
  writeFileSync(reviewPath, `${JSON.stringify(review, null, 2)}\n`);
  assert(readFileSync(reviewPath).length <= 64 * 1024, 'review.json exceeds 64 KiB');
}

function verify(options) {
  const output = resolve(ROOT, options.output);
  const reviewBytes = readFileSync(join(output, 'review.json'));
  assert(reviewBytes.length <= 64 * 1024, 'review.json exceeds 64 KiB');
  const review = JSON.parse(reviewBytes);
  assert(review.schema_version === '1.0' && review.requirement_id === 'STERN-DPI-009' && review.issue === 653, 'wrong review identity');
  assert(review.stern_version === '1.0.0-rc.2.dev' && review.specification_sha256 === 'f1d489f6f28b613c0bcfa4490b7855da341457ee20c66c892dc37ebff2d024ed', 'wrong pinned authority');
  assert(review.chrome.captures.length === 4 && review.vello.captures.length === 4 && review.artifacts.length === 8, 'wrong capture cardinality');
  assert(review.comparison.cross_renderer_pixel_equality === false && review.scene.overflow_free === true, 'wrong comparison/overflow contract');
  const aggregate = review.artifacts.reduce((sum, item) => {
    const bytes = readFileSync(resolve(ROOT, item.path));
    const dimensions = pngDimensions(bytes);
    assert(item.mime === 'image/png' && item.byte_length === bytes.length && item.sha256 === sha256(bytes), `artifact drift: ${item.path}`);
    assert(dimensions.width === item.physical_size[0] && dimensions.height === item.physical_size[1] && bytes.length <= 1024 * 1024, `artifact dimensions/budget: ${item.path}`);
    return sum + bytes.length;
  }, 0);
  assert(aggregate <= 4 * 1024 * 1024, 'PNG aggregate exceeds 4 MiB');
  const required = options['require-review'];
  if (required === 'pending_human') {
    assert(review.review.status === 'pending_human' && review.review.disposition === 'partial', 'review is not pending_human');
  } else if (required === 'approved') {
    assert(review.review.status === 'approved' && review.review.disposition === 'verified' && review.review.reviewer && review.review.reviewed_utc && review.review.approval_reference, 'review approval metadata incomplete');
    assert(review.review.artifact_verdicts.length === 8 && review.review.artifact_verdicts.every(item => item.verdict === 'PASS'), 'artifact verdicts incomplete');
    assert(review.review.assertion_results.length > 0 && review.review.assertion_results.every(item => item.result === 'PASS') && review.review.overall === 'PASS', 'assertion review incomplete');
  } else throw new Error('--require-review must be pending_human or approved');
  console.log(`verified ${review.artifacts.length} renderer artifacts (${aggregate} bytes), review=${required}`);
}

function pngDimensions(bytes) {
  assert(bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])), 'not a PNG');
  assert(bytes.toString('ascii', 12, 16) === 'IHDR', 'missing PNG IHDR');
  return { width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) };
}
function sha256(bytes) { return createHash('sha256').update(bytes).digest('hex'); }
function assert(condition, message) { if (!condition) throw new Error(message); }

const options = args();
if (options.command === 'capture') await capture(options); else verify(options);
