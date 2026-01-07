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
    
private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace ai
} // namespace web_page_manager
