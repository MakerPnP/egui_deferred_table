/// Helper method to be used by clients to help with handling column re-ordering during action processing.
///
/// ```text
/// match action {
///     Action::ColumnReorder { from, to } => {
///         egui_deferred_table::apply_column_reordering(&mut column_ordering, from, to);
///     }
///     // ...
/// }
///```
///
/// See also:
/// 1. [`DeferredTableDataSource::column_ordering`]
/// 2. [`DeferredTableDataSource::row_ordering`]
/// 3. [`Action::ColumnReorder`]
///
pub fn apply_reordering(ordering: &mut Option<Vec<usize>>, from: usize, to: usize) {
    if from == to {
        return;
    }

    //
    // Part 1
    //

    // Initialize ordering if it doesn't exist
    if ordering.is_none() {
        *ordering = Some(Vec::new());
    }

    // Get a mutable reference to column_ordering
    let ordering = ordering.as_mut().unwrap();

    // Find the maximum index needed
    let max_index = from.max(to);

    // Expand the vector if needed to include max_index
    while ordering.len() <= max_index {
        ordering.push(ordering.len());
    }

    //
    // Part 2: Perform the actual move
    //

    // Find positions of 'from' and 'to' in the ordering vector
    let from_pos = ordering.iter().position(|&x| x == from).unwrap();
    let to_pos = ordering.iter().position(|&x| x == to).unwrap();

    // Remove 'from' from its current position
    ordering.remove(from_pos);

    // if to_pos was after from_pos, it will be out by one, but this is factored into the code below.
    ordering.insert(to_pos, from);
}

#[cfg(test)]
mod reordering_tests {
    use crate::ordering::apply_reordering;
    use rstest::rstest;

    #[rstest]
    // dragging left
    #[case(0,1,vec![1,0], vec![0,1])]
    #[case(4,0,vec![0,1,2,3,4,5,6], vec![4,0,1,2,3,5,6])]
    #[case(10,0,vec![], vec![10,0,1,2,3,4,5,6,7,8,9])]
    // dragging right
    #[case(0,1,vec![], vec![1,0])]
    #[case(1,0,vec![1,0], vec![0,1])]
    #[case(1,0,vec![1,0,2,3,4], vec![0,1,2,3,4])]
    #[case(4,0,vec![], vec![4,0,1,2,3])]
    #[case(4,3,vec![4,0,1,2,3,5,6], vec![0,1,2,3,4,5,6])]
    #[case(10,9,vec![10,0,1,2,3,4,5,6,7,8,9], vec![0,1,2,3,4,5,6,7,8,9,10])]
    // from/to same
    #[case(0,0,vec![], vec![])]
    #[case(4,4,vec![0,1], vec![0,1])]
    fn test_apply_reordering(
        #[case] from: usize,
        #[case] to: usize,
        #[case] ordering: Vec<usize>,
        #[case] expected: Vec<usize>,
    ) {
        let mut ordering = Some(ordering);
        apply_reordering(&mut ordering, from, to);
        assert_eq!(ordering, Some(expected));
    }
}
