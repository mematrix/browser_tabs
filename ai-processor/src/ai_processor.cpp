#include "ai_processor.h"
#include "content_analyzer.h"
#include "similarity_calculator.h"
#include "group_suggester.h"
#include <algorithm>
#include <numeric>

namespace web_page_manager {
namespace ai {

/// Implementation class for AIContentProcessor
class AIContentProcessor::Impl {
public:
    Impl() 
        : mode_(ProcessingMode::Auto)
        , content_analyzer_(std::make_unique<ContentAnalyzer>())
        , similarity_calculator_(std::make_unique<SimilarityCalculator>())
        , group_suggester_(std::make_unique<GroupSuggester>())
    {}
    
    ProcessingMode mode_;
    std::unique_ptr<ContentAnalyzer> content_analyzer_;
    std::unique_ptr<SimilarityCalculator> similarity_calculator_;
    std::unique_ptr<GroupSuggester> group_suggester_;
};

AIContentProcessor::AIContentProcessor()
    : impl_(std::make_unique<Impl>())
{}

AIContentProcessor::~AIContentProcessor() = default;

AIContentProcessor::AIContentProcessor(AIContentProcessor&&) noexcept = default;
AIContentProcessor& AIContentProcessor::operator=(AIContentProcessor&&) noexcept = default;

ContentSummary AIContentProcessor::GenerateSummary(const PageContent& content) {
    ContentSummary summary;
    
    // Extract key sentences for summary
    // Simple implementation: take first few sentences
    std::string text = content.text;
    if (text.length() > 500) {
        text = text.substr(0, 500) + "...";
    }
    summary.summary_text = text;
    
    // Extract key points (simple: split by sentences)
    // TODO: Implement more sophisticated key point extraction
    summary.key_points = {"Key point 1", "Key point 2", "Key point 3"};
    
    // Classify content type
    summary.content_type = impl_->content_analyzer_->ClassifyContentType(content);
    
    // Detect language
    summary.language = impl_->content_analyzer_->DetectLanguage(content.text);
    
    // Estimate reading time
    summary.reading_time_minutes = impl_->content_analyzer_->EstimateReadingTime(content.text);
    
    // Set confidence score
    summary.confidence_score = 0.85f;
    
    return summary;
}

std::vector<std::string> AIContentProcessor::ExtractKeywords(const PageContent& content) {
    std::vector<std::string> keywords;
    
    // Start with meta keywords if available
    keywords = content.keywords;
    
    // TODO: Implement TF-IDF based keyword extraction
    // For now, return existing keywords or extract from title
    if (keywords.empty() && !content.title.empty()) {
        // Simple word extraction from title
        std::string word;
        for (char c : content.title) {
            if (std::isalnum(c)) {
                word += std::tolower(c);
            } else if (!word.empty()) {
                if (word.length() > 3) {
                    keywords.push_back(word);
                }
                word.clear();
            }
        }
        if (!word.empty() && word.length() > 3) {
            keywords.push_back(word);
        }
    }
    
    return keywords;
}

CategoryInfo AIContentProcessor::ClassifyContent(const PageContent& content) {
    CategoryInfo info;
    
    // Simple classification based on content type
    ContentType type = impl_->content_analyzer_->ClassifyContentType(content);
    
    switch (type) {
        case ContentType::Article:
            info.primary_category = "Articles";
            info.secondary_categories = {"Reading", "Information"};
            break;
        case ContentType::Video:
            info.primary_category = "Media";
            info.secondary_categories = {"Video", "Entertainment"};
            break;
        case ContentType::Documentation:
            info.primary_category = "Documentation";
            info.secondary_categories = {"Reference", "Technical"};
            break;
        case ContentType::SocialMedia:
            info.primary_category = "Social";
            info.secondary_categories = {"Social Media", "Communication"};
            break;
        case ContentType::Shopping:
            info.primary_category = "Shopping";
            info.secondary_categories = {"E-commerce", "Products"};
            break;
        case ContentType::News:
            info.primary_category = "News";
            info.secondary_categories = {"Current Events", "Information"};
            break;
        case ContentType::Reference:
            info.primary_category = "Reference";
            info.secondary_categories = {"Knowledge", "Information"};
            break;
        default:
            info.primary_category = "Other";
            info.secondary_categories = {};
            break;
    }
    
    info.confidence = 0.75f;
    return info;
}

double AIContentProcessor::CalculateSimilarity(const ContentSummary& a, const ContentSummary& b) {
    return impl_->similarity_calculator_->CalculateSummarySimilarity(a, b);
}

std::vector<GroupSuggestion> AIContentProcessor::SuggestGroups(const std::vector<PageContent>& pages) {
    return impl_->group_suggester_->SuggestByContent(pages);
}

RelevanceScore AIContentProcessor::CalculateContentRelevance(const PageContent& a, const PageContent& b) {
    RelevanceScore score;
    
    // Calculate text similarity
    double text_sim = impl_->similarity_calculator_->CalculateCosineSimilarity(a.text, b.text);
    
    // Calculate keyword similarity
    double keyword_sim = impl_->similarity_calculator_->CalculateJaccardSimilarity(a.keywords, b.keywords);
    
    // Combined score
    score.score = static_cast<float>(0.7 * text_sim + 0.3 * keyword_sim);
    
    // Find common keywords
    for (const auto& kw_a : a.keywords) {
        for (const auto& kw_b : b.keywords) {
            if (kw_a == kw_b) {
                score.common_keywords.push_back(kw_a);
            }
        }
    }
    
    return score;
}

ContentAnalysis AIContentProcessor::AnalyzePageStructure(const PageContent& content) {
    ContentAnalysis analysis;
    
    analysis.summary = GenerateSummary(content);
    analysis.category = ClassifyContent(content);
    
    // TODO: Implement entity extraction
    analysis.entities = {};
    
    // TODO: Implement topic extraction
    analysis.topics = {};
    
    // TODO: Implement sentiment analysis
    analysis.sentiment = "neutral";
    
    return analysis;
}

std::vector<std::string> AIContentProcessor::ExtractPageMetadata(const PageContent& content) {
    std::vector<std::string> metadata;
    
    metadata.push_back("title:" + content.title);
    
    if (content.description) {
        metadata.push_back("description:" + *content.description);
    }
    
    for (const auto& keyword : content.keywords) {
        metadata.push_back("keyword:" + keyword);
    }
    
    metadata.push_back("image_count:" + std::to_string(content.images.size()));
    metadata.push_back("link_count:" + std::to_string(content.links.size()));
    
    return metadata;
}

TopicInfo AIContentProcessor::IdentifyMainTopics(const PageContent& content) {
    TopicInfo info;
    
    // Simple topic identification based on keywords
    if (!content.keywords.empty()) {
        info.main_topic = content.keywords[0];
        
        for (size_t i = 1; i < content.keywords.size() && i < 5; ++i) {
            info.sub_topics.push_back(content.keywords[i]);
        }
    } else {
        info.main_topic = "General";
    }
    
    info.confidence = 0.7f;
    return info;
}

void AIContentProcessor::SetProcessingMode(ProcessingMode mode) {
    impl_->mode_ = mode;
}

ProcessingMode AIContentProcessor::GetProcessingMode() const {
    return impl_->mode_;
}

ProcessingCapabilities AIContentProcessor::GetCurrentCapabilities() const {
    ProcessingCapabilities caps;
    
    caps.supports_enhanced_mode = true;
    caps.supports_media_analysis = false; // Not yet implemented
    caps.supports_sentiment_analysis = false; // Not yet implemented
    caps.max_content_length = 1024 * 1024; // 1MB
    caps.supported_languages = {"en", "zh", "ja", "ko", "es", "fr", "de"};
    
    return caps;
}

} // namespace ai
} // namespace web_page_manager
