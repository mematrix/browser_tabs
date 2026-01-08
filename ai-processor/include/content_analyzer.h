#pragma once

#include "ai_processor.h"
#include <string>
#include <vector>

namespace web_page_manager {
namespace ai {

/// Content analyzer for extracting information from web pages
class ContentAnalyzer {
public:
    ContentAnalyzer();
    ~ContentAnalyzer();
    
    /// Extract text content from HTML
    std::string ExtractText(const std::string& html);
    
    /// Extract title from HTML
    std::string ExtractTitle(const std::string& html);
    
    /// Extract description from HTML meta tags
    std::optional<std::string> ExtractDescription(const std::string& html);
    
    /// Extract keywords from HTML meta tags
    std::vector<std::string> ExtractMetaKeywords(const std::string& html);
    
    /// Extract all links from HTML
    std::vector<std::string> ExtractLinks(const std::string& html);
    
    /// Extract all image URLs from HTML
    std::vector<std::string> ExtractImages(const std::string& html);
    
    /// Detect the language of the content
    std::string DetectLanguage(const std::string& text);
    
    /// Estimate reading time in minutes
    uint32_t EstimateReadingTime(const std::string& text);
    
    /// Classify content type based on structure and content
    ContentType ClassifyContentType(const PageContent& content);
    
    /// Generate a summary of the text using extractive summarization
    /// @param text The input text to summarize
    /// @param max_sentences Maximum number of sentences in the summary (default: 3)
    /// @return A summary string containing the most important sentences
    std::string GenerateSummary(const std::string& text, size_t max_sentences = 3);
    
    /// Extract keywords from text using TF-based analysis
    /// @param text The input text to analyze
    /// @param max_keywords Maximum number of keywords to extract (default: 10)
    /// @return A vector of keywords sorted by importance
    std::vector<std::string> ExtractKeywordsFromText(const std::string& text, size_t max_keywords = 10);
    
    /// Extract key points from text
    /// @param text The input text to analyze
    /// @param max_points Maximum number of key points to extract (default: 5)
    /// @return A vector of key point sentences
    std::vector<std::string> ExtractKeyPoints(const std::string& text, size_t max_points = 5);
    
    /// Analyze page structure from HTML
    /// @param html The HTML content to analyze
    /// @return PageStructure containing structural information
    PageStructure AnalyzePageStructure(const std::string& html);
    
    /// Extract headings from HTML
    /// @param html The HTML content
    /// @return Vector of heading texts (h1-h6)
    std::vector<std::string> ExtractHeadings(const std::string& html);
    
    /// Extract named entities from text
    /// @param text The text to analyze
    /// @return Vector of EntityInfo with detected entities
    std::vector<EntityInfo> ExtractEntities(const std::string& text);
    
    /// Analyze sentiment of text
    /// @param text The text to analyze
    /// @return Pair of sentiment label and score (-1.0 to 1.0)
    std::pair<std::string, float> AnalyzeSentiment(const std::string& text);
    
    /// Extract topics from text using keyword clustering
    /// @param text The text to analyze
    /// @param max_topics Maximum number of topics to extract
    /// @return Vector of topic strings
    std::vector<std::string> ExtractTopics(const std::string& text, size_t max_topics = 5);
    
private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace ai
} // namespace web_page_manager
