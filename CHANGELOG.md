# Changelog

Possible log types:

- `[added]` for new features.
- `[changed]` for changes in existing functionality.
- `[deprecated]` for once-stable features removed in upcoming releases.
- `[removed]` for deprecated features removed in this release.
- `[fixed]` for any bug fixes.
- `[security]` to invite users to upgrade in case of vulnerabilities.

For API migration details, see the git commit history and study the changes made to the demos.

## 0.3.0 (2026/06/29)

- [changed] Update to egui 0.35.0.

## 0.2.0 (2026/06/22)

- [changed] Update to egui 0.34.0, also updated other dependencies.

## 0.1.7 (2025/11/14)

- [added] Allow marked columns to expand to fill the remaining space.
- [changed] Update to egui 0.33.0, also updated other dependencies.

## 0.1.6 (2025/10/22)

- [added] Row selection.  See `Action::RowSelectionChanged`.
- [added] Edit-in-place API.  See `EditableTableRenderer` and `DeferredTable::show_and_edit`.
- [changed] Updated spreadsheet example to have edit-in-place cells.
- [changed] Added a 'shrink' button to the 'growing' example; this allows us to see the behavior of deleting selected
  rows and see that newly added rows are not selected.
- [fixed] incorrect table size when the data source dimensions were made smaller.

## 0.1.5 (2025/09/21)

- [changed] Remove the `DeferredTableBuilder` in favor of a solution that allows caching of the column parameters on an
  as-required basis so they do not have to be built every frame.
- [added] Support for row parameters in addition to column parameters.
- [changed] Move some methods from the `DeferredTableDataSource` to `DeferredTableRender`.  This allows multiple projections
  (aka 'views') of the same data source using different renderers.  See the new 'projections' example.
- [changed] each cell has a consistent ID based on the cell kind and any applicable mapped row/column index.

## 0.1.4 (2025/09/10)

- [fixed] Fix panic when column constraints are not specified for a column.
- [changed] Hovered cell is not highlighted by default. New API methods are available to enable it.

## 0.1.3 (2025/09/10)

- [added] Column constraints (min/max/resizable).

## 0.1.2 (2025/09/09)

- [added] Support row/column resizing.
- [changed] Improved pixel rendering.

## 0.1.1 (2025/09/05)

- [added] Support row/column filtering.
- [added] Support row/column re-ordering.

## 0.1.0 (2025/09/01)

First release
