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
    
private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace ai
} // namespace web_page_manager
