//! Collection models for lists, grids, tables, trees, virtualization, and selection.

mod grid;
mod identity;
mod list;
mod math;
mod navigation;
mod selection;
mod table;
mod tree_layout;
mod tree_model;
mod virtual_list;
mod virtual_table;
mod virtual_tree;
mod virtualization;

pub use grid::{GridColumns, GridLayout};
pub use identity::{CollectionProjectedItem, CollectionProjection, ItemId, ItemRect};
pub use list::ListLayout;
pub use navigation::{CollectionCursor, CollectionCursorMove, CollectionCursorTarget};
pub use selection::{Selection, SelectionProjection, SelectionProjectionPolicy};
pub use table::{
    SortDirection, TableCellRect, TableColumn, TableColumnConstraints, TableHeaderRect,
    TableLayout, TableSort,
};
pub use tree_layout::{TreeLayout, TreeRow, TreeRowRect};
pub use tree_model::{TreeExpansion, TreeItem, TreeModel, TreeModelError};
pub use virtual_list::{
    VirtualList, VirtualListConfig, VirtualListItemResponse, VirtualListOutput, VirtualListRow,
    VirtualListSelectionMode,
};
pub use virtual_table::{
    VirtualTable, VirtualTableConfig, VirtualTableHeaderResponse, VirtualTableMaterializedRow,
    VirtualTableOutput, VirtualTableRow, VirtualTableWindow,
};
pub use virtual_tree::{
    VirtualTree, VirtualTreeConfig, VirtualTreeItemResponse, VirtualTreeOutput, VirtualTreeRow,
    VirtualTreeSelectionMode,
};
pub use virtualization::{
    VirtualRangeRequest, VirtualWindow, VirtualWindowRequest, clamp_virtual_scroll_offset,
    virtual_content_extent, virtual_max_scroll_offset, virtual_range, virtual_window,
};

#[cfg(test)]
mod tests;
