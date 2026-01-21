use std::cell::OnceCell;
use std::collections::HashMap;

use page_manager::{
    PageRawSourceType, SearchOptions, SearchResultItem, SearchResultSource,
    SearchResults,
};

use crate::global::search_manager;

fn search_source_to_page_source(source: SearchResultSource) -> Option<PageRawSourceType> {
    match source {
        SearchResultSource::ActiveTab => Some(PageRawSourceType::ActiveTab),
        SearchResultSource::Bookmark => Some(PageRawSourceType::Bookmark),
        SearchResultSource::History => Some(PageRawSourceType::ClosedTab),
        SearchResultSource::Archive => Some(PageRawSourceType::ArchivedContent),
        SearchResultSource::UnifiedPage => None,
    }
}

#[derive(Debug, Clone)]
pub struct PageSearchResults {
    pages: Vec<SearchResultItem>,
    search_time_ms: u64,
    count_by_browser: OnceCell<HashMap<i32, usize>>,
    count_by_source: OnceCell<HashMap<i32, usize>>,
}

impl PageSearchResults {
    pub fn new(search_results: SearchResults) -> Self {
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

    /// Returns the total number of search results.
    pub fn total_results(&self) -> usize {
        self.pages.len()
    }

    /// Returns the time taken to perform the search in milliseconds.
    pub fn search_time_ms(&self) -> u64 {
        self.search_time_ms
    }

    /// Returns a slice of search result items from start index with given length.
    pub fn ranges(&self, start: usize, len: usize) -> Option<&[SearchResultItem]> {
        let start = std::cmp::min(start, self.pages.len());
        let end = std::cmp::min(start + len, self.pages.len());
        self.pages.get(start..end)
    }

    /// Returns the count of pages by browser type.
    pub fn count_by_browser(&self) -> &HashMap<i32, usize> {
        self.count_by_browser.get_or_init(|| {
            let mut counts = HashMap::new();
            for page in &self.pages {
                let Some(browser_type) = page.browser_type else {
                    continue;
                };
                *counts.entry(browser_type.into()).or_insert(0) += 1;
            }
            counts
        })
    }

    /// Returns the count of pages by source type.
    pub fn count_by_source(&self) -> &HashMap<i32, usize> {
        self.count_by_source.get_or_init(|| {
            let mut counts = HashMap::new();
            for page in &self.pages {
                let Some(source_type) = search_source_to_page_source(page.source_type) else {
                    continue;
                };
                *counts.entry(source_type.into()).or_insert(0) += 1;
            }
            counts
        })
    }
}

// todo: add full filter options.
pub async fn search(
    query: &str,
    browser_type: Option<i32>,
    source_type: Option<i32>,
) -> PageSearchResults {
    let mut options = SearchOptions::default();
    options.filter.browser_type = browser_type.and_then(|t| t.try_into().ok());
    let search_results = search_manager().search(query, options).await;
    PageSearchResults::new(search_results)
}
