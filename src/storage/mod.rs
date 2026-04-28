use crate::{crawler::Page, Result};

pub trait PageStore {
    fn save(&self, page: &Page) -> Result<()>;
}

#[derive(Debug, Default)]
pub struct NoopStore;

impl PageStore for NoopStore {
    fn save(&self, _page: &Page) -> Result<()> {
        Ok(())
    }
}
