"""Model Router module: Model selection based on priority."""
from ana.common import get_settings
from ana.schemas.priority import PriorityAssessment


class ModelRouter:
    """Model Router responsible for selecting the appropriate model
    based on task priority and current state.
    """
    
    def __init__(self, settings):
        self.settings = settings
    
    def select_model(self, priority: PriorityAssessment, module: str) -> str:
        """Select the appropriate model for the current task.
        
        Args:
            priority: Priority assessment of the task
            module: The brain module (amygdala, left_brain, right_brain)
            
        Returns:
            Model name to use
        """
        # Route based on module and priority
        if module == "amygdala":
            return self.settings.amygdala_model
        elif module == "left_brain":
            return self.settings.left_brain_model
        elif module == "right_brain":
            return self.settings.right_brain_model
        elif module == "cerebellum":
            return self.settings.cerebellum_model
        else:
            raise ValueError(f"Unknown module: {module}")
