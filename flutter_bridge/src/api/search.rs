use std::collections::HashMap;
use std::cell::OnceCell;

use ui_ffi_common::pm::{SearchResultItem, SearchResults};
use ui_ffi_common::pm_core::{BrowserType, PageRawSourceType, Uuid};

#[derive(Debug, Clone)]
pub struct PageSearchResults {
    pages: Vec<SearchResultItem>,
    search_time_ms: u64,
    count_by_browser: OnceCell<HashMap<BrowserType, usize>>,
    count_by_source: OnceCell<HashMap<PageRawSourceType, usize>>,
}

impl PageSearchResults {
    pub fn new(
        search_results: SearchResults,
    ) -> Self {
        Self {
            pages: search_results.items,
            search_time_ms: search_results.search_time_ms,
            count_by_browser: OnceCell::new(),
            count_by_source: OnceCell::new(),
        }
    }

    /// Returns the list of page IDs matching the search criteria.
    pub fn pages(&self) -> &[SearchResultItem] {
        &self.pages
    }

    pub fn total_results(&self) -> usize {
        self.pages.len()
    }

    /// Returns the time taken to perform the search in milliseconds.
    pub fn search_time_ms(&self) -> u64 {
        self.search_time_ms
    }

    pub fn ranges(&self, start: usize, len: usize) -> Option<&[SearchResultItem]> {
        let start = std::cmp::min(start, self.pages.len());
        let end = std::cmp::min(start + len, self.pages.len());
        self.pages.get(start..end)
    }

    /// Returns the count of pages by browser type.
    pub fn count_by_browser(&self) -> &HashMap<BrowserType, usize> {
        self.count_by_browser
            .get_or_init(|| {
                let mut counts = HashMap::new();
                for page in &self.pages {
                    let Some(browser_type) = page.browser_type else { continue };
                    *counts.entry(browser_type).or_insert(0) += 1;
                }
                counts
            })
    }

    /// Returns the count of pages by source type.
    pub fn count_by_source(&self) -> &HashMap<PageRawSourceType, usize> {
        self.count_by_source
            .get_or_init(|| {
                let mut counts = HashMap::new();
                for page_id in &self.pages {
                    // Placeholder logic; in a real implementation, you would look up the source type for each page ID.
                    let source_type = PageRawSourceType::ActiveTab; // Example placeholder
                    *counts.entry(source_type).or_insert(0) += 1;
                }
                counts
            })
    }
}

pub async fn search(query: &str, browser_type: Option<BrowserType>, source_type: Option<PageRawSourceType>) -> PageSearchResults {
    // let search_results = page_manager::search::(query, browser_type, source_type).await;
    // PageSearchResults::new(search_results)
    todo!()
}
