#include "content_analyzer.h"
#include <algorithm>
#include <regex>
#include <sstream>
#include <cctype>
#include <unordered_map>
#include <unordered_set>
#include <cmath>

namespace web_page_manager {
namespace ai {

// Common stop words to filter out during keyword extraction
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
    
    // Split text into sentences
    std::vector<std::string> SplitIntoSentences(const std::string& text) {
        std::vector<std::string> sentences;
        std::string current_sentence;
        
        for (size_t i = 0; i < text.length(); ++i) {
            char c = text[i];
            current_sentence += c;
            
            // Check for sentence endings
            if (c == '.' || c == '!' || c == '?') {
                // Make sure it's not an abbreviation (simple check)
                bool is_abbreviation = false;
                if (c == '.' && i > 0) {
                    // Check if previous char is uppercase (likely abbreviation)
                    size_t word_start = current_sentence.find_last_of(" \t\n", current_sentence.length() - 2);
                    if (word_start != std::string::npos) {
                        std::string last_word = current_sentence.substr(word_start + 1);
                        if (last_word.length() <= 3) {
                            is_abbreviation = true;
                        }
                    }
                }
                
                if (!is_abbreviation) {
                    // Trim whitespace
                    size_t start = current_sentence.find_first_not_of(" \t\n\r");
                    size_t end = current_sentence.find_last_not_of(" \t\n\r");
                    if (start != std::string::npos && end != std::string::npos) {
                        std::string trimmed = current_sentence.substr(start, end - start + 1);
                        if (trimmed.length() > 10) { // Minimum sentence length
                            sentences.push_back(trimmed);
                        }
                    }
                    current_sentence.clear();
                }
            }
        }
        
        // Add remaining text as last sentence if substantial
        if (!current_sentence.empty()) {
            size_t start = current_sentence.find_first_not_of(" \t\n\r");
            size_t end = current_sentence.find_last_not_of(" \t\n\r");
            if (start != std::string::npos && end != std::string::npos) {
                std::string trimmed = current_sentence.substr(start, end - start + 1);
                if (trimmed.length() > 10) {
                    sentences.push_back(trimmed);
                }
            }
        }
        
        return sentences;
    }
    
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
    
    // Calculate word frequency
    std::unordered_map<std::string, size_t> CalculateWordFrequency(const std::vector<std::string>& tokens) {
        std::unordered_map<std::string, size_t> freq;
        for (const auto& token : tokens) {
            freq[token]++;
        }
        return freq;
    }
    
    // Score a sentence based on word frequency
    double ScoreSentence(const std::string& sentence, 
                         const std::unordered_map<std::string, size_t>& word_freq,
                         size_t max_freq) {
        auto tokens = Tokenize(sentence);
        if (tokens.empty()) return 0.0;
        
        double score = 0.0;
        for (const auto& token : tokens) {
            auto it = word_freq.find(token);
            if (it != word_freq.end()) {
                score += static_cast<double>(it->second) / static_cast<double>(max_freq);
            }
        }
        
        // Normalize by sentence length (prefer medium-length sentences)
        double length_factor = 1.0;
        if (tokens.size() < 5) {
            length_factor = 0.5;
        } else if (tokens.size() > 30) {
            length_factor = 0.7;
        }
        
        return (score / static_cast<double>(tokens.size())) * length_factor;
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
    
    // Remove comments
    std::regex comment_regex("<!--[\\s\\S]*?-->", std::regex::icase);
    cleaned = std::regex_replace(cleaned, comment_regex, " ");
    
    // Strip remaining HTML tags
    cleaned = impl_->StripHtmlTags(cleaned);
    
    // Decode common HTML entities
    std::regex nbsp_regex("&nbsp;");
    cleaned = std::regex_replace(cleaned, nbsp_regex, " ");
    std::regex amp_regex("&amp;");
    cleaned = std::regex_replace(cleaned, amp_regex, "&");
    std::regex lt_regex("&lt;");
    cleaned = std::regex_replace(cleaned, lt_regex, "<");
    std::regex gt_regex("&gt;");
    cleaned = std::regex_replace(cleaned, gt_regex, ">");
    std::regex quot_regex("&quot;");
    cleaned = std::regex_replace(cleaned, quot_regex, "\"");
    
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
    // Language detection based on character frequency and common words
    
    int chinese_chars = 0;
    int japanese_chars = 0;
    int korean_chars = 0;
    int latin_chars = 0;
    int cyrillic_chars = 0;
    int arabic_chars = 0;
    
    for (size_t i = 0; i < text.length(); ++i) {
        unsigned char c = static_cast<unsigned char>(text[i]);
        
        if (c < 128) {
            if (std::isalpha(c)) {
                latin_chars++;
            }
        } else if (c >= 0xE4 && c <= 0xE9) {
            // CJK characters (simplified check)
            chinese_chars++;
        } else if (c >= 0xD0 && c <= 0xD3) {
            // Cyrillic characters
            cyrillic_chars++;
        } else if (c >= 0xD8 && c <= 0xDB) {
            // Arabic characters
            arabic_chars++;
        }
    }
    
    // Check for common language-specific words
    std::string lower_text;
    for (char c : text) {
        lower_text += std::tolower(static_cast<unsigned char>(c));
    }
    
    // Spanish indicators
    int spanish_score = 0;
    if (lower_text.find("que") != std::string::npos) spanish_score++;
    if (lower_text.find("para") != std::string::npos) spanish_score++;
    if (lower_text.find("como") != std::string::npos) spanish_score++;
    if (lower_text.find("pero") != std::string::npos) spanish_score++;
    
    // French indicators
    int french_score = 0;
    if (lower_text.find("que") != std::string::npos) french_score++;
    if (lower_text.find("pour") != std::string::npos) french_score++;
    if (lower_text.find("avec") != std::string::npos) french_score++;
    if (lower_text.find("dans") != std::string::npos) french_score++;
    
    // German indicators
    int german_score = 0;
    if (lower_text.find("und") != std::string::npos) german_score++;
    if (lower_text.find("der") != std::string::npos) german_score++;
    if (lower_text.find("die") != std::string::npos) german_score++;
    if (lower_text.find("das") != std::string::npos) german_score++;
    
    // Determine language based on character counts and word patterns
    if (chinese_chars > latin_chars) {
        return "zh";
    }
    
    if (cyrillic_chars > latin_chars) {
        return "ru";
    }
    
    if (arabic_chars > latin_chars) {
        return "ar";
    }
    
    // For Latin-based languages, use word patterns
    if (latin_chars > 0) {
        if (german_score >= 3) return "de";
        if (french_score >= 3) return "fr";
        if (spanish_score >= 3) return "es";
    }
    
    // Default to English
    return "en";
}

uint32_t ContentAnalyzer::EstimateReadingTime(const std::string& text) {
    // Average reading speed: ~200 words per minute for English
    // ~300 characters per minute for Chinese
    
    std::string lang = DetectLanguage(text);
    
    if (lang == "zh" || lang == "ja" || lang == "ko") {
        // Character-based languages
        size_t char_count = 0;
        for (size_t i = 0; i < text.length(); ++i) {
            unsigned char c = static_cast<unsigned char>(text[i]);
            if (c >= 128 || std::isalnum(c)) {
                char_count++;
            }
        }
        return std::max(1u, static_cast<uint32_t>(char_count / 300));
    }
    
    // Word-based languages
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
    std::string lower_text;
    
    // Convert to lowercase for comparison
    for (char c : content.title) {
        lower_title += std::tolower(static_cast<unsigned char>(c));
    }
    
    // Sample text for classification (first 1000 chars)
    std::string sample_text = content.text.substr(0, std::min(content.text.length(), size_t(1000)));
    for (char c : sample_text) {
        lower_text += std::tolower(static_cast<unsigned char>(c));
    }
    
    // Check for video content
    if (lower_title.find("video") != std::string::npos ||
        lower_title.find("watch") != std::string::npos ||
        lower_title.find("youtube") != std::string::npos ||
        lower_title.find("vimeo") != std::string::npos ||
        lower_title.find("twitch") != std::string::npos) {
        return ContentType::Video;
    }
    
    // Check for documentation
    if (lower_title.find("documentation") != std::string::npos ||
        lower_title.find("docs") != std::string::npos ||
        lower_title.find("api reference") != std::string::npos ||
        lower_title.find("manual") != std::string::npos ||
        lower_title.find("guide") != std::string::npos ||
        lower_text.find("function") != std::string::npos && 
        lower_text.find("parameter") != std::string::npos) {
        return ContentType::Documentation;
    }
    
    // Check for social media
    if (lower_title.find("twitter") != std::string::npos ||
        lower_title.find("facebook") != std::string::npos ||
        lower_title.find("instagram") != std::string::npos ||
        lower_title.find("linkedin") != std::string::npos ||
        lower_title.find("reddit") != std::string::npos ||
        lower_title.find("tweet") != std::string::npos) {
        return ContentType::SocialMedia;
    }
    
    // Check for shopping
    if (lower_title.find("buy") != std::string::npos ||
        lower_title.find("shop") != std::string::npos ||
        lower_title.find("cart") != std::string::npos ||
        lower_title.find("price") != std::string::npos ||
        lower_title.find("amazon") != std::string::npos ||
        lower_title.find("ebay") != std::string::npos ||
        lower_text.find("add to cart") != std::string::npos ||
        lower_text.find("checkout") != std::string::npos) {
        return ContentType::Shopping;
    }
    
    // Check for news
    if (lower_title.find("news") != std::string::npos ||
        lower_title.find("breaking") != std::string::npos ||
        lower_title.find("headline") != std::string::npos ||
        lower_title.find("report") != std::string::npos ||
        lower_text.find("reporter") != std::string::npos ||
        lower_text.find("journalist") != std::string::npos) {
        return ContentType::News;
    }
    
    // Check for reference (Wikipedia, etc.)
    if (lower_title.find("wikipedia") != std::string::npos ||
        lower_title.find("encyclopedia") != std::string::npos ||
        lower_title.find("dictionary") != std::string::npos ||
        lower_text.find("references") != std::string::npos &&
        lower_text.find("citation") != std::string::npos) {
        return ContentType::Reference;
    }
    
    // Default to Article
    return ContentType::Article;
}

std::string ContentAnalyzer::GenerateSummary(const std::string& text, size_t max_sentences) {
    if (text.empty()) {
        return "";
    }
    
    // Split into sentences
    auto sentences = impl_->SplitIntoSentences(text);
    
    if (sentences.empty()) {
        // If no sentences found, return truncated text
        if (text.length() <= 200) {
            return text;
        }
        return text.substr(0, 200) + "...";
    }
    
    if (sentences.size() <= max_sentences) {
        // Return all sentences if we have fewer than max
        std::string result;
        for (const auto& sentence : sentences) {
            if (!result.empty()) result += " ";
            result += sentence;
        }
        return result;
    }
    
    // Calculate word frequency across all text
    auto all_tokens = impl_->Tokenize(text);
    auto word_freq = impl_->CalculateWordFrequency(all_tokens);
    
    // Find max frequency
    size_t max_freq = 0;
    for (const auto& pair : word_freq) {
        max_freq = std::max(max_freq, pair.second);
    }
    
    if (max_freq == 0) max_freq = 1;
    
    // Score each sentence
    std::vector<std::pair<double, size_t>> scored_sentences;
    for (size_t i = 0; i < sentences.size(); ++i) {
        double score = impl_->ScoreSentence(sentences[i], word_freq, max_freq);
        // Boost score for sentences near the beginning (often contain key info)
        if (i < 3) {
            score *= 1.2;
        }
        scored_sentences.emplace_back(score, i);
    }
    
    // Sort by score (descending)
    std::sort(scored_sentences.begin(), scored_sentences.end(),
              [](const auto& a, const auto& b) { return a.first > b.first; });
    
    // Select top sentences and sort by original position
    std::vector<size_t> selected_indices;
    for (size_t i = 0; i < std::min(max_sentences, scored_sentences.size()); ++i) {
        selected_indices.push_back(scored_sentences[i].second);
    }
    std::sort(selected_indices.begin(), selected_indices.end());
    
    // Build summary
    std::string summary;
    for (size_t idx : selected_indices) {
        if (!summary.empty()) summary += " ";
        summary += sentences[idx];
    }
    
    return summary;
}

std::vector<std::string> ContentAnalyzer::ExtractKeywordsFromText(const std::string& text, size_t max_keywords) {
    if (text.empty()) {
        return {};
    }
    
    // Tokenize and count word frequency
    auto tokens = impl_->Tokenize(text);
    auto word_freq = impl_->CalculateWordFrequency(tokens);
    
    // Sort by frequency
    std::vector<std::pair<std::string, size_t>> sorted_words(word_freq.begin(), word_freq.end());
    std::sort(sorted_words.begin(), sorted_words.end(),
              [](const auto& a, const auto& b) { return a.second > b.second; });
    
    // Extract top keywords
    std::vector<std::string> keywords;
    for (size_t i = 0; i < std::min(max_keywords, sorted_words.size()); ++i) {
        // Only include words that appear more than once (unless we have few words)
        if (sorted_words[i].second > 1 || sorted_words.size() < max_keywords) {
            keywords.push_back(sorted_words[i].first);
        }
    }
    
    return keywords;
}

std::vector<std::string> ContentAnalyzer::ExtractKeyPoints(const std::string& text, size_t max_points) {
    if (text.empty()) {
        return {};
    }
    
    // Split into sentences
    auto sentences = impl_->SplitIntoSentences(text);
    
    if (sentences.empty()) {
        return {};
    }
    
    // Calculate word frequency
    auto all_tokens = impl_->Tokenize(text);
    auto word_freq = impl_->CalculateWordFrequency(all_tokens);
    
    size_t max_freq = 0;
    for (const auto& pair : word_freq) {
        max_freq = std::max(max_freq, pair.second);
    }
    if (max_freq == 0) max_freq = 1;
    
    // Score sentences
    std::vector<std::pair<double, std::string>> scored_sentences;
    for (const auto& sentence : sentences) {
        double score = impl_->ScoreSentence(sentence, word_freq, max_freq);
        scored_sentences.emplace_back(score, sentence);
    }
    
    // Sort by score
    std::sort(scored_sentences.begin(), scored_sentences.end(),
              [](const auto& a, const auto& b) { return a.first > b.first; });
    
    // Extract top key points
    std::vector<std::string> key_points;
    for (size_t i = 0; i < std::min(max_points, scored_sentences.size()); ++i) {
        // Truncate long sentences
        std::string point = scored_sentences[i].second;
        if (point.length() > 150) {
            point = point.substr(0, 147) + "...";
        }
        key_points.push_back(point);
    }
    
    return key_points;
}

PageStructure ContentAnalyzer::AnalyzePageStructure(const std::string& html) {
    PageStructure structure;
    structure.heading_count = 0;
    structure.paragraph_count = 0;
    structure.list_count = 0;
    structure.table_count = 0;
    structure.form_count = 0;
    structure.media_count = 0;
    structure.has_navigation = false;
    structure.has_sidebar = false;
    structure.has_footer = false;
    structure.content_density = 0.0f;
    
    if (html.empty()) {
        return structure;
    }
    
    // Count headings (h1-h6)
    for (int i = 1; i <= 6; ++i) {
        std::string tag = "h" + std::to_string(i);
        std::regex heading_regex("<" + tag + "[^>]*>", std::regex::icase);
        auto begin = std::sregex_iterator(html.begin(), html.end(), heading_regex);
        auto end = std::sregex_iterator();
        structure.heading_count += std::distance(begin, end);
    }
    
    // Extract headings text
    structure.headings = ExtractHeadings(html);
    
    // Count paragraphs
    std::regex p_regex("<p[^>]*>", std::regex::icase);
    auto p_begin = std::sregex_iterator(html.begin(), html.end(), p_regex);
    auto p_end = std::sregex_iterator();
    structure.paragraph_count = std::distance(p_begin, p_end);
    
    // Count lists (ul, ol)
    std::regex list_regex("<(ul|ol)[^>]*>", std::regex::icase);
    auto list_begin = std::sregex_iterator(html.begin(), html.end(), list_regex);
    auto list_end = std::sregex_iterator();
    structure.list_count = std::distance(list_begin, list_end);
    
    // Count tables
    std::regex table_regex("<table[^>]*>", std::regex::icase);
    auto table_begin = std::sregex_iterator(html.begin(), html.end(), table_regex);
    auto table_end = std::sregex_iterator();
    structure.table_count = std::distance(table_begin, table_end);
    
    // Count forms
    std::regex form_regex("<form[^>]*>", std::regex::icase);
    auto form_begin = std::sregex_iterator(html.begin(), html.end(), form_regex);
    auto form_end = std::sregex_iterator();
    structure.form_count = std::distance(form_begin, form_end);
    
    // Count media elements (img, video, audio)
    std::regex media_regex("<(img|video|audio)[^>]*>", std::regex::icase);
    auto media_begin = std::sregex_iterator(html.begin(), html.end(), media_regex);
    auto media_end = std::sregex_iterator();
    structure.media_count = std::distance(media_begin, media_end);
    
    // Check for navigation
    std::regex nav_regex("<nav[^>]*>|class=[\"'][^\"']*nav[^\"']*[\"']|id=[\"'][^\"']*nav[^\"']*[\"']", std::regex::icase);
    structure.has_navigation = std::regex_search(html, nav_regex);
    
    // Check for sidebar
    std::regex sidebar_regex("class=[\"'][^\"']*sidebar[^\"']*[\"']|id=[\"'][^\"']*sidebar[^\"']*[\"']|<aside[^>]*>", std::regex::icase);
    structure.has_sidebar = std::regex_search(html, sidebar_regex);
    
    // Check for footer
    std::regex footer_regex("<footer[^>]*>|class=[\"'][^\"']*footer[^\"']*[\"']|id=[\"'][^\"']*footer[^\"']*[\"']", std::regex::icase);
    structure.has_footer = std::regex_search(html, footer_regex);
    
    // Calculate content density
    std::string text = ExtractText(html);
    if (!html.empty()) {
        structure.content_density = static_cast<float>(text.length()) / static_cast<float>(html.length());
    }
    
    // Extract sections based on headings
    for (const auto& heading : structure.headings) {
        if (!heading.empty()) {
            structure.sections.push_back(heading);
        }
    }
    
    return structure;
}

std::vector<std::string> ContentAnalyzer::ExtractHeadings(const std::string& html) {
    std::vector<std::string> headings;
    
    // Match h1-h6 tags and extract content
    std::regex heading_regex("<h([1-6])[^>]*>([^<]*)</h\\1>", std::regex::icase);
    
    auto begin = std::sregex_iterator(html.begin(), html.end(), heading_regex);
    auto end = std::sregex_iterator();
    
    for (auto it = begin; it != end; ++it) {
        std::string heading_text = (*it)[2].str();
        // Trim whitespace
        size_t start = heading_text.find_first_not_of(" \t\n\r");
        size_t end_pos = heading_text.find_last_not_of(" \t\n\r");
        if (start != std::string::npos && end_pos != std::string::npos) {
            headings.push_back(heading_text.substr(start, end_pos - start + 1));
        }
    }
    
    return headings;
}

std::vector<EntityInfo> ContentAnalyzer::ExtractEntities(const std::string& text) {
    std::vector<EntityInfo> entities;
    
    if (text.empty()) {
        return entities;
    }
    
    // Simple entity extraction using pattern matching
    // In a production system, this would use NER models
    
    // Extract potential person names (capitalized word sequences)
    std::regex name_regex("\\b([A-Z][a-z]+(?:\\s+[A-Z][a-z]+)+)\\b");
    auto name_begin = std::sregex_iterator(text.begin(), text.end(), name_regex);
    auto name_end = std::sregex_iterator();
    
    std::unordered_map<std::string, EntityInfo> entity_map;
    
    for (auto it = name_begin; it != name_end; ++it) {
        std::string name = (*it)[1].str();
        size_t pos = it->position();
        
        if (entity_map.find(name) == entity_map.end()) {
            EntityInfo info;
            info.name = name;
            info.type = "person";
            info.confidence = 0.6f;
            info.positions.push_back(pos);
            entity_map[name] = info;
        } else {
            entity_map[name].positions.push_back(pos);
            // Increase confidence with more occurrences
            entity_map[name].confidence = std::min(0.95f, entity_map[name].confidence + 0.1f);
        }
    }
    
    // Extract potential organizations (words ending in Inc, Corp, Ltd, etc.)
    std::regex org_regex("\\b([A-Z][A-Za-z]*(?:\\s+[A-Z][A-Za-z]*)*\\s+(?:Inc|Corp|Ltd|LLC|Company|Corporation|Foundation|Institute|University))\\b");
    auto org_begin = std::sregex_iterator(text.begin(), text.end(), org_regex);
    auto org_end = std::sregex_iterator();
    
    for (auto it = org_begin; it != org_end; ++it) {
        std::string org = (*it)[1].str();
        size_t pos = it->position();
        
        if (entity_map.find(org) == entity_map.end()) {
            EntityInfo info;
            info.name = org;
            info.type = "organization";
            info.confidence = 0.75f;
            info.positions.push_back(pos);
            entity_map[org] = info;
        } else {
            entity_map[org].type = "organization";  // Override type
            entity_map[org].positions.push_back(pos);
            entity_map[org].confidence = std::min(0.95f, entity_map[org].confidence + 0.1f);
        }
    }
    
    // Extract URLs as potential product/service entities
    std::regex url_regex("https?://([a-zA-Z0-9.-]+)");
    auto url_begin = std::sregex_iterator(text.begin(), text.end(), url_regex);
    auto url_end = std::sregex_iterator();
    
    for (auto it = url_begin; it != url_end; ++it) {
        std::string domain = (*it)[1].str();
        size_t pos = it->position();
        
        if (entity_map.find(domain) == entity_map.end()) {
            EntityInfo info;
            info.name = domain;
            info.type = "website";
            info.confidence = 0.9f;
            info.positions.push_back(pos);
            entity_map[domain] = info;
        }
    }
    
    // Convert map to vector
    for (const auto& pair : entity_map) {
        entities.push_back(pair.second);
    }
    
    // Sort by confidence
    std::sort(entities.begin(), entities.end(),
              [](const EntityInfo& a, const EntityInfo& b) {
                  return a.confidence > b.confidence;
              });
    
    return entities;
}

std::pair<std::string, float> ContentAnalyzer::AnalyzeSentiment(const std::string& text) {
    if (text.empty()) {
        return {"neutral", 0.0f};
    }
    
    // Simple lexicon-based sentiment analysis
    // Positive words
    static const std::unordered_set<std::string> positive_words = {
        "good", "great", "excellent", "amazing", "wonderful", "fantastic",
        "awesome", "best", "love", "happy", "beautiful", "perfect",
        "brilliant", "outstanding", "superb", "incredible", "positive",
        "success", "successful", "win", "winner", "benefit", "helpful",
        "easy", "simple", "fast", "efficient", "effective", "recommend",
        "like", "enjoy", "pleased", "satisfied", "impressive", "innovative"
    };
    
    // Negative words
    static const std::unordered_set<std::string> negative_words = {
        "bad", "terrible", "awful", "horrible", "worst", "hate", "poor",
        "disappointing", "disappointed", "fail", "failure", "problem",
        "issue", "bug", "error", "wrong", "broken", "slow", "difficult",
        "hard", "complicated", "confusing", "frustrating", "annoying",
        "useless", "waste", "expensive", "overpriced", "scam", "fake",
        "never", "cannot", "impossible", "unfortunately", "sadly"
    };
    
    // Tokenize and count sentiment words
    auto tokens = impl_->Tokenize(text);
    
    int positive_count = 0;
    int negative_count = 0;
    
    for (const auto& token : tokens) {
        if (positive_words.count(token)) {
            positive_count++;
        }
        if (negative_words.count(token)) {
            negative_count++;
        }
    }
    
    // Calculate sentiment score
    int total_sentiment_words = positive_count + negative_count;
    float score = 0.0f;
    
    if (total_sentiment_words > 0) {
        score = static_cast<float>(positive_count - negative_count) / 
                static_cast<float>(total_sentiment_words);
    }
    
    // Determine sentiment label
    std::string label;
    if (score > 0.3f) {
        label = "positive";
    } else if (score < -0.3f) {
        label = "negative";
    } else {
        label = "neutral";
    }
    
    return {label, score};
}

std::vector<std::string> ContentAnalyzer::ExtractTopics(const std::string& text, size_t max_topics) {
    if (text.empty()) {
        return {};
    }
    
    // Extract keywords as potential topics
    auto keywords = ExtractKeywordsFromText(text, max_topics * 2);
    
    // Filter and cluster keywords into topics
    std::vector<std::string> topics;
    std::unordered_set<std::string> seen;
    
    for (const auto& keyword : keywords) {
        // Skip very short keywords
        if (keyword.length() < 4) continue;
        
        // Check if similar topic already exists
        bool is_duplicate = false;
        for (const auto& topic : topics) {
            // Simple substring check for similarity
            if (topic.find(keyword) != std::string::npos ||
                keyword.find(topic) != std::string::npos) {
                is_duplicate = true;
                break;
            }
        }
        
        if (!is_duplicate && seen.find(keyword) == seen.end()) {
            topics.push_back(keyword);
            seen.insert(keyword);
            
            if (topics.size() >= max_topics) {
                break;
            }
        }
    }
    
    return topics;
}

} // namespace ai
} // namespace web_page_manager
