use std::collections::HashSet;

#[derive(Debug, PartialEq)]
pub enum Change {
    Remove(usize),
    Insert(usize),
    Move { from: usize, to: usize },
}

/// Reconciles unique process identities without resetting surviving rows.
pub fn reconcile(current: &mut Vec<String>, desired: &[String]) -> Vec<Change> {
    let wanted: HashSet<_> = desired.iter().collect();
    let mut changes = Vec::new();
    for index in (0..current.len()).rev() {
        if !wanted.contains(&current[index]) {
            current.remove(index);
            changes.push(Change::Remove(index));
        }
    }
    for (index, key) in desired.iter().enumerate() {
        if current.get(index) == Some(key) {
            continue;
        }
        if let Some(from) = current.iter().position(|candidate| candidate == key) {
            let key = current.remove(from);
            current.insert(index, key);
            changes.push(Change::Move { from, to: index });
        } else {
            current.insert(index, key.clone());
            changes.push(Change::Insert(index));
        }
    }
    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn unchanged_refresh_does_not_touch_the_collection() {
        let mut current = keys(&["1:100", "2:200"]);
        let desired = current.clone();
        assert!(reconcile(&mut current, &desired).is_empty());
    }

    #[test]
    fn sorting_moves_existing_rows_instead_of_replacing_them() {
        let mut current = keys(&["a", "b", "c"]);
        let desired = keys(&["c", "a", "b"]);
        assert_eq!(
            reconcile(&mut current, &desired),
            vec![Change::Move { from: 2, to: 0 }]
        );
        assert_eq!(current, desired);
    }

    #[test]
    fn count_changes_and_pid_reuse_only_replace_affected_identities() {
        let mut current = keys(&["1:100", "2:200", "3:300"]);
        let desired = keys(&["3:300", "2:201", "4:400", "5:500"]);
        assert_eq!(
            reconcile(&mut current, &desired),
            vec![
                Change::Remove(1),
                Change::Remove(0),
                Change::Insert(1),
                Change::Insert(2),
                Change::Insert(3),
            ]
        );
        assert_eq!(current, desired);
    }

    #[test]
    fn filtering_to_empty_and_back_is_incremental() {
        let mut current = keys(&["a", "b"]);
        assert_eq!(
            reconcile(&mut current, &[]),
            vec![Change::Remove(1), Change::Remove(0)]
        );
        assert_eq!(
            reconcile(&mut current, &keys(&["b"])),
            vec![Change::Insert(0)]
        );
    }
}
