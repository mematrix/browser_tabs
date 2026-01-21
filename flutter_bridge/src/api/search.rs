use std::collections::HashMap;

use flutter_rust_bridge::frb;
use ui_ffi_common::pm::SearchResultItem;
use ui_ffi_common::search::{search, PageSearchResults};

#[frb(opaque)]
#[derive(Debug, Clone)]
pub struct SearchResults {
    inner: PageSearchResults,
}

impl SearchResults {
    #[frb(ignore)]
    pub fn new(inner: PageSearchResults) -> Self {
        Self { inner }
    }

    /// Returns the list of page IDs matching the search criteria.
    #[frb(getter)]
    pub fn pages(&self) -> &[SearchResultItem] {
        self.inner.pages()
    }

    /// Returns the total number of search results.
    #[frb(getter)]
    pub fn total_results(&self) -> usize {
        self.inner.total_results()
    }

    /// Returns the time taken to perform the search in milliseconds.
    #[frb(getter)]
    pub fn search_time_ms(&self) -> u64 {
        self.inner.search_time_ms()
    }

    /// Returns a slice of search result items from start index with given length.
    #[frb(sync)]
    pub fn ranges(&self, start: usize, len: usize) -> Option<&[SearchResultItem]> {
        self.inner.ranges(start, len)
    }

    /// Returns the count of pages by browser type.
    #[frb(sync)]
    pub fn count_by_browser(&self) -> &HashMap<i32, usize> {
        self.inner.count_by_browser()
    }

    /// Returns the count of pages by source type.
    #[frb(sync)]
    pub fn count_by_source(&self) -> &HashMap<i32, usize> {
        self.inner.count_by_source()
    }
}

pub async fn do_search(
    query: &str,
    browser_type: Option<i32>,
    source_type: Option<i32>,
) -> SearchResults {
    let result = search(query, browser_type, source_type).await;
    SearchResults::new(result)
}
