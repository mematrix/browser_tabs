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

std::vector<GroupSuggestion> GroupSuggester::SuggestGroupsCombined(
    const std::vector<PageContent>& pages,
    double similarity_threshold
) {
    if (pages.empty()) {
        return {};
    }
    
    // Get suggestions from all methods
    auto content_groups = SuggestByContent(pages, similarity_threshold);
    auto domain_groups = SuggestByDomain(pages);
    auto topic_groups = SuggestByTopic(pages);
    
    // Combine all suggestions
    std::vector<GroupSuggestion> all_suggestions;
    all_suggestions.insert(all_suggestions.end(), content_groups.begin(), content_groups.end());
    all_suggestions.insert(all_suggestions.end(), domain_groups.begin(), domain_groups.end());
    all_suggestions.insert(all_suggestions.end(), topic_groups.begin(), topic_groups.end());
    
    // Merge overlapping groups
    auto merged = MergeGroups(all_suggestions, 0.5);
    
    // Rank and return
    return RankSuggestions(merged);
}

std::vector<CrossRecommendation> GroupSuggester::GenerateCrossRecommendations(
    const std::vector<PageContent>& pages,
    float min_relevance
) {
    std::vector<CrossRecommendation> recommendations;
    
    if (pages.size() < 2) {
        return recommendations;
    }
    
    // Calculate pairwise relevance
    for (size_t i = 0; i < pages.size(); ++i) {
        for (size_t j = i + 1; j < pages.size(); ++j) {
            // Calculate content similarity
            double text_sim = impl_->similarity_calculator.CalculateCombinedSimilarity(
                pages[i].text, pages[j].text
            );
            
            // Calculate keyword overlap
            double keyword_sim = impl_->similarity_calculator.CalculateJaccardSimilarity(
                pages[i].keywords, pages[j].keywords
            );
            
            // Combined relevance score
            float relevance = static_cast<float>(0.6 * text_sim + 0.4 * keyword_sim);
            
            if (relevance >= min_relevance) {
                CrossRecommendation rec;
                rec.source_id = std::to_string(i);
                rec.target_id = std::to_string(j);
                rec.relevance_score = relevance;
                
                // Find common topics/keywords
                for (const auto& kw_i : pages[i].keywords) {
                    for (const auto& kw_j : pages[j].keywords) {
                        if (kw_i == kw_j) {
                            rec.common_topics.push_back(kw_i);
                        }
                    }
                }
                
                // Generate reason
                if (!rec.common_topics.empty()) {
                    rec.reason = "Both pages discuss: " + rec.common_topics[0];
                    if (rec.common_topics.size() > 1) {
                        rec.reason += " and " + std::to_string(rec.common_topics.size() - 1) + " more topics";
                    }
                } else if (relevance > 0.7f) {
                    rec.reason = "Highly similar content";
                } else {
                    rec.reason = "Related content";
                }
                
                recommendations.push_back(rec);
            }
        }
    }
    
    // Sort by relevance score
    std::sort(recommendations.begin(), recommendations.end(),
              [](const CrossRecommendation& a, const CrossRecommendation& b) {
                  return a.relevance_score > b.relevance_score;
              });
    
    return recommendations;
}

std::vector<GroupSuggestion> GroupSuggester::RankSuggestions(
    const std::vector<GroupSuggestion>& suggestions
) {
    if (suggestions.empty()) {
        return suggestions;
    }
    
    // Create a copy for ranking
    std::vector<GroupSuggestion> ranked = suggestions;
    
    // Calculate quality score for each suggestion
    std::vector<std::pair<float, size_t>> scores;
    
    for (size_t i = 0; i < ranked.size(); ++i) {
        float quality = 0.0f;
        
        // Factor 1: Group size (prefer medium-sized groups)
        size_t size = ranked[i].page_ids.size();
        if (size >= 2 && size <= 5) {
            quality += 0.3f;
        } else if (size > 5 && size <= 10) {
            quality += 0.2f;
        } else if (size > 10) {
            quality += 0.1f;
        }
        
        // Factor 2: Similarity score
        quality += ranked[i].similarity_score * 0.4f;
        
        // Factor 3: Name quality (longer, more descriptive names are better)
        if (ranked[i].group_name.length() > 5) {
            quality += 0.15f;
        }
        if (ranked[i].group_name.find(" ") != std::string::npos) {
            quality += 0.1f;  // Multi-word names are more descriptive
        }
        
        // Factor 4: Has description
        if (!ranked[i].description.empty()) {
            quality += 0.05f;
        }
        
        scores.emplace_back(quality, i);
    }
    
    // Sort by quality score
    std::sort(scores.begin(), scores.end(),
              [](const auto& a, const auto& b) { return a.first > b.first; });
    
    // Reorder suggestions
    std::vector<GroupSuggestion> result;
    for (const auto& score : scores) {
        result.push_back(ranked[score.second]);
    }
    
    return result;
}

std::vector<GroupSuggestion> GroupSuggester::DetectClusters(
    const std::vector<PageContent>& pages,
    size_t num_clusters
) {
    std::vector<GroupSuggestion> clusters;
    
    if (pages.empty()) {
        return clusters;
    }
    
    // If num_clusters is 0, auto-detect based on page count
    if (num_clusters == 0) {
        num_clusters = std::max(size_t(2), pages.size() / 3);
        num_clusters = std::min(num_clusters, size_t(10));
    }
    
    // Simple hierarchical clustering using similarity matrix
    // Build similarity matrix
    std::vector<std::vector<double>> similarity_matrix(pages.size(), 
                                                        std::vector<double>(pages.size(), 0.0));
    
    for (size_t i = 0; i < pages.size(); ++i) {
        similarity_matrix[i][i] = 1.0;
        for (size_t j = i + 1; j < pages.size(); ++j) {
            double sim = impl_->similarity_calculator.CalculateCombinedSimilarity(
                pages[i].text, pages[j].text
            );
            similarity_matrix[i][j] = sim;
            similarity_matrix[j][i] = sim;
        }
    }
    
    // Initialize each page as its own cluster
    std::vector<std::vector<size_t>> current_clusters;
    for (size_t i = 0; i < pages.size(); ++i) {
        current_clusters.push_back({i});
    }
    
    // Agglomerative clustering: merge most similar clusters until we reach target
    while (current_clusters.size() > num_clusters && current_clusters.size() > 1) {
        // Find most similar pair of clusters
        double max_sim = -1.0;
        size_t merge_i = 0, merge_j = 1;
        
        for (size_t i = 0; i < current_clusters.size(); ++i) {
            for (size_t j = i + 1; j < current_clusters.size(); ++j) {
                // Calculate average linkage similarity
                double avg_sim = 0.0;
                size_t count = 0;
                
                for (size_t pi : current_clusters[i]) {
                    for (size_t pj : current_clusters[j]) {
                        avg_sim += similarity_matrix[pi][pj];
                        count++;
                    }
                }
                
                if (count > 0) {
                    avg_sim /= static_cast<double>(count);
                }
                
                if (avg_sim > max_sim) {
                    max_sim = avg_sim;
                    merge_i = i;
                    merge_j = j;
                }
            }
        }
        
        // Merge clusters
        current_clusters[merge_i].insert(
            current_clusters[merge_i].end(),
            current_clusters[merge_j].begin(),
            current_clusters[merge_j].end()
        );
        current_clusters.erase(current_clusters.begin() + static_cast<long>(merge_j));
    }
    
    // Convert clusters to GroupSuggestions
    for (size_t i = 0; i < current_clusters.size(); ++i) {
        if (current_clusters[i].size() < 2) {
            continue;  // Skip single-page clusters
        }
        
        GroupSuggestion suggestion;
        
        // Collect pages for this cluster
        std::vector<PageContent> cluster_pages;
        for (size_t idx : current_clusters[i]) {
            suggestion.page_ids.push_back(std::to_string(idx));
            cluster_pages.push_back(pages[idx]);
        }
        
        // Generate name and description
        suggestion.group_name = GenerateGroupName(cluster_pages);
        suggestion.description = GenerateGroupDescription(cluster_pages);
        
        // Calculate average similarity within cluster
        double avg_sim = 0.0;
        size_t count = 0;
        for (size_t pi : current_clusters[i]) {
            for (size_t pj : current_clusters[i]) {
                if (pi < pj) {
                    avg_sim += similarity_matrix[pi][pj];
                    count++;
                }
            }
        }
        suggestion.similarity_score = count > 0 ? 
            static_cast<float>(avg_sim / static_cast<double>(count)) : 0.5f;
        
        clusters.push_back(suggestion);
    }
    
    return RankSuggestions(clusters);
}

} // namespace ai
} // namespace web_page_manager
