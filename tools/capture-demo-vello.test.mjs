import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, unlinkSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

import {
  LOGICAL_DIMENSIONS,
  REVIEW_CRITERIA,
  ROOT,
  encodePng,
  expectedArtifacts,
  sha256,
  verifyEvidence,
} from './capture-demo-vello.mjs';

function git(...args) {
  return execFileSync('git', args, { cwd: ROOT, encoding: 'utf8' }).trim();
}

function fakePng(width, height) {
  return encodePng(width, height, Buffer.alloc(width * height * 4));
}

function fixture() {
  mkdirSync(join(ROOT, 'target'), { recursive: true });
  const output = mkdtempSync(join(ROOT, 'target', 'issue-845-verifier-'));
  const artifacts = expectedArtifacts().map(coordinate => {
    const path = join(output, coordinate.path);
    mkdirSync(join(path, '..'), { recursive: true });
    const bytes = fakePng(...coordinate.physical_dimensions);
    writeFileSync(path, bytes);
    return {
      ...coordinate,
      renderer: 'Vello',
      backend: 'Dx12',
      byte_length: bytes.length,
      sha256: sha256(bytes),
    };
  });
  const manifest = {
    schema_version: '1.0',
    issue: 845,
    capture_status: 'provisional',
    source: {
      commit: git('rev-parse', 'HEAD'),
      tree: git('rev-parse', 'HEAD^{tree}'),
      guarded_paths: [
        'Cargo.toml', 'Cargo.lock', 'apps/stern-demo', 'crates/stern', 'crates/stern-core',
        'crates/stern-render', 'crates/stern-text', 'crates/stern-vello',
        'crates/stern-vello-winit', 'crates/stern-widgets', 'crates/stern-winit',
        'tools/capture-demo-vello.mjs',
      ],
    },
    viewport: { logical_dimensions: LOGICAL_DIMENSIONS },
    renderer: { name: 'Vello', backend: 'Dx12' },
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
  const manifestPath = join(output, 'manifest.json');
  writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
  return { output, manifest, manifestPath };
}

test('verifier accepts the exact provisional eight-coordinate manifest', () => {
  const { output } = fixture();
  try {
    assert.equal(verifyEvidence(output, 'provisional', { checkSourceInputs: false }).artifacts.length, 8);
  } finally {
    rmSync(output, { recursive: true });
  }
});

test('verifier keeps final capture bytes separate from human approval', () => {
  const { output, manifest, manifestPath } = fixture();
  try {
    manifest.capture_status = 'final';
    writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
    const verified = verifyEvidence(output, 'final', {
      checkSourceInputs: false,
      requiredReview: 'pending_human',
    });
    assert.equal(verified.review.status, 'pending_human');
  } finally {
    rmSync(output, { recursive: true });
  }
});

test('verifier rejects missing, mismatched, and stale evidence', async t => {
  await t.test('missing artifact', () => {
    const { output, manifest } = fixture();
    try {
      unlinkSync(join(output, manifest.artifacts[0].path));
      assert.throws(
        () => verifyEvidence(output, 'provisional', { checkSourceInputs: false }),
        /missing artifact file/,
      );
    } finally {
      rmSync(output, { recursive: true });
    }
  });

  await t.test('hash mismatch', () => {
    const { output, manifest } = fixture();
    try {
      const path = join(output, manifest.artifacts[0].path);
      const bytes = readFileSync(path);
      bytes[8] ^= 1;
      writeFileSync(path, bytes);
      assert.throws(
        () => verifyEvidence(output, 'provisional', { checkSourceInputs: false }),
        /artifact hash mismatch/,
      );
    } finally {
      rmSync(output, { recursive: true });
    }
  });

  await t.test('source tree mismatch', () => {
    const { output, manifest, manifestPath } = fixture();
    try {
      manifest.source.tree = '0000000000000000000000000000000000000000';
      writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
      assert.throws(
        () => verifyEvidence(output, 'provisional', { checkSourceInputs: false }),
        /source tree mismatch/,
      );
    } finally {
      rmSync(output, { recursive: true });
    }
  });
});
