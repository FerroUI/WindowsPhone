use super::{Cell, Game};

impl Game {
    pub(super) fn neighbors(&self, index: usize) -> impl Iterator<Item = usize> {
        let (width, height, _) = self.difficulty.dimensions();
        let x = index % width;
        let y = index / width;
        let mut output = [usize::MAX; 8];
        let mut count = 0;
        for row in y.saturating_sub(1)..=(y + 1).min(height - 1) {
            for column in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                let next = row * width + column;
                if next != index {
                    output[count] = next;
                    count += 1;
                }
            }
        }
        output.into_iter().take(count)
    }

    pub(super) fn place_mines(&mut self, first: usize) {
        let (_, _, mines) = self.difficulty.dimensions();
        let excluded: Vec<_> = self
            .neighbors(first)
            .chain(std::iter::once(first))
            .collect();
        let mut available: Vec<_> = (0..self.cells.len())
            .filter(|index| !excluded.contains(index))
            .collect();
        let mut random = self.seed.max(1);
        for index in 0..mines {
            random ^= random << 13;
            random ^= random >> 17;
            random ^= random << 5;
            let selected = index + (random % (available.len() - index) as u32) as usize;
            available.swap(index, selected);
            self.cells[available[index]].mine = true;
        }
        for index in 0..self.cells.len() {
            self.cells[index].adjacent = self
                .neighbors(index)
                .filter(|other| self.cells[*other].mine)
                .count() as u8;
        }
    }

    pub(super) fn expected_cells(&self, first: usize) -> Vec<Cell> {
        let mut game = Self::new(self.difficulty, self.seed);
        game.place_mines(first);
        game.cells
    }
}
