"""State-graph driven Agent Loop core."""
from typing import Dict, Callable, Literal
from ana.common import logger, get_settings
from ana.schemas.state import HelixState


class AgentLoop:
    """State-graph driven main loop for Helix agent.
    
    Uses declarative transition matrix to define state transitions,
    following the Engineering Manual's requirement of table-driven transitions.
    """
    
    def __init__(self, settings):
        self.settings = settings
        # Declarative transition matrix: current_step -> next_step -> condition
        self.transitions: Dict[
            Literal["perceive", "assess_priority", "plan", "execute", "reflect", "consolidate", "sleep"],
            Dict[str, Callable[[HelixState], bool]]
        ] = {
            "perceive": {"assess_priority": lambda s: True},
            "assess_priority": {"plan": lambda s: True},
            "plan": {"execute": lambda s: True},
            "execute": {"reflect": lambda s: True},
            "reflect": {"consolidate": lambda s: self._check_fatigue(s),
                        "plan": lambda s: not self._check_fatigue(s)},
            "consolidate": {"sleep": lambda s: self._check_apoptosis(s),
                            "plan": lambda s: not self._check_apoptosis(s)},
            "sleep": {},
        }
    
    def _check_fatigue(self, state: HelixState) -> bool:
        """Check if we've hit the fatigue line."""
        usage_ratio = state.metabolism.used_tokens / state.metabolism.budget_total
        return usage_ratio >= state.metabolism.fatigue_line
    
    def _check_apoptosis(self, state: HelixState) -> bool:
        """Check if we've hit the apoptosis line."""
        usage_ratio = state.metabolism.used_tokens / state.metabolism.budget_total
        return usage_ratio >= state.metabolism.apoptosis_line
    
    async def run(self, state: HelixState) -> None:
        """Run the agent loop until completion."""
        logger.info("agent_loop.start", epoch_id=state.epoch_id)
        
        # Placeholder: implement state transitions
        raise NotImplementedError(
            "AgentLoop.run is not implemented yet. "
            "Implement the state transition logic following the declarative transition matrix."
        )
