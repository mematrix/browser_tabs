#pragma once

#include "ai_processor.h"
#include <string>
#include <vector>

namespace web_page_manager {
namespace ai {

/// Suggester for intelligent page grouping
class GroupSuggester {
public:
    GroupSuggester();
    ~GroupSuggester();
    
    /// Suggest groups based on content similarity
    std::vector<GroupSuggestion> SuggestByContent(
        const std::vector<PageContent>& pages,
        double similarity_threshold = 0.6
    );
    
    /// Suggest groups based on domain
    std::vector<GroupSuggestion> SuggestByDomain(
        const std::vector<PageContent>& pages
    );
    
    /// Suggest groups based on topics
    std::vector<GroupSuggestion> SuggestByTopic(
        const std::vector<PageContent>& pages
    );
    
    /// Merge similar groups
    std::vector<GroupSuggestion> MergeGroups(
        const std::vector<GroupSuggestion>& groups,
        double merge_threshold = 0.8
    );
    
    /// Generate group name from content
    std::string GenerateGroupName(const std::vector<PageContent>& pages);
    
    /// Generate group description from content
    std::string GenerateGroupDescription(const std::vector<PageContent>& pages);
    
    /// Suggest groups using combined analysis (content + domain + topic)
    /// @param pages Vector of page contents to analyze
    /// @param similarity_threshold Minimum similarity for grouping
    /// @return Vector of group suggestions with combined scoring
    std::vector<GroupSuggestion> SuggestGroupsCombined(
        const std::vector<PageContent>& pages,
        double similarity_threshold = 0.5
    );
    
    /// Generate cross-content recommendations
    /// @param pages Vector of page contents
    /// @param min_relevance Minimum relevance score for recommendations
    /// @return Vector of cross-recommendations between pages
    std::vector<CrossRecommendation> GenerateCrossRecommendations(
        const std::vector<PageContent>& pages,
        float min_relevance = 0.5f
    );
    
    /// Rank group suggestions by quality
    /// @param suggestions Vector of suggestions to rank
    /// @return Sorted vector with quality scores
    std::vector<GroupSuggestion> RankSuggestions(
        const std::vector<GroupSuggestion>& suggestions
    );
    
    /// Detect content clusters using hierarchical clustering
    /// @param pages Vector of page contents
    /// @param num_clusters Target number of clusters (0 for auto)
    /// @return Vector of group suggestions representing clusters
    std::vector<GroupSuggestion> DetectClusters(
        const std::vector<PageContent>& pages,
        size_t num_clusters = 0
    );
    
private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace ai
} // namespace web_page_manager
