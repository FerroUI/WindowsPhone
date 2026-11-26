import assert from 'node:assert/strict';
import test from 'node:test';
import {assertGameGridGeometry} from './ui-sdk-games.mjs';

function fixture(tracks, scale = 1) {
  const gap = 2;
  const offsets = tracks.map((_, index) => tracks.slice(0, index).reduce((sum, size) => sum + size + gap, 0));
  const extent = tracks.reduce((sum, size) => sum + size, gap * (tracks.length - 1));
  const board = {name: 'MinesweeperBoard', x: 460, y: 363, width: extent, height: extent};
  const cells = tracks.flatMap((height, row) => tracks.map((width, column) => ({
    name: `MineCell${row * tracks.length + column}`,
    x: board.x + offsets[column], y: board.y + offsets[row], width, height, hit: true, enabled: true,
  })));
  const verify = () => assertGameGridGeometry(board, cells, 'MineCell', tracks.length, tracks.length, gap, scale, true);
  return {board, cells, verify};
}

test('native grid remainder distribution preserves complete playable coverage at DPR1 and DPR2', () => {
  // The DPR1 allocation is from the retained a4a7b85 browser failure. Its third
  // column starts at x=92, not 90.5; every native cell fills the correct track.
  const one = fixture([44, 44, 43, 43, 43, 43, 43, 43]);
  assert.equal(one.verify()[2][0], 92);
  const two = fixture([43.5, 43.5, 43.5, 43.5, 43, 43, 43, 43], 2);
  assert.equal(two.verify().length, 64);
});

test('grid validation rejects lost cells, overlap, gaps, unused margins and undersized controls', () => {
  const invalid = [
    value => value.cells.pop(),
    value => { value.cells[1].name = 'MineCell0'; },
    value => { value.cells[2].x -= 1; },
    value => { value.cells[2].x += 1; },
    value => { value.cells[20].width -= 1; },
    value => { for (const cell of value.cells) cell.x += 1; },
    value => { value.board.width += 1; },
    value => { value.cells[0].width = 0; },
    value => { value.cells[0].x = NaN; },
  ];
  for (const mutate of invalid) {
    const value = fixture([44, 44, 43, 43, 43, 43, 43, 43]);
    mutate(value);
    assert.throws(value.verify, {name: 'AssertionError'});
  }
});

test('empty game cells must remain enabled and reachable', () => {
  for (const property of ['hit', 'enabled']) {
    const value = fixture([44, 44, 43, 43, 43, 43, 43, 43]);
    value.cells[17][property] = false;
    assert.throws(value.verify, /empty cell 17 accepts native input/);
  }
});
