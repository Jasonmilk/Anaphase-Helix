#!/usr/bin/env python3
"""
Super Analyst Framework Selection Utility

This script helps identify the most appropriate analytical framework(s)
based on problem characteristics.
"""

import sys
import json

# Framework database with selection criteria
FRAMEWORKS = {
    "first_principles": {
        "name": "First Principles Thinking",
        "prompt_title": "First Principles Thinking to analyze problems",
        "keywords": ["innovation", "breakthrough", "redesign", "fundamental", "from scratch", "new approach"],
        "problem_types": ["innovation", "complex_decision", "unconventional"],
        "complexity": "high",
        "time": "long"
    },
    "5_whys": {
        "name": "5 Whys",
        "prompt_title": "5 Whys method to analyze problems",
        "keywords": ["why", "root cause", "failure", "bug", "issue", "problem", "repeatedly"],
        "problem_types": ["diagnosis", "quality", "operational"],
        "complexity": "low",
        "time": "short"
    },
    "swot": {
        "name": "SWOT Analysis",
        "prompt_title": "SWOT to analyze problems",
        "keywords": ["strengths", "weaknesses", "opportunities", "threats", "strategy", "competitive"],
        "problem_types": ["strategy", "planning", "assessment"],
        "complexity": "medium",
        "time": "medium"
    },
    "porter": {
        "name": "Porter's Five Forces",
        "prompt_title": "Porter's Five Forces model to analyze problems",
        "keywords": ["industry", "competition", "market entry", "competitive", "sector"],
        "problem_types": ["industry_analysis", "competitive_strategy"],
        "complexity": "medium-high",
        "time": "medium"
    },
    "cost_benefit": {
        "name": "Cost-Benefit Analysis",
        "prompt_title": "Cost-Benefit Analysis to analyze problems",
        "keywords": ["cost", "benefit", "roi", "investment", "financial", "budget"],
        "problem_types": ["financial_decision", "project_evaluation"],
        "complexity": "medium",
        "time": "medium"
    },
    "design_thinking": {
        "name": "Design Thinking",
        "prompt_title": "Design Thinking to analyze problems",
        "keywords": ["user", "customer", "experience", "prototype", "iterate", "empathy"],
        "problem_types": ["innovation", "user_problem", "product_development"],
        "complexity": "high",
        "time": "long"
    },
    "systems_thinking": {
        "name": "Systems Thinking",
        "prompt_title": "Systems Thinking to analyze problems",
        "keywords": ["system", "complex", "interconnected", "ripple effects", "long-term"],
        "problem_types": ["complex_system", "policy", "organizational"],
        "complexity": "high",
        "time": "long"
    },
    "socratic": {
        "name": "Socratic Method",
        "prompt_title": "Socratic Method to analyze problems",
        "keywords": ["understand", "explore", "question", "clarify", "assumptions", "debate"],
        "problem_types": ["learning", "critical_analysis", "ethical"],
        "complexity": "medium",
        "time": "medium"
    },
    "pareto": {
        "name": "Pareto Analysis",
        "prompt_title": "Pareto Analysis to analyze problems",
        "keywords": ["prioritize", "80/20", "most important", "focus", "efficiency"],
        "problem_types": ["prioritization", "resource_allocation", "optimization"],
        "complexity": "low-medium",
        "time": "short"
    },
    "mece": {
        "name": "MECE Principle",
        "prompt_title": "MECE Principle to analyze problems",
        "keywords": ["structure", "organize", "categories", "breakdown", "segments"],
        "problem_types": ["problem_decomposition", "consulting", "analysis"],
        "complexity": "medium",
        "time": "short-medium"
    },
    "hypothesis": {
        "name": "Hypothesis-Driven Analysis",
        "prompt_title": "Hypothesis-Driven Analysis to analyze problems",
        "keywords": ["test", "hypothesis", "validate", "experiment", "data", "research"],
        "problem_types": ["research", "testing", "validation"],
        "complexity": "medium-high",
        "time": "medium"
    },
    "scenario": {
        "name": "Scenario Planning",
        "prompt_title": "Scenario Planning to analyze problems",
        "keywords": ["future", "uncertainty", "what if", "scenarios", "risk", "planning"],
        "problem_types": ["strategic_planning", "risk_management", "uncertainty"],
        "complexity": "high",
        "time": "long"
    }
}


def analyze_text(text):
    """Analyze text to identify relevant frameworks."""
    text_lower = text.lower()
    scores = {}
    
    for fw_id, fw_data in FRAMEWORKS.items():
        score = 0
        # Check for keyword matches
        for keyword in fw_data["keywords"]:
            if keyword in text_lower:
                score += 1
        scores[fw_id] = score
    
    return scores


def recommend_frameworks(problem_text, top_n=3):
    """Recommend top N frameworks based on problem description."""
    scores = analyze_text(problem_text)
    
    # Sort by score
    sorted_frameworks = sorted(scores.items(), key=lambda x: x[1], reverse=True)
    
    recommendations = []
    for fw_id, score in sorted_frameworks[:top_n]:
        if score > 0:  # Only include frameworks with matches
            fw_data = FRAMEWORKS[fw_id]
            recommendations.append({
                "id": fw_id,
                "name": fw_data["name"],
                "prompt_title": fw_data["prompt_title"],
                "score": score,
                "complexity": fw_data["complexity"],
                "time": fw_data["time"],
                "available": True  # All frameworks are now available
            })
    
    return recommendations


def main():
    if len(sys.argv) < 2:
        print("Usage: python framework_selector.py '<problem description>'")
        print("\nExample:")
        print("  python framework_selector.py 'How can we innovate our product development process?'")
        sys.exit(1)
    
    problem = " ".join(sys.argv[1:])
    recommendations = recommend_frameworks(problem)
    
    if not recommendations:
        print("No strong framework matches found.")
        print("Consider using First Principles Thinking or SWOT Analysis as general-purpose frameworks.")
        print("All 12 frameworks are available in your prompt library!")
    else:
        print("Recommended Frameworks:\n")
        for i, rec in enumerate(recommendations, 1):
            status = "Available" if rec["available"] else "Not in library"
            print(f"{i}. {rec['name']}")
            print(f"   Prompt: {rec['prompt_title']}")
            print(f"   Match Score: {rec['score']} | Complexity: {rec['complexity']} | Time: {rec['time']}")
            print(f"   Status: {status}")
            print()


if __name__ == "__main__":
    main()
