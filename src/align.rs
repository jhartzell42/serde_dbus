use std::num::Wrapping;

pub(crate) fn align(ix: usize, alignment: usize) -> usize {
    debug_assert!(
        alignment.is_power_of_two(),
        "{} is not power of 2, cannot be used as alignment",
        alignment
    );
    let mask = Wrapping(alignment) - Wrapping(1);
    let old_size = Wrapping(ix);
    let new_size = old_size + ((-old_size) & mask);
    debug_assert!(
        new_size >= old_size,
        "align function broke: {} < {}",
        new_size,
        old_size
    );
    new_size.0
}

#[cfg(test)]
mod tests {
    use crate::align::align;

    #[test]
    fn alignment() {
        assert_eq!(align(23usize, 4usize), 24usize);
        assert_eq!(align(32usize, 4usize), 32usize);
        assert_eq!(align(31usize, 1usize), 31usize);
        assert_eq!(align(0usize, 1usize), 0usize);
        assert_eq!(align(25usize, 4usize), 28usize);
    }
}
