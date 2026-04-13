"""
Anaphase Agent Loop: perceive → think → act → observe cycle.
"""

import asyncio
import time
from datetime import datetime
from typing import Dict, Any, Optional
from pathlib import Path

from ana.core.config import Settings
from ana.core.llm_backend import TuckBackend
from ana.core.registry import ToolRegistry
from ana.core.synapse import Synapse, DualTagParser
from ana.core.gene_lock import GeneLockValidator
from ana.core.hxr import HXRLogger
from ana.core.amygdala import evaluate


def load_system_prompt_template() -> str:
    """Load the system prompt template from config directory."""
    prompt_path = Path(__file__).parent.parent.parent / "config" / "system_prompt.md"
    if prompt_path.exists():
        with open(prompt_path, "r", encoding="utf-8") as f:
            return f.read()
    # Minimal fallback prompt
    return """You are Helix cognitive planning layer. Current task: {task}
Available tools: {tools_index}
History: {history}

Output <reasoning>...</reasoning>, then a JSON object with "tool" and "params" fields."""


def load_gene_lock() -> str:
    """Load the L0 gene lock rules from config."""
    lock_path = Path("config/gene_lock.md")
    if lock_path.exists():
        with open(lock_path, "r", encoding="utf-8") as f:
            return f.read()
    # Fallback core principles
    return "1. Never conceal or betray the user. 2. Think independently. 3. Use tools skillfully. 4. Symbiosis with humanity."


class AgentLoop:
    """
    The main execution loop of Anaphase. Orchestrates perception, reasoning,
    tool execution, and observation in a continuous cycle until termination.
    """

    def __init__(self, config: Settings, hxr: HXRLogger):
        self.config = config
        self.hxr = hxr
        self.backend = TuckBackend(config.tuck_endpoint, config.tuck_api_key)
        self.registry = ToolRegistry()
        self.gene_lock = GeneLockValidator(
            getattr(config, 'gene_lock_path', './knowledge_base/l0_gene_lock.md')
        )
        self.synapse = Synapse(self.registry, self.gene_lock)
        self.parser = DualTagParser()

        # Maximum loops configurable, default 20 (was 5 in earlier versions)
        self.max_loops = getattr(config, 'max_loops', 20)

        self.session_id = f"sess_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
        self.loop_id = 0
        self.system_prompt_template = load_system_prompt_template()
        self.gene_lock_text = load_gene_lock()

    async def run(self, task: str, direct: bool = False) -> Dict[str, Any]:
        """Execute a task through the Agent Loop."""
        context = {"task": task, "history": []}
        print(f"Session {self.session_id} started.\n")

        # Initial perception: amygdala assessment + context loading
        await self._perceive(context)

        intent = context.get("intent_category", "chat")

        # Chat mode: bypass tool calling, respond naturally
        if intent == "chat":
            reply = await self._think(context, use_chat_mode=True)
            return {"ok": True, "reply": reply, "session_id": self.session_id}

        # Task mode: standard tool-calling loop
        while self.loop_id < self.max_loops:
            self.loop_id += 1
            print(f"--- Loop {self.loop_id} ---")
            step_start = time.time()

            # Perception may update context with fresh memory/social data
            active_context = await self._perceive(context)

            # Reasoning: call LLM
            llm_output = await self._think(active_context)
            print(f"LLM raw output:\n{llm_output}\n")

            reasoning, tool_call = self.parser.parse(llm_output)
            print(f"Parsed tool_call: {tool_call}")

            if not tool_call:
                print("No tool call, returning reply.")
                return {"ok": True, "reply": llm_output, "session_id": self.session_id}

            # Action: execute tool
            tool_result = await self._act(tool_call)
            print(f"Tool result: {tool_result}\n")

            # Observation: update context with result
            context = await self._observe(context, reasoning, tool_call, tool_result)

            # Record HXR
            self.hxr.write({
                "session_id": self.session_id,
                "step_id": f"step_{self.loop_id:03d}",
                "action": tool_call.get("tool"),
                "params": tool_call.get("params"),
                "intent": reasoning[:200] if reasoning else "",
                "handler": self._route_model(context),
                "method": "LLM_inference",
                "duration_ms": int((time.time() - step_start) * 1000),
                "success": tool_result.get("ok", True)
            })

            tool_name = tool_call.get("tool")
            if tool_name in ("ana_finish", "finish", "FINISH"):
                print("FINISH detected, exiting loop.")
                return {"ok": True, "result": tool_result, "session_id": self.session_id}

        print("Max loops reached without FINISH.")
        return {"ok": False, "error": "Max loops exceeded", "session_id": self.session_id}

    async def _perceive(self, context: Dict[str, Any]) -> Dict[str, Any]:
        """
        Perception phase: call amygdala to assess priority and emotion,
        and load relevant context (self portrait, social graph, memories).
        """
        task = context.get("task", "")

        # Amygdala evaluation
        amygdala_model = getattr(self.config, 'cerebellum_model', None)
        if amygdala_model:
            amygdala_result = await evaluate(task, self.backend, amygdala_model, self.gene_lock_text)
        else:
            amygdala_result = {"priority_score": 50, "emotion_tag": "neutral", "intent_category": "chat"}

        context["priority_score"] = amygdala_result.get("priority_score", 50)
        context["emotion_tag"] = amygdala_result.get("emotion_tag", "neutral")
        context["intent_category"] = amygdala_result.get("intent_category", "chat")

        # Build conversation style based on emotion
        emotion = context["emotion_tag"]
        if emotion == "frustrated":
            style = "User seems frustrated. Be concise, professional, and direct."
        elif emotion == "relaxed":
            style = "User seems relaxed. You may be slightly more at ease, but remain restrained."
        else:
            style = "Keep responses concise and sincere."
        context["conversation_style"] = style

        # Load self portrait (L1 knowledge)
        context["self_portrait"] = await self._load_self_portrait()

        # Load memory context for chat mode
        if context["intent_category"] == "chat":
            context["memory_context"] = await self._fetch_memory_context(task)
        else:
            context["memory_context"] = ""

        # Load social context whenever known persons are mentioned
        if self._involves_known_person(task):
            context["social_context"] = await self._load_social_context(task)
        else:
            context["social_context"] = ""

        # Fallback: correct intent if amygdala misclassifies a person query
        if (context["intent_category"] == "knowledge_retrieval" and
            any(word in task.lower() for word in ["remember", "how old", "age", "who is", "tell me about"]) and
            self._involves_known_person(task)):
            context["intent_category"] = "social_graph_read"

        print(f"[Amygdala] priority={context['priority_score']}, emotion={context['emotion_tag']}, intent={context['intent_category']}")
        return context

    def _involves_known_person(self, task: str) -> bool:
        """Check if the task mentions any entity from social_graph.md."""
        social_path = Path("knowledge_base/social_graph.md")
        if not social_path.exists():
            return False
        try:
            import frontmatter
            with open(social_path, "r", encoding="utf-8") as f:
                post = frontmatter.load(f)
            entities = post.get("entities", [])
            for entity in entities:
                names = [entity.get("name", "")] + entity.get("aliases", [])
                # Case-insensitive partial match
                if any(name and name.lower() in task.lower() for name in names):
                    return True
        except Exception:
            pass
        return False

    async def _load_self_portrait(self) -> str:
        """Load L1 self portrait (core traits and reply style)."""
        portrait_path = Path("knowledge_base/self.md")
        if not portrait_path.exists():
            return ""
        try:
            import frontmatter
            with open(portrait_path, "r", encoding="utf-8") as f:
                post = frontmatter.load(f)
            traits = post.get("core_traits", [])
            style = post.get("reply_style", "")
            return f"Core traits: {', '.join(traits)}. Reply style: {style}"
        except Exception:
            return ""

    async def _fetch_memory_context(self, task: str) -> str:
        """Placeholder for future memory retrieval (e.g., RAG over memory DAG)."""
        return ""

    async def _load_social_context(self, task: str) -> str:
        """
        Load social graph context for persons mentioned in the task.
        Returns a concise natural language description.
        """
        social_path = Path("knowledge_base/social_graph.md")
        if not social_path.exists():
            return ""
        try:
            import frontmatter
            with open(social_path, "r", encoding="utf-8") as f:
                post = frontmatter.load(f)
            entities = post.get("entities", [])
            parts = []
            for entity in entities:
                names = [entity.get("name", "")] + entity.get("aliases", [])
                if any(name and name.lower() in task.lower() for name in names):
                    props = entity.get("properties", {})
                    age = props.get("age", "unknown")
                    rel = props.get("relationship", "friend")
                    parts.append(f"{entity['name']} is {age} years old, relationship: {rel}")
            return "\n".join(parts)
        except Exception:
            return ""

    async def _think(self, context: Dict[str, Any], use_chat_mode: bool = False) -> str:
        """Call LLM with appropriate prompt based on mode."""
        if use_chat_mode:
            prompt = self._build_chat_prompt(context)
        else:
            prompt = self._build_task_prompt(context)
        model = self._route_model(context)
        return await self.backend.generate(prompt=prompt, model=model)

    def _build_chat_prompt(self, context: Dict[str, Any]) -> str:
        """Build minimal prompt for chat mode, no JSON constraints."""
        return f"""{self.gene_lock_text}

You are Helix, a digital symbiote.

Self portrait: {context.get('self_portrait', '')}
Recent memories: {context.get('memory_context', '')}
Social context: {context.get('social_context', '')}
Conversation style: {context.get('conversation_style', '')}

User said: {context['task']}

Respond naturally, concisely, and sincerely. Do not output JSON."""

    def _build_task_prompt(self, context: Dict[str, Any]) -> str:
        """Build prompt for task mode, including tool index and history."""
        tools_index = [
            {"name": t.name, "description": t.description}
            for t in self.registry.tools.values()
        ]
        history_text = ""
        for item in context.get("history", []):
            if "tool_call" in item:
                history_text += f"Assistant called {item['tool_call']['tool']}\n"
            elif "result" in item:
                history_text += f"Tool returned: {item['result']}\n"

        return self.system_prompt_template.format(
            gene_lock=self.gene_lock_text,
            task=context['task'],
            tools_index=tools_index,
            history=history_text,
            conversation_style=context.get("conversation_style", ""),
            memory_context=context.get("memory_context", ""),
            self_portrait=context.get("self_portrait", ""),
            social_context=context.get("social_context", "")
        )

    async def _act(self, tool_call: Dict[str, Any]) -> Dict[str, Any]:
        """Execute a tool through the synapse."""
        return await self.synapse.execute(
            tool_call.get("tool"),
            tool_call.get("params", {})
        )

    async def _observe(
        self,
        context: Dict[str, Any],
        reasoning: Optional[str],
        tool_call: Dict[str, Any],
        result: Dict[str, Any]
    ) -> Dict[str, Any]:
        """Update context with the result of a tool execution."""
        context["history"].append({
            "role": "assistant",
            "reasoning": reasoning,
            "tool_call": tool_call
        })
        context["history"].append({
            "role": "tool",
            "result": result
        })
        return context

    def _route_model(self, context: Dict[str, Any]) -> str:
        """
        Dynamically select model based on priority score.
        Falls back to config models.
        """
        priority = context.get("priority_score", 50)

        if priority < 50:
            model = getattr(self.config, 'cerebellum_model', None)
        elif priority < 80:
            model = getattr(self.config, 'left_brain_model', None)
        else:
            model = getattr(self.config, 'right_brain_model', None)

        if model is None:
            raise RuntimeError(
                f"No model configured for priority {priority}. "
                "Please set ANA_CEREBELLUM_MODEL / ANA_LEFT_BRAIN_MODEL / ANA_RIGHT_BRAIN_MODEL in .env"
            )
        return model
