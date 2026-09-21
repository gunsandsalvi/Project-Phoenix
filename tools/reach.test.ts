/**
 * What the reach walk asserts, against FIXTURES rather than against today's kernel: a test that
 * encodes the state of the tree goes red when the tree is wired up, which is the opposite of what
 * a test is for.
 *
 * Run by `npm run check:tools` (node's own runner, through tsx).
 */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { absentCitations, namesInTree, unreached, withoutTests } from './reach.js';

/** A kernel of the files given, so the walk has an `assembly.rs` and a `systems.rs` to start at. */
function kernel(files: Record<string, string>): string {
  const at = mkdtempSync(join(tmpdir(), 'phoenix-reach-'));
  for (const [name, text] of Object.entries(files)) writeFileSync(join(at, name), text);
  return at;
}

/** The two doors every fixture needs: a world to build, and a table to wire it from. */
const ASSEMBLY = 'pub struct World {\n    parties: Parties,\n}\n';
const NOTHING = 'pub fn all() -> Vec<Wired> {\n    Vec::new()\n}\n';

test('a cluster that only calls itself is not reached', () => {
  // The defect this walk exists to end: `money.rs` is entered for one type, and five items beside
  // it that nothing wires read as live because the module names them.
  const at = kernel({
    'assembly.rs': ASSEMBLY,
    'systems.rs': 'pub fn all() -> Vec<Wired> {\n    vec![Owed::new()]\n}\n',
    'money.rs':
      'pub struct Owed {}\n' +
      'impl Owed {\n    pub fn new() -> Owed {\n        Owed {}\n    }\n}\n' +
      'pub trait Issuer {}\n' +
      'pub fn as_legs(i: &Issuer) -> u32 {\n    0\n}\n',
  });
  assert.deepEqual(
    unreached(at)
      .filter((u) => u.file.endsWith('money.rs'))
      .map((u) => u.item),
    ['as_legs', 'Issuer'],
  );
});

test('what a wired type does inside a trait method is what that type reaches', () => {
  // `run` is a trait method nobody names, and everything a mechanism does happens in it. A walk
  // that attributed those names to `run` would report the whole world unreached.
  const at = kernel({
    'assembly.rs': ASSEMBLY,
    'systems.rs': 'pub fn all() -> Vec<Wired> {\n    vec![Servicing {}]\n}\n',
    'lending.rs':
      'pub struct Servicing {}\n' +
      'impl Mechanism for Servicing {\n' +
      '    fn run(&self, ctx: &mut MechanismContext) {\n' +
      '        pays(ctx);\n' +
      '    }\n' +
      '}\n' +
      'pub fn pays(ctx: &mut MechanismContext) -> u32 {\n    0\n}\n',
  });
  assert.deepEqual(unreached(at), []);
});

test('a private helper carries reach through to what it calls', () => {
  const at = kernel({
    'assembly.rs': ASSEMBLY,
    'systems.rs': 'pub fn all() -> Vec<Wired> {\n    vec![helper()]\n}\n',
    'goods.rs':
      'fn helper() -> u32 {\n    onward()\n}\n' + 'pub fn onward() -> u32 {\n    0\n}\n',
  });
  assert.deepEqual(unreached(at), []);
});

test('a test exercising a helper is not the world running it', () => {
  const at = kernel({
    'assembly.rs': ASSEMBLY,
    'systems.rs': NOTHING,
    'goods.rs':
      'pub fn carry(cost: f64) -> f64 {\n    cost\n}\n' +
      '#[cfg(test)]\nmod tests {\n    fn t() {\n        carry(1.0);\n    }\n}\n',
  });
  assert.deepEqual(
    unreached(at).map((u) => u.item),
    ['carry'],
  );
});

test('a private item is a node and never a finding: only what a row could cite is reported', () => {
  const at = kernel({
    'assembly.rs': ASSEMBLY,
    'systems.rs': NOTHING,
    'goods.rs': 'fn hidden() -> u32 {\n    0\n}\n',
  });
  assert.deepEqual(unreached(at), []);
});

test('a body between a declaration and its brace still belongs to the declaration', () => {
  // A `where` clause puts the opening brace on a later line, and the names below it are the
  // function's own.
  const at = kernel({
    'assembly.rs': ASSEMBLY,
    'systems.rs': 'pub fn all() -> Vec<Wired> {\n    vec![wide(1)]\n}\n',
    'goods.rs':
      'pub fn wide<T>(x: T) -> u32\nwhere\n    T: Into<u32>,\n{\n    onward()\n}\n' +
      'pub fn onward() -> u32 {\n    0\n}\n',
  });
  assert.deepEqual(unreached(at), []);
});

test('a name in a comment or a string is not a call', () => {
  const at = kernel({
    'assembly.rs': ASSEMBLY,
    'systems.rs':
      'pub fn all() -> Vec<Wired> {\n' +
      '    // onward() would be the door\n' +
      '    let said = "onward";\n' +
      '    Vec::new()\n' +
      '}\n',
    'goods.rs': 'pub fn onward() -> u32 {\n    0\n}\n',
  });
  assert.deepEqual(
    unreached(at).map((u) => u.item),
    ['onward'],
  );
});

test('a cfg(test) block is removed by brace depth, and the code after it survives', () => {
  const text = 'pub fn a() {}\n#[cfg(test)]\nmod t {\n    fn inner() { let x = 1; }\n}\npub fn b() {}\n';
  assert.equal(withoutTests(text), 'pub fn a() {}\n\npub fn b() {}\n');
});

test('a citation naming something the tree does not contain is reported, whatever the mark', () => {
  // The step past a claim on unreached code: there is not even an item that failed to be entered,
  // so the citation resolves to nothing and cannot be wrong.
  const present = new Set(['schedule_of', 'PaymentFrequency']);
  assert.deepEqual(
    absentCitations(
      [
        { id: 'Bond N6', status: 'MET', where: 'instruments.rs `Periodicity` and `schedule_of`' },
        { id: 'Bond N5', status: 'MET', where: 'instruments.rs `PaymentFrequency` states it' },
        { id: 'Bond N4', status: 'MISSING', where: '`plus_months` would place it' },
      ],
      present,
    ),
    [
      { id: 'Bond N6', status: 'MET', names: 'Periodicity' },
      { id: 'Bond N4', status: 'MISSING', names: 'plus_months' },
    ],
  );
});

test('a qualified citation is read segment by segment, because each names a real thing', () => {
  assert.deepEqual(
    absentCitations([{ id: 'X', status: 'MET', where: '`agreed::COMMITMENT`' }], new Set(['agreed'])),
    [{ id: 'X', status: 'MET', names: 'COMMITMENT' }],
  );
});

test('the tree is read whole, so a variant a test uses is still a name the tree contains', () => {
  const at = kernel({
    'assembly.rs': ASSEMBLY,
    'systems.rs': NOTHING,
    'goods.rs': '#[cfg(test)]\nmod tests {\n    fn t() { let g = Gone::Perished; }\n}\n',
  });
  assert.equal(namesInTree(at).has('Perished'), true);
});
