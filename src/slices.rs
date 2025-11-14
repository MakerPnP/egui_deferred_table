use crate::CellIndex;
use crate::DeferredTableDataSource;
use crate::DeferredTableRenderer;
use crate::TableDimensions;

use egui::Ui;

/// Helper for rendering tables based on tuple slices
///
/// Implementations of `DeferredTableRender` for tuples with 2 to 16 elements are provided by the
/// `impl_deferred_table_for_tuple` macro.
///
/// See crate examples.
#[derive(Default)]
pub struct SimpleTupleRenderer {}

// define a macro that handles the implementation for a specific tuple size
macro_rules! impl_tuple_for_size {
    // Pattern: tuple type names, tuple size, match arms for indexing
    (($($T:ident),*), $size:expr, $( ($idx:expr, $field:tt) ),* ) => {
        impl<$($T),*> DeferredTableDataSource for &[($($T),*)] {
            fn get_dimensions(&self) -> TableDimensions {
                TableDimensions {
                    row_count: self.len(),
                    column_count: $size,
                }
            }
        }

        impl<$($T: std::fmt::Display),*> DeferredTableRenderer<&[($($T),*)]> for SimpleTupleRenderer {
            fn render_cell(&self, ui: &mut Ui, cell_index: CellIndex, source: &&[($($T),*)]) {
                if let Some(row_data) = source.get(cell_index.row) {
                    match cell_index.column {
                        $( $idx => ui.label(row_data.$field.to_string()), )*
                        _ => panic!("cell_index out of bounds. {:?}", cell_index),
                    };
                }
            }
        }
    };
}

// use a front-end macro that calls the implementation macro with the right parameters
macro_rules! impl_deferred_table_for_tuple {
    ((A, B), 2) => {
        impl_tuple_for_size!((A, B), 2, (0, 0), (1, 1));
    };

    ((A, B, C), 3) => {
        impl_tuple_for_size!((A, B, C), 3, (0, 0), (1, 1), (2, 2));
    };

    ((A, B, C, D), 4) => {
        impl_tuple_for_size!((A, B, C, D), 4, (0, 0), (1, 1), (2, 2), (3, 3));
    };

    ((A, B, C, D, E), 5) => {
        impl_tuple_for_size!((A, B, C, D, E), 5, (0, 0), (1, 1), (2, 2), (3, 3), (4, 4));
    };

    ((A, B, C, D, E, F), 6) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F),
            6,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5)
        );
    };

    ((A, B, C, D, E, F, G), 7) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G),
            7,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6)
        );
    };

    ((A, B, C, D, E, F, G, H), 8) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G, H),
            8,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7)
        );
    };

    ((A, B, C, D, E, F, G, H, I), 9) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G, H, I),
            9,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7),
            (8, 8)
        );
    };

    ((A, B, C, D, E, F, G, H, I, J), 10) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G, H, I, J),
            10,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7),
            (8, 8),
            (9, 9)
        );
    };

    ((A, B, C, D, E, F, G, H, I, J, K), 11) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G, H, I, J, K),
            11,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7),
            (8, 8),
            (9, 9),
            (10, 10)
        );
    };

    ((A, B, C, D, E, F, G, H, I, J, K, L), 12) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G, H, I, J, K, L),
            12,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7),
            (8, 8),
            (9, 9),
            (10, 10),
            (11, 11)
        );
    };

    ((A, B, C, D, E, F, G, H, I, J, K, L, M), 13) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G, H, I, J, K, L, M),
            13,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7),
            (8, 8),
            (9, 9),
            (10, 10),
            (11, 11),
            (12, 12)
        );
    };

    ((A, B, C, D, E, F, G, H, I, J, K, L, M, N), 14) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G, H, I, J, K, L, M, N),
            14,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7),
            (8, 8),
            (9, 9),
            (10, 10),
            (11, 11),
            (12, 12),
            (13, 13)
        );
    };

    ((A, B, C, D, E, F, G, H, I, J, K, L, M, N, O), 15) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O),
            15,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7),
            (8, 8),
            (9, 9),
            (10, 10),
            (11, 11),
            (12, 12),
            (13, 13),
            (14, 14)
        );
    };

    ((A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P), 16) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P),
            16,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7),
            (8, 8),
            (9, 9),
            (10, 10),
            (11, 11),
            (12, 12),
            (13, 13),
            (14, 14),
            (15, 15)
        );
    };
}

impl_deferred_table_for_tuple!((A, B), 2);
impl_deferred_table_for_tuple!((A, B, C), 3);
impl_deferred_table_for_tuple!((A, B, C, D), 4);
impl_deferred_table_for_tuple!((A, B, C, D, E), 5);
impl_deferred_table_for_tuple!((A, B, C, D, E, F), 6);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G), 7);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G, H), 8);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G, H, I), 9);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G, H, I, J), 10);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G, H, I, J, K), 11);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G, H, I, J, K, L), 12);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G, H, I, J, K, L, M), 13);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G, H, I, J, K, L, M, N), 14);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G, H, I, J, K, L, M, N, O), 15);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P), 16);
