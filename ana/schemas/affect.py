from pydantic import BaseModel, Field


class AffectVector(BaseModel):
    heliotropism: float = Field(..., ge=-1.0, le=1.0, description="Heliotropism [-1.0, 1.0]")
    pulse: float = Field(..., ge=0.0, le=1.0, description="Pulse [0.0, 1.0]")
    vigilance: float = Field(default=0.0, ge=0.0, le=1.0, description="Vigilance [0.0, 1.0]")
