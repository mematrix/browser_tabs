#pragma once

#include "ai_processor.h"
#include <string>
#include <vector>
#include <unordered_map>

namespace web_page_manager {
namespace ai {

/// Calculator for content similarity
class SimilarityCalculator {
public:
    SimilarityCalculator();
    ~SimilarityCalculator();
    
    /// Calculate cosine similarity between two text documents
    double CalculateCosineSimilarity(const std::string& text_a, const std::string& text_b);
    
    /// Calculate Jaccard similarity between two keyword sets
    double CalculateJaccardSimilarity(
        const std::vector<std::string>& keywords_a,
        const std::vector<std::string>& keywords_b
    );
    
    /// Calculate overall similarity between two content summaries
    double CalculateSummarySimilarity(const ContentSummary& a, const ContentSummary& b);
    
    /// Calculate TF-IDF vectors for a document
    std::unordered_map<std::string, double> CalculateTfIdf(
        const std::string& document,
        const std::vector<std::string>& corpus
    );
    
    /// Find similar documents in a corpus
    std::vector<std::pair<size_t, double>> FindSimilarDocuments(
        const std::string& query,
        const std::vector<std::string>& corpus,
        double threshold = 0.5
    );
    
private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace ai
} // namespace web_page_manager
