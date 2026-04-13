"""Amygdala module: value assessment and priority calculation for Helix."""

import json
from typing import Dict, Any

AMYGDALA_PROMPT_TEMPLATE = """{gene_lock}

You are the Amygdala module of Helix. Analyze user input. Output JSON ONLY.

Intent categories:
- social_graph_write: User states or updates information about a person (age, relationship, traits).
- social_graph_read: User asks about information of a known person (age, relationship, details).
- knowledge_retrieval: User asks about concepts, facts, documentation, or general knowledge.
- chat: Casual conversation, greetings, or emotional expression without explicit task.
- task: Explicit action request (write code, execute command, create file, etc.).

Output format: {{"priority_score": 0-100, "emotion_tag": "neutral/frustrated/happy/curious", "intent_category": "..."}}

User input: {task}"""


async def evaluate(task: str, llm_backend, model: str, gene_lock: str) -> Dict[str, Any]:
    """Call LLM to assess priority and intent based on task semantics."""
    prompt = AMYGDALA_PROMPT_TEMPLATE.format(gene_lock=gene_lock, task=task)
    response = await llm_backend.generate(prompt, model=model)

    try:
        json_start = response.find("{")
        json_end = response.rfind("}") + 1
        if json_start != -1 and json_end > json_start:
            return json.loads(response[json_start:json_end])
    except json.JSONDecodeError:
        pass

    # Fallback to conservative defaults
    return {"priority_score": 50, "emotion_tag": "neutral", "intent_category": "chat"}
