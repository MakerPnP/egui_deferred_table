use crate::TableDimensions;

pub trait DeferredTableDataSource {
    /// called once per frame, before any other methods are used.
    fn prepare(&mut self) {}
    /// called once per frame, after the source has been used.
    fn finalize(&mut self) {}

    fn get_dimensions(&self) -> TableDimensions;
}
