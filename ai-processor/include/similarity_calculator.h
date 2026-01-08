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
    
    /// Calculate n-gram based similarity between two texts
    /// @param text_a First text
    /// @param text_b Second text
    /// @param n Size of n-grams (default: 2 for bigrams)
    /// @return Similarity score between 0 and 1
    double CalculateNGramSimilarity(
        const std::string& text_a,
        const std::string& text_b,
        size_t n = 2
    );
    
    /// Calculate combined similarity using multiple methods
    /// @param text_a First text
    /// @param text_b Second text
    /// @return Combined similarity score between 0 and 1
    double CalculateCombinedSimilarity(
        const std::string& text_a,
        const std::string& text_b
    );
    
private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace ai
} // namespace web_page_manager
