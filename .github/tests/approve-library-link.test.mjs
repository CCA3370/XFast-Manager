import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'vitest';

const workflow = readFileSync(new URL('../workflows/approve-library-link.yml', import.meta.url), 'utf8');
const normalized = workflow.replace(/\r\n/g, '\n');
const extractBlock = normalized.split('      - name: Extract library link fields from issue body\n')[1]?.split('      - name: Update data/library_links.json\n')[0];
assert.ok(extractBlock, 'Extraction step must exist');
const script = extractBlock.split('          script: |\n')[1].split('\n').map((line) => line.replace(/^            /, '')).join('\n');
const extract = (body) => {
  const outputs = {};
  let failure;
  new Function('context', 'core', script)(
    { payload: { issue: { body } } },
    { setOutput: (key, value) => { outputs[key] = value; }, setFailed: (message) => { failure = message; } },
  );
  return { outputs, failure };
};
const form = (name, url, suffix = '') => `### Library Name\n\n${name}\n\n### Download URL\n\n${url}${suffix}`;

test('GitHub form with blank lines and following optional section', () => {
  const result = extract(form('Example_Library', 'https://example.com/library', '\n\n### Additional Notes (optional)\n\n_No response_'));
  assert.equal(result.failure, undefined);
  assert.equal(result.outputs.library_name, 'example_library');
  assert.equal(result.outputs.download_url, 'https://example.com/library');
});
test('GitHub form at end of input and CRLF', () => {
  for (const body of [form('Library', 'https://example.com'), form('Library', 'https://example.com').replace(/\n/g, '\r\n')]) {
    assert.equal(extract(body).outputs.download_url, 'https://example.com');
  }
});
test('inline and app bullet formats', () => {
  for (const body of ['Library Name: Library\nDownload URL: https://example.com', '- Library Name: `Library`\n- Download URL: https://example.com']) {
    assert.equal(extract(body).outputs.library_name, 'library');
  }
});
test('missing and unanswered form fields fail without publishing outputs', () => {
  for (const body of ['', form('_No response_', 'https://example.com'), form('Library', '_No response_')]) {
    const result = extract(body);
    assert.ok(result.failure);
    assert.deepEqual(result.outputs, {});
  }
});
test('non-HTTP and malformed URLs fail', () => {
  for (const url of ['file:///etc/passwd', 'javascript:alert(1)', 'not-a-url']) {
    assert.ok(extract(form('Library', url)).failure);
  }
});
test('failure handler does not close issues', () => {
  const failure = normalized.split('      - name: Comment on failure and keep issue open\n')[1];
  assert.ok(failure);
  assert.ok(!failure.includes('github.rest.issues.update'));
  assert.ok(failure.includes('The issue remains open'));
});

test('issue 307 regression: section parser preserves fields for review', () => {
  const body = '### Library Name\n\n[Library Link] aaa_Boundless_Library_1.0\n\n### Download URL\n\nhttps://modsfire.com/2n4lxtpXz3I4C3I)\n\n### Referenced By Scenery (optional)\n\naaa_Boundless_EGCW_Airport\n\n### Additional Notes (optional)\n\n_No response_';
  const result = extract(body);
  assert.equal(result.failure, undefined);
  assert.equal(result.outputs.library_name_raw, '[Library Link] aaa_Boundless_Library_1.0');
  assert.equal(result.outputs.download_url, 'https://modsfire.com/2n4lxtpXz3I4C3I)');
});
