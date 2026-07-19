import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, statSync, unlinkSync, writeFileSync } from 'node:fs';
import { dirname, isAbsolute, join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { deflateSync } from 'node:zlib';

export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
export const LOGICAL_DIMENSIONS = [960, 640];
export const SCALES = [
  { value: 1, label: '1.00x' },
  { value: 1.25, label: '1.25x' },
  { value: 1.5, label: '1.50x' },
  { value: 2, label: '2.00x' },
];
export const WORKSPACES = ['edit', 'graph'];
export const REVIEW_CRITERIA = [
  'clipping',
  'overlay_placement',
  'hairlines',
  'text_baselines',
  'focus_selection',
  'private_override_absence',
];

const SOURCE_PATHS = [
  'Cargo.toml',
  'Cargo.lock',
  'apps/stern-demo',
  'crates/stern',
  'crates/stern-core',
  'crates/stern-render',
  'crates/stern-text',
  'crates/stern-vello',
  'crates/stern-vello-winit',
  'crates/stern-widgets',
  'crates/stern-winit',
  'tools/capture-demo-vello.mjs',
];

function parseArgs(argv) {
  const [command, ...rest] = argv;
  const options = { command };
  for (let index = 0; index < rest.length; index += 2) {
    const name = rest[index];
    const value = rest[index + 1];
    if (!name?.startsWith('--') || value === undefined) {
      throw new Error(`invalid argument ${name ?? '<missing>'}`);
    }
    options[name.slice(2)] = value;
  }
  if (!['capture', 'verify'].includes(command) || !options.output) {
    throw new Error('usage: capture|verify --output <directory> [--capture-status provisional|final] [--require-status provisional|final --require-review pending_human|approved]');
  }
  return options;
}

function git(...args) {
  const result = spawnSync('git', args, { cwd: ROOT, encoding: 'utf8', windowsHide: true });
  if (result.status !== 0) throw new Error(result.stderr || `git ${args.join(' ')} failed`);
  return result.stdout.trim();
}

export function expectedArtifacts() {
  return WORKSPACES.flatMap(workspace => SCALES.map(scale => ({
    workspace,
    scale: scale.value,
    scale_label: scale.label,
    logical_dimensions: [...LOGICAL_DIMENSIONS],
    physical_dimensions: LOGICAL_DIMENSIONS.map(value => value * scale.value),
    path: `${workspace}/${scale.label}.png`,
  })));
}

function runCapture(output) {
  const command = [
    'run', '-p', 'stern-demo', '--example', 'capture_vello_workspaces', '--all-features',
    '--', '--output', output,
  ];
  const result = spawnSync('cargo', command, {
    cwd: ROOT,
    env: { ...process.env, WGPU_BACKEND: 'dx12' },
    encoding: 'utf8',
    windowsHide: true,
  });
  if (result.status !== 0) {
    throw new Error(`Vello capture failed (${result.status})\n${result.stdout}\n${result.stderr}`);
  }
  const line = result.stdout
    .split(/\r?\n/)
    .find(value => value.startsWith('STERN_DEMO_VELLO_METADATA='));
  if (!line) throw new Error(`missing Vello capture metadata\n${result.stdout}`);
  return JSON.parse(line.slice('STERN_DEMO_VELLO_METADATA='.length));
}

function capture(options) {
  const status = options['capture-status'];
  assert(['provisional', 'final'].includes(status), '--capture-status must be provisional or final');
  assert(git('status', '--porcelain') === '', 'capture source must be clean');
  const output = resolve(ROOT, options.output);
  assert(!existsSync(output), `output already exists: ${output}`);
  mkdirSync(output, { recursive: true });
  const metadata = runCapture(output);
  assert(metadata.renderer === 'Vello', `unexpected renderer ${metadata.renderer}`);
  assert(metadata.backend === 'Dx12', `unexpected backend ${metadata.backend}`);

  const expected = expectedArtifacts();
  const artifacts = expected.map(coordinate => {
    const rawPath = join(output, coordinate.path.replace(/\.png$/, '.rgba'));
    const rgba = readFileSync(rawPath);
    const expectedBytes = coordinate.physical_dimensions[0] * coordinate.physical_dimensions[1] * 4;
    assert(rgba.length === expectedBytes, `raw RGBA dimensions mismatch for ${coordinate.path}`);
    const path = join(output, coordinate.path);
    writeFileSync(path, encodePng(...coordinate.physical_dimensions, rgba));
    unlinkSync(rawPath);
    const bytes = readFileSync(path);
    const dimensions = pngDimensions(bytes);
    assert(samePair([dimensions.width, dimensions.height], coordinate.physical_dimensions), `PNG dimensions mismatch for ${coordinate.path}`);
    return {
      ...coordinate,
      renderer: metadata.renderer,
      backend: metadata.backend,
      byte_length: bytes.length,
      sha256: sha256(bytes),
    };
  });

  const manifest = {
    schema_version: '1.0',
    issue: 845,
    capture_status: status,
    source: {
      commit: git('rev-parse', 'HEAD'),
      tree: git('rev-parse', 'HEAD^{tree}'),
      guarded_paths: SOURCE_PATHS,
    },
    viewport: { logical_dimensions: LOGICAL_DIMENSIONS },
    renderer: {
      name: metadata.renderer,
      backend: metadata.backend,
      adapter: metadata.adapter,
      vendor: metadata.vendor,
      device: metadata.device,
      driver: metadata.driver,
      driver_info: metadata.driver_info,
      device_type: metadata.device_type,
      texture_format: metadata.texture_format,
      antialiasing: metadata.antialiasing,
    },
    artifacts,
    claims: {
      public_demo_app: true,
      private_stern_crates: false,
      alternate_scene: false,
      browser_capture: false,
      cross_scale_pixel_equality: false,
    },
    review: {
      status: 'pending_human',
      reviewer: null,
      reviewed_utc: null,
      approval_reference: null,
      artifact_verdicts: [],
      criteria: REVIEW_CRITERIA.map(criterion => ({ criterion, result: 'PENDING', notes: null })),
      overall: null,
    },
  };
  writeFileSync(join(output, 'manifest.json'), `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(`captured ${artifacts.length} public DemoApp Vello artifacts (${status})`);
}

export function verifyEvidence(
  outputPath,
  requiredStatus,
  { checkSourceInputs = true, requiredReview = 'pending_human' } = {},
) {
  assert(['provisional', 'final'].includes(requiredStatus), '--require-status must be provisional or final');
  assert(['pending_human', 'approved'].includes(requiredReview), '--require-review must be pending_human or approved');
  const output = resolve(ROOT, outputPath);
  const manifest = JSON.parse(readFileSync(join(output, 'manifest.json')));
  assert(manifest.schema_version === '1.0' && manifest.issue === 845, 'wrong manifest identity');
  assert(manifest.capture_status === requiredStatus, `capture status is not ${requiredStatus}`);
  assert(samePair(manifest.viewport?.logical_dimensions, LOGICAL_DIMENSIONS), 'wrong logical viewport');
  assert(manifest.renderer?.name === 'Vello' && typeof manifest.renderer.backend === 'string', 'wrong renderer metadata');
  assert(manifest.claims?.public_demo_app === true, 'public DemoApp claim is missing');
  assert(manifest.claims?.private_stern_crates === false && manifest.claims?.alternate_scene === false, 'private or alternate scene claim detected');
  assert(manifest.claims?.browser_capture === false && manifest.claims?.cross_scale_pixel_equality === false, 'browser or pixel-equality claim detected');
  verifySource(manifest.source, checkSourceInputs);

  const expected = expectedArtifacts();
  assert(Array.isArray(manifest.artifacts) && manifest.artifacts.length === expected.length, 'wrong artifact cardinality');
  const seen = new Set();
  for (const coordinate of expected) {
    const item = manifest.artifacts.find(candidate =>
      candidate.workspace === coordinate.workspace
      && candidate.scale === coordinate.scale
      && candidate.scale_label === coordinate.scale_label
    );
    assert(item, `missing artifact coordinate ${coordinate.workspace}/${coordinate.scale_label}`);
    const key = `${item.workspace}/${item.scale_label}`;
    assert(!seen.has(key), `duplicate artifact coordinate ${key}`);
    seen.add(key);
    assert(item.path === coordinate.path, `unexpected artifact path for ${key}`);
    assert(samePair(item.logical_dimensions, coordinate.logical_dimensions), `logical dimensions mismatch for ${key}`);
    assert(samePair(item.physical_dimensions, coordinate.physical_dimensions), `physical dimensions mismatch for ${key}`);
    assert(item.renderer === manifest.renderer.name && item.backend === manifest.renderer.backend, `backend mismatch for ${key}`);
    const path = containedPath(output, item.path);
    assert(existsSync(path) && statSync(path).isFile(), `missing artifact file ${item.path}`);
    const bytes = readFileSync(path);
    const dimensions = pngDimensions(bytes);
    assert(samePair([dimensions.width, dimensions.height], coordinate.physical_dimensions), `PNG dimensions mismatch for ${key}`);
    assert(item.byte_length === bytes.length && item.sha256 === sha256(bytes), `artifact hash mismatch for ${key}`);
  }

  const criteria = manifest.review?.criteria;
  assert(Array.isArray(criteria) && criteria.length === REVIEW_CRITERIA.length, 'review criteria cardinality mismatch');
  assert(REVIEW_CRITERIA.every(name => criteria.some(item => item.criterion === name)), 'review criteria are incomplete');
  if (requiredReview === 'pending_human') {
    assert(manifest.review.status === 'pending_human', 'evidence is not pending human review');
    assert(criteria.every(item => item.result === 'PENDING'), 'pending review contains a non-pending result');
    assert(manifest.review.artifact_verdicts.length === 0 && manifest.review.overall === null, 'pending review contains a final verdict');
  } else {
    assert(manifest.review.status === 'approved', 'final evidence is not human-approved');
    assert(manifest.review.reviewer && manifest.review.reviewed_utc && manifest.review.approval_reference, 'final review metadata is incomplete');
    assert(criteria.every(item => item.result === 'PASS' && typeof item.notes === 'string' && item.notes.length > 0), 'final review criteria are incomplete');
    assert(manifest.review.artifact_verdicts.length === 8 && manifest.review.artifact_verdicts.every(item => item.verdict === 'PASS'), 'final artifact verdicts are incomplete');
    assert(manifest.review.overall === 'PASS', 'final review overall is not PASS');
  }
  return manifest;
}

function verifySource(source, checkSourceInputs) {
  assert(source && typeof source.commit === 'string' && typeof source.tree === 'string', 'source provenance is incomplete');
  const observedTree = git('rev-parse', `${source.commit}^{tree}`);
  assert(observedTree === source.tree, 'source tree mismatch');
  assert(Array.isArray(source.guarded_paths) && source.guarded_paths.join('\n') === SOURCE_PATHS.join('\n'), 'source guard paths mismatch');
  if (!checkSourceInputs) return;
  const result = spawnSync('git', ['diff', '--quiet', source.commit, '--', ...SOURCE_PATHS], {
    cwd: ROOT,
    encoding: 'utf8',
    windowsHide: true,
  });
  if (result.status === 1) throw new Error('stale captures: guarded source changed after capture');
  if (result.status !== 0) throw new Error(result.stderr || 'source staleness check failed');
}

function containedPath(output, artifactPath) {
  assert(typeof artifactPath === 'string' && !isAbsolute(artifactPath), `invalid artifact path ${artifactPath}`);
  const path = resolve(output, artifactPath);
  assert(path.startsWith(`${output}${sep}`), `artifact escapes output directory: ${artifactPath}`);
  return path;
}

export function pngDimensions(bytes) {
  assert(bytes.length >= 24, 'truncated PNG');
  assert(bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])), 'not a PNG');
  assert(bytes.toString('ascii', 12, 16) === 'IHDR', 'missing PNG IHDR');
  return { width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) };
}

export function encodePng(width, height, rgba) {
  assert(Number.isInteger(width) && width > 0 && Number.isInteger(height) && height > 0, 'invalid PNG dimensions');
  assert(rgba.length === width * height * 4, 'RGBA byte length does not match dimensions');
  const rowBytes = width * 4;
  const scanlines = Buffer.alloc((rowBytes + 1) * height);
  for (let row = 0; row < height; row++) {
    const outputOffset = row * (rowBytes + 1);
    scanlines[outputOffset] = 0;
    rgba.copy(scanlines, outputOffset + 1, row * rowBytes, (row + 1) * rowBytes);
  }
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header[8] = 8;
  header[9] = 6;
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  return Buffer.concat([
    signature,
    pngChunk('IHDR', header),
    pngChunk('IDAT', deflateSync(scanlines, { level: 9 })),
    pngChunk('IEND', Buffer.alloc(0)),
  ]);
}

function pngChunk(type, data) {
  const typeBytes = Buffer.from(type, 'ascii');
  const chunk = Buffer.alloc(12 + data.length);
  chunk.writeUInt32BE(data.length, 0);
  typeBytes.copy(chunk, 4);
  data.copy(chunk, 8);
  chunk.writeUInt32BE(crc32(Buffer.concat([typeBytes, data])), 8 + data.length);
  return chunk;
}

function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit++) {
      crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0);
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

export function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function samePair(actual, expected) {
  return Array.isArray(actual) && actual.length === 2 && actual[0] === expected[0] && actual[1] === expected[1];
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function verify(options) {
  const required = options['require-status'];
  const requiredReview = options['require-review'];
  const manifest = verifyEvidence(options.output, required, { requiredReview });
  const bytes = manifest.artifacts.reduce((sum, item) => sum + item.byte_length, 0);
  console.log(`verified ${manifest.artifacts.length} public DemoApp Vello artifacts (${bytes} bytes, capture=${required}, review=${requiredReview})`);
}

const isMain = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  const options = parseArgs(process.argv.slice(2));
  if (options.command === 'capture') capture(options);
  else verify(options);
}
