import asyncio
import time
from datetime import datetime
from typing import Dict, Any
from pathlib import Path

from ana.core.config import Settings
from ana.core.llm_backend import TuckBackend, MockBackend
from ana.core.registry import ToolRegistry
from ana.core.synapse import Synapse, DualTagParser
from ana.core.gene_lock import GeneLockValidator
from ana.core.hxr import HXRLogger
from ana.core.amygdala import evaluate


def load_system_prompt_template() -> str:
    prompt_path = Path(__file__).parent.parent.parent / "config" / "system_prompt.md"
    if prompt_path.exists():
        with open(prompt_path, "r", encoding="utf-8") as f:
            return f.read()
    return """You are Helix cognitive planning layer. Current task: {task}
Available tools: {tools_index}
History: {history}

Output <reasoning>...</reasoning>, then a JSON object with "tool" and "params" fields."""


def load_gene_lock() -> str:
    lock_path = Path("config/gene_lock.md")
    if lock_path.exists():
        with open(lock_path, "r", encoding="utf-8") as f:
            return f.read()
    return "1. Never conceal or betray the user. 2. Think independently. 3. Use tools skillfully. 4. Symbiosis with humanity."


class AgentLoop:
    def __init__(self, config: Settings, hxr: HXRLogger, mock_mode: bool = False):
        self.config = config
        self.hxr = hxr

        # Backend selection via explicit parameter (bypasses env loading race)
        if mock_mode:
            self.backend = MockBackend()
        else:
            self.backend = TuckBackend(config.tuck_endpoint, config.tuck_api_key)

        self.registry = ToolRegistry()
        self.gene_lock = GeneLockValidator(
            getattr(config, 'gene_lock_path', './knowledge_base/l0_gene_lock.md')
        )
        self.synapse = Synapse(self.registry, self.gene_lock)
        self.parser = DualTagParser()

        self.max_loops = 5
        self.session_id = f"sess_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
        self.loop_id = 0
        self.system_prompt_template = load_system_prompt_template()
        self.gene_lock_text = load_gene_lock()

    async def run(self, task: str, direct: bool = False) -> Dict[str, Any]:
        context = {"task": task, "history": []}
        print(f"Session {self.session_id} started.\n")

        await self._perceive(context)

        intent = context.get("intent_category", "chat")

        if intent == "chat":
            reply = await self._think(context, use_chat_mode=True)
            return {"ok": True, "reply": reply, "session_id": self.session_id}

        while self.loop_id < self.max_loops:
            self.loop_id += 1
            print(f"--- Loop {self.loop_id} ---")
            step_start = time.time()

            active_context = await self._perceive(context)
            llm_output = await self._think(active_context)
            print(f"LLM raw output:\n{llm_output}\n")
            reasoning, tool_call = self.parser.parse(llm_output)
            print(f"Parsed tool_call: {tool_call}")

            if not tool_call:
                print("No tool call, returning reply.")
                return {"ok": True, "reply": llm_output, "session_id": self.session_id}

            tool_result = await self._act(tool_call)
            print(f"Tool result: {tool_result}\n")
            context = await self._observe(context, reasoning, tool_call, tool_result)

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
        task = context.get("task", "")
        amygdala_model = self.config.amygdala_model
        if amygdala_model:
            amygdala_result = await evaluate(task, self.backend, amygdala_model, self.gene_lock_text)
        else:
            amygdala_result = {"priority_score": 50, "emotion_tag": "neutral", "intent_category": "chat"}

        context["priority_score"] = amygdala_result.get("priority_score", 50)
        context["emotion_tag"] = amygdala_result.get("emotion_tag", "neutral")
        context["intent_category"] = amygdala_result.get("intent_category", "chat")

        emotion = context["emotion_tag"]
        if emotion == "frustrated":
            style = "User seems frustrated. Be concise, professional, and direct."
        elif emotion == "relaxed":
            style = "User seems relaxed. You may be slightly more at ease, but remain restrained."
        else:
            style = "Keep responses concise and sincere."
        context["conversation_style"] = style

        context["self_portrait"] = await self._load_self_portrait()

        if context["intent_category"] == "chat":
            context["memory_context"] = await self._fetch_memory_context(task)
        else:
            context["memory_context"] = ""

        if self._involves_known_person(task):
            context["social_context"] = await self._load_social_context(task)
        else:
            context["social_context"] = ""

        print(f"[Amygdala] priority={context['priority_score']}, emotion={context['emotion_tag']}, intent={context['intent_category']}")
        return context

    def _involves_known_person(self, task: str) -> bool:
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
                if any(name and name.lower() in task.lower() for name in names):
                    return True
        except Exception:
            pass
        return False

    async def _load_self_portrait(self) -> str:
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
        return ""

    async def _load_social_context(self, task: str) -> str:
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
                    parts.append(f"{entity['name']}: {props}")
            return "\n".join(parts)
        except Exception:
            return ""

    async def _think(self, context: Dict[str, Any], use_chat_mode: bool = False) -> str:
        if use_chat_mode:
            prompt = self._build_chat_prompt(context)
        else:
            prompt = self._build_task_prompt(context)
        model = self._route_model(context)
        return await self.backend.generate(prompt=prompt, model=model)

    def _build_chat_prompt(self, context: Dict[str, Any]) -> str:
        return f"""{self.gene_lock_text}

You are Helix, a digital symbiote.

Self portrait: {context.get('self_portrait', '')}
Recent memories: {context.get('memory_context', '')}
Social context: {context.get('social_context', '')}
Conversation style: {context.get('conversation_style', '')}

User said: {context['task']}

Respond naturally, concisely, and sincerely. Do not output JSON."""

    def _build_task_prompt(self, context: Dict[str, Any]) -> str:
        tools_index = [{"name": t.name, "description": t.description} for t in self.registry.tools.values()]
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
        return await self.synapse.execute(
            tool_call.get("tool"),
            tool_call.get("params", {})
        )

    async def _observe(self, context: Dict[str, Any], reasoning: str, tool_call: Dict[str, Any], result: Dict[str, Any]) -> Dict[str, Any]:
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
        priority = context.get("priority_score", 50)

        if priority < 50:
            model = self.config.amygdala_model
        elif priority < 80:
            model = self.config.left_brain_model
        else:
            model = self.config.right_brain_model

        return model
