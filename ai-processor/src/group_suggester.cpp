#include "group_suggester.h"
#include "similarity_calculator.h"
#include <algorithm>
#include <unordered_map>
#include <unordered_set>
#include <sstream>
#include <regex>

namespace web_page_manager {
namespace ai {

class GroupSuggester::Impl {
public:
    SimilarityCalculator similarity_calculator;
    
    // Extract domain from URL-like strings in content
    std::string ExtractDomain(const std::string& text) {
        std::regex url_regex("https?://([^/]+)");
        std::smatch match;
        if (std::regex_search(text, match, url_regex)) {
            return match[1].str();
        }
        return "";
    }
    
    // Find most common words in a set of texts
    std::vector<std::string> FindCommonWords(
        const std::vector<std::string>& texts,
        size_t max_words = 5
    ) {
        std::unordered_map<std::string, size_t> word_counts;
        
        for (const auto& text : texts) {
            std::unordered_set<std::string> seen_in_doc;
            std::string word;
            
            for (char c : text) {
                if (std::isalnum(static_cast<unsigned char>(c))) {
                    word += std::tolower(static_cast<unsigned char>(c));
                } else if (!word.empty()) {
                    if (word.length() > 3 && seen_in_doc.find(word) == seen_in_doc.end()) {
                        word_counts[word]++;
                        seen_in_doc.insert(word);
                    }
                    word.clear();
                }
            }
            
            if (!word.empty() && word.length() > 3 && 
                seen_in_doc.find(word) == seen_in_doc.end()) {
                word_counts[word]++;
            }
        }
        
        // Sort by count
        std::vector<std::pair<std::string, size_t>> sorted_words(
            word_counts.begin(), word_counts.end()
        );
        std::sort(sorted_words.begin(), sorted_words.end(),
                  [](const auto& a, const auto& b) { return a.second > b.second; });
        
        std::vector<std::string> result;
        for (size_t i = 0; i < std::min(max_words, sorted_words.size()); ++i) {
            result.push_back(sorted_words[i].first);
        }
        
        return result;
    }
};

GroupSuggester::GroupSuggester() : impl_(std::make_unique<Impl>()) {}
GroupSuggester::~GroupSuggester() = default;

std::vector<GroupSuggestion> GroupSuggester::SuggestByContent(
    const std::vector<PageContent>& pages,
    double similarity_threshold
) {
    std::vector<GroupSuggestion> suggestions;
    
    if (pages.empty()) {
        return suggestions;
    }
    
    // Simple clustering: group pages with high similarity
    std::vector<bool> assigned(pages.size(), false);
    
    for (size_t i = 0; i < pages.size(); ++i) {
        if (assigned[i]) continue;
        
        std::vector<size_t> group_indices;
        group_indices.push_back(i);
        assigned[i] = true;
        
        for (size_t j = i + 1; j < pages.size(); ++j) {
            if (assigned[j]) continue;
            
            double similarity = impl_->similarity_calculator.CalculateCosineSimilarity(
                pages[i].text, pages[j].text
            );
            
            if (similarity >= similarity_threshold) {
                group_indices.push_back(j);
                assigned[j] = true;
            }
        }
        
        // Only create group if more than one page
        if (group_indices.size() > 1) {
            GroupSuggestion suggestion;
            
            // Collect texts for naming
            std::vector<std::string> texts;
            for (size_t idx : group_indices) {
                suggestion.page_ids.push_back(std::to_string(idx));
                texts.push_back(pages[idx].text);
            }
            
            // Generate name from common words
            auto common_words = impl_->FindCommonWords(texts, 3);
            if (!common_words.empty()) {
                suggestion.group_name = common_words[0];
                for (size_t k = 1; k < common_words.size(); ++k) {
                    suggestion.group_name += " & " + common_words[k];
                }
            } else {
                suggestion.group_name = "Group " + std::to_string(suggestions.size() + 1);
            }
            
            suggestion.description = "Pages with similar content";
            suggestion.similarity_score = static_cast<float>(similarity_threshold);
            
            suggestions.push_back(suggestion);
        }
    }
    
    return suggestions;
}

std::vector<GroupSuggestion> GroupSuggester::SuggestByDomain(
    const std::vector<PageContent>& pages
) {
    std::vector<GroupSuggestion> suggestions;
    std::unordered_map<std::string, std::vector<size_t>> domain_groups;
    
    // Group pages by domain
    for (size_t i = 0; i < pages.size(); ++i) {
        // Try to extract domain from links or text
        std::string domain;
        if (!pages[i].links.empty()) {
            domain = impl_->ExtractDomain(pages[i].links[0]);
        }
        
        if (domain.empty()) {
            domain = "unknown";
        }
        
        domain_groups[domain].push_back(i);
    }
    
    // Create suggestions for domains with multiple pages
    for (const auto& pair : domain_groups) {
        if (pair.second.size() > 1) {
            GroupSuggestion suggestion;
            suggestion.group_name = pair.first;
            suggestion.description = "Pages from " + pair.first;
            suggestion.similarity_score = 1.0f;
            
            for (size_t idx : pair.second) {
                suggestion.page_ids.push_back(std::to_string(idx));
            }
            
            suggestions.push_back(suggestion);
        }
    }
    
    return suggestions;
}

std::vector<GroupSuggestion> GroupSuggester::SuggestByTopic(
    const std::vector<PageContent>& pages
) {
    std::vector<GroupSuggestion> suggestions;
    std::unordered_map<std::string, std::vector<size_t>> topic_groups;
    
    // Group pages by primary keyword/topic
    for (size_t i = 0; i < pages.size(); ++i) {
        std::string topic = "general";
        if (!pages[i].keywords.empty()) {
            topic = pages[i].keywords[0];
        }
        
        topic_groups[topic].push_back(i);
    }
    
    // Create suggestions for topics with multiple pages
    for (const auto& pair : topic_groups) {
        if (pair.second.size() > 1) {
            GroupSuggestion suggestion;
            suggestion.group_name = pair.first;
            suggestion.description = "Pages about " + pair.first;
            suggestion.similarity_score = 0.8f;
            
            for (size_t idx : pair.second) {
                suggestion.page_ids.push_back(std::to_string(idx));
            }
            
            suggestions.push_back(suggestion);
        }
    }
    
    return suggestions;
}

std::vector<GroupSuggestion> GroupSuggester::MergeGroups(
    const std::vector<GroupSuggestion>& groups,
    double merge_threshold
) {
    if (groups.size() <= 1) {
        return groups;
    }
    
    std::vector<GroupSuggestion> merged;
    std::vector<bool> processed(groups.size(), false);
    
    for (size_t i = 0; i < groups.size(); ++i) {
        if (processed[i]) continue;
        
        GroupSuggestion merged_group = groups[i];
        processed[i] = true;
        
        for (size_t j = i + 1; j < groups.size(); ++j) {
            if (processed[j]) continue;
            
            // Calculate overlap between groups
            std::unordered_set<std::string> set_a(
                merged_group.page_ids.begin(), merged_group.page_ids.end()
            );
            std::unordered_set<std::string> set_b(
                groups[j].page_ids.begin(), groups[j].page_ids.end()
            );
            
            size_t intersection = 0;
            for (const auto& id : set_a) {
                if (set_b.count(id)) intersection++;
            }
            
            size_t union_size = set_a.size() + set_b.size() - intersection;
            double overlap = union_size > 0 ? 
                static_cast<double>(intersection) / static_cast<double>(union_size) : 0.0;
            
            if (overlap >= merge_threshold) {
                // Merge groups
                for (const auto& id : groups[j].page_ids) {
                    if (set_a.find(id) == set_a.end()) {
                        merged_group.page_ids.push_back(id);
                    }
                }
                merged_group.similarity_score = std::min(
                    merged_group.similarity_score, groups[j].similarity_score
                );
                processed[j] = true;
            }
        }
        
        merged.push_back(merged_group);
    }
    
    return merged;
}

std::string GroupSuggester::GenerateGroupName(const std::vector<PageContent>& pages) {
    if (pages.empty()) {
        return "Empty Group";
    }
    
    std::vector<std::string> texts;
    for (const auto& page : pages) {
        texts.push_back(page.title + " " + page.text);
    }
    
    auto common_words = impl_->FindCommonWords(texts, 2);
    
    if (common_words.empty()) {
        return "Unnamed Group";
    }
    
    std::string name = common_words[0];
    if (common_words.size() > 1) {
        name += " " + common_words[1];
    }
    
    // Capitalize first letter
    if (!name.empty()) {
        name[0] = std::toupper(static_cast<unsigned char>(name[0]));
    }
    
    return name;
}

std::string GroupSuggester::GenerateGroupDescription(const std::vector<PageContent>& pages) {
    if (pages.empty()) {
        return "No pages in this group";
    }
    
    std::stringstream ss;
    ss << "A collection of " << pages.size() << " related pages";
    
    // Add common keywords if available
    std::vector<std::string> all_keywords;
    for (const auto& page : pages) {
        all_keywords.insert(all_keywords.end(), 
                           page.keywords.begin(), page.keywords.end());
    }
    
    if (!all_keywords.empty()) {
        auto common = impl_->FindCommonWords(
            std::vector<std::string>{all_keywords.begin(), all_keywords.end()}, 3
        );
        
        if (!common.empty()) {
            ss << " about ";
            for (size_t i = 0; i < common.size(); ++i) {
                if (i > 0) ss << ", ";
                ss << common[i];
            }
        }
    }
    
    return ss.str();
}

} // namespace ai
} // namespace web_page_manager
