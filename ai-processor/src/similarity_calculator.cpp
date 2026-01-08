#include "similarity_calculator.h"
#include <algorithm>
#include <cmath>
#include <sstream>
#include <unordered_set>
#include <cctype>
#include <numeric>

namespace web_page_manager {
namespace ai {

// Common stop words to filter out
static const std::unordered_set<std::string> STOP_WORDS = {
    "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
    "of", "with", "by", "from", "as", "is", "was", "are", "were", "been",
    "be", "have", "has", "had", "do", "does", "did", "will", "would", "could",
    "should", "may", "might", "must", "shall", "can", "need", "dare", "ought",
    "used", "this", "that", "these", "those", "i", "you", "he", "she", "it",
    "we", "they", "what", "which", "who", "whom", "whose", "where", "when",
    "why", "how", "all", "each", "every", "both", "few", "more", "most",
    "other", "some", "such", "no", "nor", "not", "only", "own", "same",
    "so", "than", "too", "very", "just", "also", "now", "here", "there"
};

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
                if (word.length() > 2 && STOP_WORDS.find(word) == STOP_WORDS.end()) {
                    tokens.push_back(word);
                }
                word.clear();
            }
        }
        
        if (!word.empty() && word.length() > 2 && STOP_WORDS.find(word) == STOP_WORDS.end()) {
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
    
    // Calculate n-grams for better similarity detection
    std::vector<std::string> CalculateNGrams(const std::string& text, size_t n = 2) {
        std::vector<std::string> ngrams;
        auto tokens = Tokenize(text);
        
        if (tokens.size() < n) {
            return tokens;
        }
        
        for (size_t i = 0; i <= tokens.size() - n; ++i) {
            std::string ngram;
            for (size_t j = 0; j < n; ++j) {
                if (j > 0) ngram += " ";
                ngram += tokens[i + j];
            }
            ngrams.push_back(ngram);
        }
        
        return ngrams;
    }
    
    // Calculate document vector magnitude
    double CalculateMagnitude(const std::unordered_map<std::string, double>& vec) {
        double sum = 0.0;
        for (const auto& pair : vec) {
            sum += pair.second * pair.second;
        }
        return std::sqrt(sum);
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
    // Calculate text similarity using cosine similarity
    double text_sim = CalculateCosineSimilarity(a.summary_text, b.summary_text);
    
    // Calculate key points similarity using Jaccard
    double keypoints_sim = CalculateJaccardSimilarity(a.key_points, b.key_points);
    
    // Content type match bonus
    double type_bonus = (a.content_type == b.content_type) ? 0.1 : 0.0;
    
    // Language match bonus
    double lang_bonus = (a.language == b.language) ? 0.05 : 0.0;
    
    // Reading time similarity (closer reading times suggest similar content length/complexity)
    double reading_time_sim = 0.0;
    if (a.reading_time_minutes > 0 && b.reading_time_minutes > 0) {
        double max_time = std::max(static_cast<double>(a.reading_time_minutes), 
                                   static_cast<double>(b.reading_time_minutes));
        double min_time = std::min(static_cast<double>(a.reading_time_minutes), 
                                   static_cast<double>(b.reading_time_minutes));
        reading_time_sim = (min_time / max_time) * 0.05;
    }
    
    // Weighted combination
    // Text similarity is most important, followed by key points
    double similarity = 0.55 * text_sim + 0.25 * keypoints_sim + type_bonus + lang_bonus + reading_time_sim;
    
    // Clamp to [0, 1]
    return std::min(1.0, std::max(0.0, similarity));
}

double SimilarityCalculator::CalculateNGramSimilarity(
    const std::string& text_a,
    const std::string& text_b,
    size_t n
) {
    auto ngrams_a = impl_->CalculateNGrams(text_a, n);
    auto ngrams_b = impl_->CalculateNGrams(text_b, n);
    
    if (ngrams_a.empty() && ngrams_b.empty()) {
        return 1.0;
    }
    
    if (ngrams_a.empty() || ngrams_b.empty()) {
        return 0.0;
    }
    
    std::unordered_set<std::string> set_a(ngrams_a.begin(), ngrams_a.end());
    std::unordered_set<std::string> set_b(ngrams_b.begin(), ngrams_b.end());
    
    size_t intersection = 0;
    for (const auto& ngram : set_a) {
        if (set_b.count(ngram)) {
            intersection++;
        }
    }
    
    size_t union_size = set_a.size() + set_b.size() - intersection;
    
    return static_cast<double>(intersection) / static_cast<double>(union_size);
}

double SimilarityCalculator::CalculateCombinedSimilarity(
    const std::string& text_a,
    const std::string& text_b
) {
    // Combine multiple similarity measures for more robust results
    double cosine_sim = CalculateCosineSimilarity(text_a, text_b);
    double bigram_sim = CalculateNGramSimilarity(text_a, text_b, 2);
    double trigram_sim = CalculateNGramSimilarity(text_a, text_b, 3);
    
    // Weighted combination
    return 0.5 * cosine_sim + 0.3 * bigram_sim + 0.2 * trigram_sim;
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
