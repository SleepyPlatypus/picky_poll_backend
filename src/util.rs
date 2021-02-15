use std::slice::Iter;
use std::hash::Hash;
use std::collections::HashSet;

pub fn first_duplicate<'a, A>(iter: Iter<'a, A>) -> Option<&'a A>
where A: Eq + Hash {
    let mut set = HashSet::<&A>::new();
    for a in iter {
        let new = set.insert(a);
        if !new {
            return Some(a);
        }
    }
    None
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn empty() {
        let empty: Vec<String> = vec!();
        let result = first_duplicate(empty.iter());
        assert!(result.is_none(), "Should return None for empty vector");
    }

    #[test]
    fn finds_dupe() {
        let empty = vec!("🍦", "🍪", "🍪");
        let result = first_duplicate(empty.iter())
            .expect("Should find duplicate");
        assert_eq!("🍪", *result);
    }
}