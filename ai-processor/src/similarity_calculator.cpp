#include "similarity_calculator.h"
#include <algorithm>
#include <cmath>
#include <sstream>
#include <unordered_set>
#include <cctype>

namespace web_page_manager {
namespace ai {

class SimilarityCalculator::Impl {
public:
    // Tokenize text into words
    std::vector<std::string> Tokenize(const std::string& text) {
        std::vector<std::string> tokens;
        std::string word;
        
        for (char c : text) {
            if (std::isalnum(static_cast<unsigned char>(c))) {
                word += std::tolower(static_cast<unsigned char>(c));
            } else if (!word.empty()) {
                if (word.length() > 2) { // Skip very short words
                    tokens.push_back(word);
                }
                word.clear();
            }
        }
        
        if (!word.empty() && word.length() > 2) {
            tokens.push_back(word);
        }
        
        return tokens;
    }
    
    // Calculate term frequency
    std::unordered_map<std::string, double> CalculateTermFrequency(
        const std::vector<std::string>& tokens
    ) {
        std::unordered_map<std::string, double> tf;
        
        for (const auto& token : tokens) {
            tf[token] += 1.0;
        }
        
        // Normalize by document length
        double doc_length = static_cast<double>(tokens.size());
        if (doc_length > 0) {
            for (auto& pair : tf) {
                pair.second /= doc_length;
            }
        }
        
        return tf;
    }
};

SimilarityCalculator::SimilarityCalculator() : impl_(std::make_unique<Impl>()) {}
SimilarityCalculator::~SimilarityCalculator() = default;

double SimilarityCalculator::CalculateCosineSimilarity(
    const std::string& text_a,
    const std::string& text_b
) {
    auto tokens_a = impl_->Tokenize(text_a);
    auto tokens_b = impl_->Tokenize(text_b);
    
    if (tokens_a.empty() || tokens_b.empty()) {
        return 0.0;
    }
    
    auto tf_a = impl_->CalculateTermFrequency(tokens_a);
    auto tf_b = impl_->CalculateTermFrequency(tokens_b);
    
    // Calculate dot product and magnitudes
    double dot_product = 0.0;
    double magnitude_a = 0.0;
    double magnitude_b = 0.0;
    
    // Collect all unique terms
    std::unordered_set<std::string> all_terms;
    for (const auto& pair : tf_a) all_terms.insert(pair.first);
    for (const auto& pair : tf_b) all_terms.insert(pair.first);
    
    for (const auto& term : all_terms) {
        double val_a = tf_a.count(term) ? tf_a[term] : 0.0;
        double val_b = tf_b.count(term) ? tf_b[term] : 0.0;
        
        dot_product += val_a * val_b;
        magnitude_a += val_a * val_a;
        magnitude_b += val_b * val_b;
    }
    
    magnitude_a = std::sqrt(magnitude_a);
    magnitude_b = std::sqrt(magnitude_b);
    
    if (magnitude_a == 0.0 || magnitude_b == 0.0) {
        return 0.0;
    }
    
    return dot_product / (magnitude_a * magnitude_b);
}

double SimilarityCalculator::CalculateJaccardSimilarity(
    const std::vector<std::string>& keywords_a,
    const std::vector<std::string>& keywords_b
) {
    if (keywords_a.empty() && keywords_b.empty()) {
        return 1.0; // Both empty = identical
    }
    
    if (keywords_a.empty() || keywords_b.empty()) {
        return 0.0;
    }
    
    std::unordered_set<std::string> set_a(keywords_a.begin(), keywords_a.end());
    std::unordered_set<std::string> set_b(keywords_b.begin(), keywords_b.end());
    
    size_t intersection_size = 0;
    for (const auto& keyword : set_a) {
        if (set_b.count(keyword)) {
            intersection_size++;
        }
    }
    
    size_t union_size = set_a.size() + set_b.size() - intersection_size;
    
    if (union_size == 0) {
        return 0.0;
    }
    
    return static_cast<double>(intersection_size) / static_cast<double>(union_size);
}

double SimilarityCalculator::CalculateSummarySimilarity(
    const ContentSummary& a,
    const ContentSummary& b
) {
    // Calculate text similarity
    double text_sim = CalculateCosineSimilarity(a.summary_text, b.summary_text);
    
    // Calculate key points similarity
    double keypoints_sim = CalculateJaccardSimilarity(a.key_points, b.key_points);
    
    // Content type match bonus
    double type_bonus = (a.content_type == b.content_type) ? 0.1 : 0.0;
    
    // Language match bonus
    double lang_bonus = (a.language == b.language) ? 0.05 : 0.0;
    
    // Weighted combination
    double similarity = 0.6 * text_sim + 0.25 * keypoints_sim + type_bonus + lang_bonus;
    
    // Clamp to [0, 1]
    return std::min(1.0, std::max(0.0, similarity));
}

std::unordered_map<std::string, double> SimilarityCalculator::CalculateTfIdf(
    const std::string& document,
    const std::vector<std::string>& corpus
) {
    auto tokens = impl_->Tokenize(document);
    auto tf = impl_->CalculateTermFrequency(tokens);
    
    // Calculate IDF for each term
    std::unordered_map<std::string, double> tfidf;
    size_t corpus_size = corpus.size() + 1; // +1 for the document itself
    
    for (const auto& pair : tf) {
        const std::string& term = pair.first;
        double term_tf = pair.second;
        
        // Count documents containing this term
        size_t doc_count = 1; // The document itself
        for (const auto& doc : corpus) {
            auto doc_tokens = impl_->Tokenize(doc);
            for (const auto& token : doc_tokens) {
                if (token == term) {
                    doc_count++;
                    break;
                }
            }
        }
        
        // Calculate IDF
        double idf = std::log(static_cast<double>(corpus_size) / static_cast<double>(doc_count));
        
        tfidf[term] = term_tf * idf;
    }
    
    return tfidf;
}

std::vector<std::pair<size_t, double>> SimilarityCalculator::FindSimilarDocuments(
    const std::string& query,
    const std::vector<std::string>& corpus,
    double threshold
) {
    std::vector<std::pair<size_t, double>> results;
    
    for (size_t i = 0; i < corpus.size(); ++i) {
        double similarity = CalculateCosineSimilarity(query, corpus[i]);
        if (similarity >= threshold) {
            results.emplace_back(i, similarity);
        }
    }
    
    // Sort by similarity (descending)
    std::sort(results.begin(), results.end(),
              [](const auto& a, const auto& b) { return a.second > b.second; });
    
    return results;
}

} // namespace ai
} // namespace web_page_manager
