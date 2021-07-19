pub struct CombinationIterator {
    state: Vec<usize>,
    n: usize,
    r: usize,
}

impl CombinationIterator {
    pub fn new(n: usize, r: usize) -> Self {
        let mut state = Vec::new();
        for k in 0..r {
            state.push(k);
        }
        CombinationIterator { state, n, r }
    }
}

impl Iterator for CombinationIterator {
    type Item = Vec<usize>;
    fn next(&mut self) -> Option<Self::Item> {
        // if self.state.len() == 0 {
        //     return None;
        // }
        if self.state[0] == 1 + self.n - self.r {
            return None;
        }
        let copy = self.state.clone();
        let last = self.state.last_mut().unwrap();
        if *last < self.n - 1 {
            *last += 1;
        } else {
            drop(last);
            let mut last_idx = self.r - 1;
            loop {
                // println!("last_idx = {}", last_idx);
                if last_idx == 0 {
                    break;
                }
                if copy[last_idx - 1] < copy[last_idx] - 1 {
                    last_idx -= 1;
                    // println!("last_idx = {}", last_idx);
                    break;
                } else {
                    last_idx -= 1;
                    // println!("last_idx = {}", last_idx);
                }
            }
            // println!("{:?}, last_idx = {}", self.state, last_idx);
            self.state[last_idx] += 1;
            // println!("{:?}, last_idx = {}", self.state, last_idx);
            for idx in (last_idx + 1)..self.r {
                self.state[idx] = self.state[idx - 1] + 1;
                // println!("{:?}", self.state);
            }
        }

        Some(copy)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_combinations() {
        let mut combination_iterator = CombinationIterator::new(10, 3);
        let mut count = 0;
        for combination in combination_iterator {
            count += 1;
            println!("{:?}", combination);
        }
        println!("found {} total combinations", count);
        assert!(count == 10 * 9 * 8 / 3 / 2 / 1);
    }

    #[test]
    fn test_mine_count_partitions() {
        let remaining_mines = 10;
        let groups = vec![0; 3];
        for mut partition_indices in CombinationIterator::new(remaining_mines, groups.len() - 1) {
            partition_indices.insert(0, 0);
            partition_indices.push(remaining_mines);
            // println!("{:?}", partition_indices);
            let mine_counts: Vec<_> = partition_indices.windows(2).map(|w| w[1] - w[0]).collect();
            println!("{:?}", mine_counts);
        }
    }
}
