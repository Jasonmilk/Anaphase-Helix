class AnaphaseBaseError(Exception):
    pass

class ContractViolationError(AnaphaseBaseError):
    pass

class TuckRejectionError(AnaphaseBaseError):
    pass

class CommissuralSplitError(AnaphaseBaseError):
    pass

class MunchausenRiskError(AnaphaseBaseError):
    pass

class ToolExecutionError(AnaphaseBaseError):
    pass

class MetabolismApoptosisError(AnaphaseBaseError):
    pass
