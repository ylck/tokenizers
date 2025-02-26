use crate::utils::padding::PaddingDirection;
use rayon::prelude::*;

/// Represents the output of a `Tokenizer`.
#[derive(Default, PartialEq, Debug, Clone)]
pub struct Encoding {
    ids: Vec<u32>,
    type_ids: Vec<u32>,
    tokens: Vec<String>,
    offsets: Vec<(usize, usize)>,
    special_tokens_mask: Vec<u32>,
    attention_mask: Vec<u32>,
    overflowing: Vec<Encoding>,
}
impl Encoding {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ids: Vec<u32>,
        type_ids: Vec<u32>,
        tokens: Vec<String>,
        offsets: Vec<(usize, usize)>,
        special_tokens_mask: Vec<u32>,
        attention_mask: Vec<u32>,
        overflowing: Vec<Encoding>,
    ) -> Self {
        Encoding {
            ids,
            type_ids,
            tokens,
            offsets,
            special_tokens_mask,
            attention_mask,
            overflowing,
        }
    }

    pub fn get_tokens(&self) -> &[String] {
        &self.tokens[..]
    }

    pub fn get_ids(&self) -> &[u32] {
        &self.ids
    }

    pub fn get_type_ids(&self) -> &[u32] {
        &self.type_ids
    }

    pub fn get_offsets(&self) -> &[(usize, usize)] {
        &self.offsets
    }

    pub fn get_offsets_mut(&mut self) -> &mut [(usize, usize)] {
        &mut self.offsets
    }

    pub fn get_special_tokens_mask(&self) -> &[u32] {
        &self.special_tokens_mask
    }

    pub fn get_attention_mask(&self) -> &[u32] {
        &self.attention_mask
    }

    pub fn get_overflowing(&self) -> &Vec<Encoding> {
        &self.overflowing
    }

    pub fn take_overflowing(&mut self) -> Vec<Encoding> {
        std::mem::replace(&mut self.overflowing, vec![])
    }

    /// Truncate the current `Encoding`.
    ///
    /// Panic if `stride >= max_len` or `max_len == 0`.
    pub fn truncate(&mut self, max_len: usize, stride: usize) {
        if max_len >= self.ids.len() {
            return;
        }
        // We only truncate if max_len > 0, it makes no sense otherwise
        assert!(max_len > 0);

        // Get the main overflowing part
        let o_ids = self.ids.split_off(max_len);
        let o_type_ids = self.type_ids.split_off(max_len);
        let o_tokens = self.tokens.split_off(max_len);
        let o_offsets = self.offsets.split_off(max_len);
        let o_spe_toks = self.special_tokens_mask.split_off(max_len);
        let o_attent = self.attention_mask.split_off(max_len);

        // Now we need to separate the overflowing part into as many Encoding as needed
        assert!(stride < max_len);
        let part_size = max_len - stride;
        let mut overflowing = vec![];
        let mut part_id = 0;
        let mut prev_encoding: &Encoding = self;

        loop {
            if part_size * part_id >= o_ids.len() {
                break;
            }

            let o = Encoding {
                ids: get_current_part(&prev_encoding.ids, &o_ids, part_size, part_id, stride),
                type_ids: get_current_part(
                    &prev_encoding.type_ids,
                    &o_type_ids,
                    part_size,
                    part_id,
                    stride,
                ),
                tokens: get_current_part(
                    &prev_encoding.tokens,
                    &o_tokens,
                    part_size,
                    part_id,
                    stride,
                ),
                offsets: get_current_part(
                    &prev_encoding.offsets,
                    &o_offsets,
                    part_size,
                    part_id,
                    stride,
                ),
                special_tokens_mask: get_current_part(
                    &prev_encoding.special_tokens_mask,
                    &o_spe_toks,
                    part_size,
                    part_id,
                    stride,
                ),
                attention_mask: get_current_part(
                    &prev_encoding.attention_mask,
                    &o_attent,
                    part_size,
                    part_id,
                    stride,
                ),
                overflowing: vec![],
            };

            part_id += 1;
            overflowing.push(o);
            prev_encoding = &overflowing.last().unwrap();
        }

        self.overflowing = overflowing;
    }

    /// Merge ourself with the given `Encoding`. Happens in place.
    pub fn merge_with(&mut self, pair: Encoding, growing_offsets: bool) {
        // Handle merging the overflowing parts too: Combine them all
        // In most of the cases, we expect `pair.overflowing.len() == 0`
        let mut overflowings = vec![];

        // 1. All our overflowings with all the others
        for self_o in &self.overflowing {
            // 1. The pair itself
            let mut n_encoding = self_o.clone();
            n_encoding.merge_with(pair.clone(), growing_offsets);
            overflowings.push(n_encoding);

            // 2. Its overflowings (this should rarely happen...)
            for other_o in &pair.overflowing {
                let mut n_encoding = self_o.clone();
                n_encoding.merge_with(other_o.clone(), growing_offsets);
                overflowings.push(n_encoding);
            }
        }
        // 2. Ourself with all the other overflowings (this should rarely happen too...)
        for other_o in &pair.overflowing {
            let mut n_encoding = self.clone();
            n_encoding.merge_with(other_o.clone(), growing_offsets);
            overflowings.push(n_encoding);
        }

        // Finish by merging ourself with the other encoding
        self.ids.extend(pair.ids);
        self.type_ids.extend(pair.type_ids);
        self.tokens.extend(pair.tokens);

        let starting_offset = if growing_offsets {
            self.offsets.last().map_or(0, |o| o.1)
        } else {
            0
        };
        self.offsets.extend(
            pair.offsets
                .into_iter()
                .map(|(start, end)| (start + starting_offset, end + starting_offset))
                .collect::<Vec<_>>(),
        );
        self.special_tokens_mask.extend(pair.special_tokens_mask);
        self.attention_mask.extend(pair.attention_mask);
        self.overflowing = overflowings;
    }

    pub fn pad(
        &mut self,
        target_length: usize,
        pad_id: u32,
        pad_type_id: u32,
        pad_token: &str,
        direction: PaddingDirection,
    ) {
        // Dispatch call to all the overflowings first
        self.overflowing.par_iter_mut().for_each(|encoding| {
            encoding.pad(target_length, pad_id, pad_type_id, pad_token, direction)
        });

        // Then check if we should pad ourself
        if self.ids.len() >= target_length {
            // We just do nothing if the wanted padding length is smaller than us
            return;
        }
        let pad_length = target_length - self.ids.len();

        match direction {
            PaddingDirection::Left => {
                self.ids = (0..pad_length)
                    .map(|_| pad_id)
                    .chain(self.ids.drain(..))
                    .collect();
                self.type_ids = (0..pad_length)
                    .map(|_| pad_type_id)
                    .chain(self.type_ids.drain(..))
                    .collect();
                self.tokens = (0..pad_length)
                    .map(|_| pad_token.to_owned())
                    .chain(self.tokens.drain(..))
                    .collect();
                self.attention_mask = (0..pad_length)
                    .map(|_| 0)
                    .chain(self.attention_mask.drain(..))
                    .collect();
                self.special_tokens_mask = (0..pad_length)
                    .map(|_| 1)
                    .chain(self.special_tokens_mask.drain(..))
                    .collect();
                self.offsets = (0..pad_length)
                    .map(|_| (0, 0))
                    .chain(self.offsets.drain(..))
                    .collect();
            }
            PaddingDirection::Right => {
                self.ids.extend((0..pad_length).map(|_| pad_id));
                self.type_ids.extend((0..pad_length).map(|_| pad_type_id));
                self.tokens
                    .extend((0..pad_length).map(|_| pad_token.to_owned()));
                self.attention_mask.extend((0..pad_length).map(|_| 0));
                self.special_tokens_mask.extend((0..pad_length).map(|_| 1));
                self.offsets.extend((0..pad_length).map(|_| (0, 0)));
            }
        }
    }
}

#[inline]
fn get_current_part<T: Clone>(
    prev: &[T],
    current: &[T],
    size: usize,
    idx: usize,
    stride: usize,
) -> Vec<T> {
    let curr_slice = if (idx + 1) * size > current.len() {
        &current[idx * size..]
    } else {
        &current[idx * size..(idx + 1) * size]
    };
    let prev_slice = &prev[prev.len() - stride..];
    [prev_slice, curr_slice].concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_encodings() {
        let mut a = Encoding {
            ids: vec![1],
            type_ids: vec![0],
            tokens: vec![String::from("Hello ")],
            offsets: vec![(0, 6)],
            special_tokens_mask: vec![0],
            attention_mask: vec![1],
            overflowing: vec![],
        };
        let b = Encoding {
            ids: vec![2],
            type_ids: vec![1],
            tokens: vec![String::from("World!")],
            offsets: vec![(0, 6)],
            special_tokens_mask: vec![0],
            attention_mask: vec![1],
            overflowing: vec![],
        };
        a.merge_with(b, true);

        assert_eq!(
            a,
            Encoding {
                ids: vec![1, 2],
                type_ids: vec![0, 1],
                tokens: vec![String::from("Hello "), String::from("World!")],
                offsets: vec![(0, 6), (6, 12)],
                special_tokens_mask: vec![0, 0],
                attention_mask: vec![1, 1],
                overflowing: vec![],
            }
        );
    }

    #[test]
    fn truncate() {
        let mut a = Encoding {
            ids: vec![1, 2, 3],
            type_ids: vec![0, 0, 0],
            tokens: vec![
                String::from("Hello"),
                String::from("World"),
                String::from("!"),
            ],
            offsets: vec![(0, 5), (6, 11), (11, 12)],
            special_tokens_mask: vec![0, 0, 0],
            attention_mask: vec![1, 1, 1],
            overflowing: vec![],
        };
        a.truncate(2, 0);

        assert_eq!(
            a,
            Encoding {
                ids: vec![1, 2],
                type_ids: vec![0, 0],
                tokens: vec![String::from("Hello"), String::from("World")],
                offsets: vec![(0, 5), (6, 11)],
                special_tokens_mask: vec![0, 0],
                attention_mask: vec![1, 1],
                overflowing: vec![Encoding {
                    ids: vec![3],
                    type_ids: vec![0],
                    tokens: vec![String::from("!")],
                    offsets: vec![(11, 12)],
                    special_tokens_mask: vec![0],
                    attention_mask: vec![1],
                    overflowing: vec![],
                }]
            }
        );
    }
}
