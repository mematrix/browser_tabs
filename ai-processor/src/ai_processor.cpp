#include "ai_processor.h"
#include "content_analyzer.h"
#include "similarity_calculator.h"
#include "group_suggester.h"
#include <algorithm>
#include <numeric>
#include <unordered_set>

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
    
    // Use the content analyzer to generate an extractive summary
    std::string text_to_summarize = content.text;
    if (text_to_summarize.empty() && !content.html.empty()) {
        text_to_summarize = impl_->content_analyzer_->ExtractText(content.html);
    }
    
    // Generate summary using extractive summarization
    summary.summary_text = impl_->content_analyzer_->GenerateSummary(text_to_summarize, 3);
    
    // If summary is empty, fall back to description or truncated text
    if (summary.summary_text.empty()) {
        if (content.description && !content.description->empty()) {
            summary.summary_text = *content.description;
        } else if (!text_to_summarize.empty()) {
            // Truncate to first 300 characters
            if (text_to_summarize.length() > 300) {
                summary.summary_text = text_to_summarize.substr(0, 297) + "...";
            } else {
                summary.summary_text = text_to_summarize;
            }
        }
    }
    
    // Extract key points
    summary.key_points = impl_->content_analyzer_->ExtractKeyPoints(text_to_summarize, 5);
    
    // Classify content type
    summary.content_type = impl_->content_analyzer_->ClassifyContentType(content);
    
    // Detect language
    summary.language = impl_->content_analyzer_->DetectLanguage(text_to_summarize);
    
    // Estimate reading time
    summary.reading_time_minutes = impl_->content_analyzer_->EstimateReadingTime(text_to_summarize);
    
    // Calculate confidence score based on content quality
    float confidence = 0.5f;
    
    // Boost confidence if we have good content
    if (!summary.summary_text.empty()) confidence += 0.15f;
    if (!summary.key_points.empty()) confidence += 0.1f;
    if (!content.title.empty()) confidence += 0.1f;
    if (content.description && !content.description->empty()) confidence += 0.1f;
    if (text_to_summarize.length() > 500) confidence += 0.05f;
    
    summary.confidence_score = std::min(0.95f, confidence);
    
    return summary;
}

std::vector<std::string> AIContentProcessor::ExtractKeywords(const PageContent& content) {
    std::vector<std::string> keywords;
    
    // Start with meta keywords if available
    keywords = content.keywords;
    
    // Extract keywords from text content
    std::string text_to_analyze = content.text;
    if (text_to_analyze.empty() && !content.html.empty()) {
        text_to_analyze = impl_->content_analyzer_->ExtractText(content.html);
    }
    
    // Use the content analyzer to extract keywords from text
    auto extracted_keywords = impl_->content_analyzer_->ExtractKeywordsFromText(text_to_analyze, 15);
    
    // Merge with existing keywords, avoiding duplicates
    std::unordered_set<std::string> keyword_set(keywords.begin(), keywords.end());
    for (const auto& kw : extracted_keywords) {
        if (keyword_set.find(kw) == keyword_set.end()) {
            keywords.push_back(kw);
            keyword_set.insert(kw);
        }
    }
    
    // Also extract keywords from title
    if (!content.title.empty()) {
        auto title_keywords = impl_->content_analyzer_->ExtractKeywordsFromText(content.title, 5);
        for (const auto& kw : title_keywords) {
            if (keyword_set.find(kw) == keyword_set.end()) {
                // Title keywords are important, add them at the beginning
                keywords.insert(keywords.begin(), kw);
                keyword_set.insert(kw);
            }
        }
    }
    
    // Limit to top 20 keywords
    if (keywords.size() > 20) {
        keywords.resize(20);
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
    
    // Extract text for analysis
    std::string text_to_analyze = content.text;
    if (text_to_analyze.empty() && !content.html.empty()) {
        text_to_analyze = impl_->content_analyzer_->ExtractText(content.html);
    }
    
    // Extract entities
    auto detailed_entities = impl_->content_analyzer_->ExtractEntities(text_to_analyze);
    analysis.detailed_entities = detailed_entities;
    
    // Convert to simple string list for backward compatibility
    for (const auto& entity : detailed_entities) {
        analysis.entities.push_back(entity.name);
    }
    
    // Extract topics
    analysis.topics = impl_->content_analyzer_->ExtractTopics(text_to_analyze, 5);
    
    // Analyze sentiment
    auto [sentiment_label, sentiment_score] = impl_->content_analyzer_->AnalyzeSentiment(text_to_analyze);
    analysis.sentiment = sentiment_label;
    analysis.sentiment_score = sentiment_score;
    
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
    
    // Extract text for analysis
    std::string text_to_analyze = content.text;
    if (text_to_analyze.empty() && !content.html.empty()) {
        text_to_analyze = impl_->content_analyzer_->ExtractText(content.html);
    }
    
    // Extract topics using content analyzer
    auto topics = impl_->content_analyzer_->ExtractTopics(text_to_analyze, 6);
    
    if (!topics.empty()) {
        info.main_topic = topics[0];
        
        for (size_t i = 1; i < topics.size(); ++i) {
            info.sub_topics.push_back(topics[i]);
        }
        info.confidence = 0.75f;
    } else if (!content.keywords.empty()) {
        // Fall back to keywords
        info.main_topic = content.keywords[0];
        
        for (size_t i = 1; i < content.keywords.size() && i < 5; ++i) {
            info.sub_topics.push_back(content.keywords[i]);
        }
        info.confidence = 0.6f;
    } else {
        info.main_topic = "General";
        info.confidence = 0.3f;
    }
    
    return info;
}

PageStructure AIContentProcessor::AnalyzePageLayout(const PageContent& content) {
    return impl_->content_analyzer_->AnalyzePageStructure(content.html);
}

std::vector<EntityInfo> AIContentProcessor::ExtractEntities(const PageContent& content) {
    std::string text_to_analyze = content.text;
    if (text_to_analyze.empty() && !content.html.empty()) {
        text_to_analyze = impl_->content_analyzer_->ExtractText(content.html);
    }
    return impl_->content_analyzer_->ExtractEntities(text_to_analyze);
}

std::pair<std::string, float> AIContentProcessor::AnalyzeSentiment(const std::string& text) {
    return impl_->content_analyzer_->AnalyzeSentiment(text);
}

std::vector<CrossRecommendation> AIContentProcessor::GenerateCrossRecommendations(
    const std::vector<PageContent>& pages,
    float min_relevance
) {
    return impl_->group_suggester_->GenerateCrossRecommendations(pages, min_relevance);
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
