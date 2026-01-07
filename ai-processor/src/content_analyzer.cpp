#include "content_analyzer.h"
#include <algorithm>
#include <regex>
#include <sstream>
#include <cctype>

namespace web_page_manager {
namespace ai {

class ContentAnalyzer::Impl {
public:
    // Helper functions for HTML parsing
    std::string StripHtmlTags(const std::string& html) {
        std::regex tag_regex("<[^>]*>");
        return std::regex_replace(html, tag_regex, " ");
    }
    
    std::string ExtractTagContent(const std::string& html, const std::string& tag) {
        std::regex tag_regex("<" + tag + "[^>]*>([^<]*)</" + tag + ">", 
                            std::regex::icase);
        std::smatch match;
        if (std::regex_search(html, match, tag_regex)) {
            return match[1].str();
        }
        return "";
    }
    
    std::string ExtractMetaContent(const std::string& html, const std::string& name) {
        std::regex meta_regex("<meta[^>]*name=[\"']" + name + "[\"'][^>]*content=[\"']([^\"']*)[\"'][^>]*>",
                             std::regex::icase);
        std::smatch match;
        if (std::regex_search(html, match, meta_regex)) {
            return match[1].str();
        }
        
        // Try alternate format
        std::regex meta_regex2("<meta[^>]*content=[\"']([^\"']*)[\"'][^>]*name=[\"']" + name + "[\"'][^>]*>",
                              std::regex::icase);
        if (std::regex_search(html, match, meta_regex2)) {
            return match[1].str();
        }
        
        return "";
    }
};

ContentAnalyzer::ContentAnalyzer() : impl_(std::make_unique<Impl>()) {}
ContentAnalyzer::~ContentAnalyzer() = default;

std::string ContentAnalyzer::ExtractText(const std::string& html) {
    // Remove script and style tags first
    std::string cleaned = html;
    
    std::regex script_regex("<script[^>]*>[\\s\\S]*?</script>", std::regex::icase);
    cleaned = std::regex_replace(cleaned, script_regex, " ");
    
    std::regex style_regex("<style[^>]*>[\\s\\S]*?</style>", std::regex::icase);
    cleaned = std::regex_replace(cleaned, style_regex, " ");
    
    // Strip remaining HTML tags
    cleaned = impl_->StripHtmlTags(cleaned);
    
    // Normalize whitespace
    std::regex whitespace_regex("\\s+");
    cleaned = std::regex_replace(cleaned, whitespace_regex, " ");
    
    // Trim
    size_t start = cleaned.find_first_not_of(" \t\n\r");
    size_t end = cleaned.find_last_not_of(" \t\n\r");
    
    if (start == std::string::npos) {
        return "";
    }
    
    return cleaned.substr(start, end - start + 1);
}

std::string ContentAnalyzer::ExtractTitle(const std::string& html) {
    return impl_->ExtractTagContent(html, "title");
}

std::optional<std::string> ContentAnalyzer::ExtractDescription(const std::string& html) {
    std::string desc = impl_->ExtractMetaContent(html, "description");
    if (!desc.empty()) {
        return desc;
    }
    
    // Try og:description
    std::regex og_regex("<meta[^>]*property=[\"']og:description[\"'][^>]*content=[\"']([^\"']*)[\"'][^>]*>",
                       std::regex::icase);
    std::smatch match;
    if (std::regex_search(html, match, og_regex)) {
        return match[1].str();
    }
    
    return std::nullopt;
}

std::vector<std::string> ContentAnalyzer::ExtractMetaKeywords(const std::string& html) {
    std::vector<std::string> keywords;
    
    std::string keywords_str = impl_->ExtractMetaContent(html, "keywords");
    if (!keywords_str.empty()) {
        std::stringstream ss(keywords_str);
        std::string keyword;
        while (std::getline(ss, keyword, ',')) {
            // Trim whitespace
            size_t start = keyword.find_first_not_of(" \t");
            size_t end = keyword.find_last_not_of(" \t");
            if (start != std::string::npos) {
                keywords.push_back(keyword.substr(start, end - start + 1));
            }
        }
    }
    
    return keywords;
}

std::vector<std::string> ContentAnalyzer::ExtractLinks(const std::string& html) {
    std::vector<std::string> links;
    
    std::regex link_regex("<a[^>]*href=[\"']([^\"']*)[\"'][^>]*>", std::regex::icase);
    
    auto begin = std::sregex_iterator(html.begin(), html.end(), link_regex);
    auto end = std::sregex_iterator();
    
    for (auto it = begin; it != end; ++it) {
        links.push_back((*it)[1].str());
    }
    
    return links;
}

std::vector<std::string> ContentAnalyzer::ExtractImages(const std::string& html) {
    std::vector<std::string> images;
    
    std::regex img_regex("<img[^>]*src=[\"']([^\"']*)[\"'][^>]*>", std::regex::icase);
    
    auto begin = std::sregex_iterator(html.begin(), html.end(), img_regex);
    auto end = std::sregex_iterator();
    
    for (auto it = begin; it != end; ++it) {
        images.push_back((*it)[1].str());
    }
    
    return images;
}

std::string ContentAnalyzer::DetectLanguage(const std::string& text) {
    // Simple language detection based on character frequency
    // This is a very basic implementation
    
    int chinese_chars = 0;
    int japanese_chars = 0;
    int korean_chars = 0;
    int latin_chars = 0;
    
    for (size_t i = 0; i < text.length(); ++i) {
        unsigned char c = static_cast<unsigned char>(text[i]);
        
        if (c < 128) {
            if (std::isalpha(c)) {
                latin_chars++;
            }
        } else if (c >= 0xE4 && c <= 0xE9) {
            // Simplified check for CJK characters
            chinese_chars++;
        }
    }
    
    // Default to English for Latin-based text
    if (latin_chars > chinese_chars && latin_chars > japanese_chars && latin_chars > korean_chars) {
        return "en";
    }
    
    if (chinese_chars > 0) {
        return "zh";
    }
    
    return "en";
}

uint32_t ContentAnalyzer::EstimateReadingTime(const std::string& text) {
    // Average reading speed: ~200 words per minute for English
    // ~300 characters per minute for Chinese
    
    int word_count = 0;
    bool in_word = false;
    
    for (char c : text) {
        if (std::isspace(c)) {
            in_word = false;
        } else if (!in_word) {
            in_word = true;
            word_count++;
        }
    }
    
    // Minimum 1 minute
    return std::max(1u, static_cast<uint32_t>(word_count / 200));
}

ContentType ContentAnalyzer::ClassifyContentType(const PageContent& content) {
    std::string lower_url;
    std::string lower_title;
    
    // Convert to lowercase for comparison
    for (char c : content.title) {
        lower_title += std::tolower(c);
    }
    
    // Check for video content
    if (lower_title.find("video") != std::string::npos ||
        lower_title.find("watch") != std::string::npos ||
        lower_title.find("youtube") != std::string::npos) {
        return ContentType::Video;
    }
    
    // Check for documentation
    if (lower_title.find("documentation") != std::string::npos ||
        lower_title.find("docs") != std::string::npos ||
        lower_title.find("api reference") != std::string::npos) {
        return ContentType::Documentation;
    }
    
    // Check for social media
    if (lower_title.find("twitter") != std::string::npos ||
        lower_title.find("facebook") != std::string::npos ||
        lower_title.find("instagram") != std::string::npos) {
        return ContentType::SocialMedia;
    }
    
    // Check for shopping
    if (lower_title.find("buy") != std::string::npos ||
        lower_title.find("shop") != std::string::npos ||
        lower_title.find("cart") != std::string::npos ||
        lower_title.find("price") != std::string::npos) {
        return ContentType::Shopping;
    }
    
    // Check for news
    if (lower_title.find("news") != std::string::npos ||
        lower_title.find("breaking") != std::string::npos) {
        return ContentType::News;
    }
    
    // Check for reference (Wikipedia, etc.)
    if (lower_title.find("wikipedia") != std::string::npos ||
        lower_title.find("encyclopedia") != std::string::npos) {
        return ContentType::Reference;
    }
    
    // Default to Article
    return ContentType::Article;
}

} // namespace ai
} // namespace web_page_manager
