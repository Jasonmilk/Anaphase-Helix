# -*- coding: utf-8 -*-
import sys

try:
    print("Helix OS Boot Sequence v1.0")
    print(f"Current Directory: {sys.path[0]}")
except Exception as e:
    print(f"Survival Error: {e}")